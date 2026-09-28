//! Watches the user's Gmail (inbox and spam) for employers' replies to the
//! applications Job Hunter sent, attaches them to the application and tells
//! the user on every device. Read-only: nothing is ever answered or moved.

use std::sync::Arc;

use chrono::Duration;
use serde::Deserialize;
use serde_json::{json, Value};

use super::steps::{self, StepCtx};
use crate::context::AppContext;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::prompts;
use crate::util::now;

/// How far back replies are looked for, and how long after applying an
/// application stays watched.
pub const WATCH_DAYS: i64 = 60;

/// Applications worth watching: sent (or further along) recently.
pub fn tracked_applications(app: &AppContext) -> CoreResult<Vec<(Application, Job)>> {
    let since = now() - Duration::days(WATCH_DAYS);
    let jobs = app.store.list::<Job>()?;
    let mut out: Vec<(Application, Job)> = app
        .store
        .list::<Application>()?
        .into_iter()
        .filter(|a| {
            matches!(
                a.status,
                ApplicationStatus::Applied
                    | ApplicationStatus::Interview
                    | ApplicationStatus::Offer
            ) && a.applied_at.unwrap_or(a.updated_at) >= since
        })
        .filter_map(|a| {
            let job = jobs.iter().find(|j| j.id == a.job_id)?.clone();
            Some((a, job))
        })
        .collect();
    out.sort_by_key(|(a, _)| std::cmp::Reverse(a.applied_at.unwrap_or(a.updated_at)));
    Ok(out)
}

fn applications_for_prompt(tracked: &[(Application, Job)]) -> Value {
    Value::Array(
        tracked
            .iter()
            .map(|(a, j)| {
                json!({
                    "applicationId": a.id,
                    "company": j.company,
                    "title": j.title,
                    "applyEmail": j.apply_email,
                    "appliedAt": a.applied_at,
                    "source": j.source,
                    "knownReplies": a.replies.iter().map(|r| json!({
                        "from": r.from, "subject": r.subject, "receivedAt": r.received_at,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplyOut {
    application_id: String,
    #[serde(flatten)]
    reply: EmailReply,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxOut {
    #[serde(default)]
    replies: Vec<ReplyOut>,
    #[serde(default)]
    blocked: bool,
    #[serde(default)]
    blocked_reason: String,
}

/// Check Gmail now, in the background.
pub async fn start_inbox_check(app: Arc<AppContext>) -> CoreResult<AgentRun> {
    let tracked = tracked_applications(&app)?;
    if tracked.is_empty() {
        return Err(CoreError::Validation(
            "No sent applications to look for replies to yet.".into(),
        ));
    }
    let mut run = AgentRun::new(&app.user_id(), RunKind::InboxCheck, app.is_mock().await);
    run.job_ids = tracked.iter().map(|(_, j)| j.id.clone()).collect();
    let cancel = app.agent.begin(&run).await?;
    run.updated_at = now();
    app.store.put(&run)?;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = StepCtx::new(app2.clone(), &run_id, cancel);
        let result = check_inbox(&step, &tracked).await;
        let mut run: AgentRun = app2
            .store
            .get(&run_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| AgentRun::new(&app2.user_id(), RunKind::InboxCheck, false));
        run.finished_at = Some(now());
        run.stats = app2.agent.status().await.stats;
        let (state, error) = match &result {
            Ok(_) | Err(CoreError::Cancelled) => (AgentState::Completed, None),
            Err(e) => (AgentState::Failed, Some(e.user_message())),
        };
        if let Err(e) = &result {
            if !matches!(e, CoreError::Cancelled) {
                app2.notify(
                    Notification::new(
                        &app2.user_id(),
                        EventLevel::Error,
                        "INBOX_CHECK_FAILED",
                        "Couldn't check your inbox for replies",
                        e.user_message(),
                    )
                    .link("applications", None),
                );
            }
        }
        run.state = state;
        run.error = error.clone();
        app2.agent.finish(state, error, &run_id).await;
        run.updated_at = now();
        let _ = app2.store.put(&run);
    });
    app.agent.set_task(task).await;
    Ok(run)
}

/// Returns how many new replies were found.
async fn check_inbox(step: &StepCtx, tracked: &[(Application, Job)]) -> CoreResult<usize> {
    let app = &step.app;
    steps::preflight(step, true).await?;
    app.agent
        .transition(AgentState::CheckingInbox, &step.run_id)
        .await?;
    step.activity("Checking Gmail for replies").await;
    step.event(
        EventLevel::Info,
        "STEP_STARTED",
        format!(
            "Looking for replies to {} applications in Gmail (inbox and spam)",
            tracked.len()
        ),
    );
    let settings = app.settings().await;
    let applications = applications_for_prompt(tracked);
    let mut req = step
        .request(
            "check_inbox",
            prompts::inbox_check_prompt(&applications, WATCH_DAYS as u32),
        )
        .await?;
    req.chrome = true;
    // Reading only: no form input or uploads.
    req.allowed_tools = prompts::chrome_tools(false);
    req.json_schema = Some(prompts::schemas::inbox_check());
    req.max_turns = settings.claude.max_turns_browser;
    req.mock_context = json!({ "applications": applications });
    let resp = step.run_claude(req).await?;
    let out: InboxOut = steps::structured(&resp, "inbox check")?;
    if out.blocked {
        return Err(CoreError::Validation(if out.blocked_reason.is_empty() {
            "Gmail isn't available in Chrome. Sign in to Gmail and try again.".into()
        } else {
            out.blocked_reason
        }));
    }

    let mut found = 0;
    for ReplyOut {
        application_id,
        mut reply,
    } in out.replies
    {
        let Some(mut application) = app.store.get::<Application>(&application_id)? else {
            tracing::warn!(%application_id, "reply for an unknown application ignored");
            continue;
        };
        if application.replies.iter().any(|r| r.same_email(&reply)) {
            continue;
        }
        let Ok(job) = app.store.require::<Job>(&application.job_id) else {
            continue;
        };
        reply.found_at = now();
        if reply.id.is_empty() {
            reply.id = crate::util::new_id();
        }
        application.replies.push(reply.clone());
        application.updated_at = now();
        app.store.put(&application)?;
        found += 1;

        let in_spam = reply.folder.eq_ignore_ascii_case("SPAM");
        let (level, what) = match reply.kind.as_str() {
            "INTERVIEW" => (EventLevel::Success, "wants to interview you"),
            "OFFER" => (EventLevel::Success, "made you an offer"),
            "ASSESSMENT" => (EventLevel::Warn, "sent you an assessment"),
            "QUESTION" => (EventLevel::Warn, "has a question for you"),
            "REJECTION" => (EventLevel::Info, "replied: not moving forward"),
            "ACKNOWLEDGEMENT" => (EventLevel::Info, "acknowledged your application"),
            _ => (EventLevel::Info, "replied"),
        };
        let title = format!(
            "{} {what}{}",
            job.company,
            if in_spam { " (in Spam)" } else { "" }
        );
        step.event_with(
            level,
            "REPLY_FOUND",
            title.clone(),
            json!({ "applicationId": application.id, "kind": reply.kind, "folder": reply.folder }),
        );
        app.notify(
            Notification::new(
                &application.user_id,
                level,
                "APPLICATION_REPLY",
                title,
                format!("{} — {}", reply.subject, reply.summary),
            )
            .link("application", Some(&application.id)),
        );
    }
    step.event(
        EventLevel::Success,
        "STEP_DONE",
        match found {
            0 => "No new replies".to_string(),
            1 => "1 new reply".to_string(),
            n => format!("{n} new replies"),
        },
    );
    Ok(found)
}

/// Check the inbox when it's due: enabled, something to watch, the agent
/// idle, and the last check at least `every` ago.
pub async fn inbox_tick(app: &Arc<AppContext>) -> CoreResult<Option<AgentRun>> {
    let hours = app.settings().await.inbox_check_hours;
    if hours == 0 || app.agent.is_busy().await {
        return Ok(None);
    }
    if tracked_applications(app)?.is_empty() {
        return Ok(None);
    }
    let last = app
        .store
        .find::<AgentRun>(|r| r.kind == RunKind::InboxCheck)?
        .into_iter()
        .map(|r| r.started_at)
        .max();
    if last.is_some_and(|t| now() - t < Duration::hours(i64::from(hours))) {
        return Ok(None);
    }
    start_inbox_check(app.clone()).await.map(Some)
}

/// Checks when an inbox check is due, every few minutes, for the life of
/// the app. The host spawns this on its runtime.
pub async fn run_inbox_checks(app: Arc<AppContext>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
    loop {
        interval.tick().await;
        if let Err(e) = inbox_tick(&app).await {
            tracing::warn!(error = %e, "inbox check scheduling failed");
        }
    }
}
