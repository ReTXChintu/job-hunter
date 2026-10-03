//! "Apply from a link": the user pastes a posting's URL from any job site.
//! The agent opens it, reads the full posting, analyses it, prepares the
//! tailored resume and cover letter, and -- when the user asked to apply
//! right away -- approves and applies through the same approval-gated path
//! as every other application (`orchestrator::approve_application`).

use std::sync::Arc;

use serde_json::json;

use super::orchestrator;
use super::steps::{self, StepCtx};
use crate::context::AppContext;
use crate::dedup;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::util::now;

/// A pasted link, cleaned up: must be http(s) with a host.
pub fn normalize_link(raw: &str) -> CoreResult<String> {
    let trimmed = raw.trim();
    let parsed = url::Url::parse(trimmed).map_err(|_| {
        CoreError::Validation("Paste the full link to the job, starting with https://".into())
    })?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(CoreError::Validation(
            "That isn't a web link to a job posting.".into(),
        ));
    }
    Ok(parsed.to_string())
}

/// The job site a link belongs to, for the job's source label.
pub fn source_for(url: &str) -> String {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .unwrap_or_default();
    let known = [
        ("linkedin.com", "LinkedIn"),
        ("naukri.com", "Naukri"),
        ("indeed.", "Indeed"),
        ("wellfound.com", "Wellfound"),
        ("cutshort.io", "Cutshort"),
        ("instahyre.com", "Instahyre"),
        ("hirist.", "Hirist"),
        ("foundit.", "Foundit"),
        ("hiring.cafe", "Hiring Cafe"),
        ("welcometothejungle.com", "Welcome to the Jungle"),
        ("himalayas.app", "Himalayas"),
        ("workatastartup.com", "Y Combinator"),
        ("ycombinator.com", "Y Combinator"),
    ];
    known
        .iter()
        .find(|(needle, _)| host.contains(needle))
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| {
            if host.is_empty() {
                "Link".into()
            } else {
                host.trim_start_matches("www.").to_string()
            }
        })
}

/// Read and prepare the job at `url` in the background; with `apply_now`,
/// approve and apply as soon as the resume is ready.
pub async fn start_job_link(
    app: Arc<AppContext>,
    url: &str,
    apply_now: bool,
) -> CoreResult<AgentRun> {
    let url = normalize_link(url)?;
    let mut run = AgentRun::new(&app.user_id(), RunKind::JobLink, app.is_mock().await);
    run.sources = vec![source_for(&url)];
    let cancel = app.agent.begin(&run).await?;
    run.updated_at = now();
    app.store.put(&run)?;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = StepCtx::new(app2.clone(), &run_id, cancel);
        let result = prepare(&step, &url).await;
        let mut run: AgentRun = app2
            .store
            .get(&run_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| AgentRun::new(&app2.user_id(), RunKind::JobLink, false));
        run.finished_at = Some(now());
        run.stats = app2.agent.status().await.stats;
        let (state, error) = match &result {
            Ok(_) | Err(CoreError::Cancelled) => (AgentState::Completed, None),
            Err(e) => (AgentState::Failed, Some(e.user_message())),
        };
        if let Ok(application) = &result {
            run.application_id = Some(application.id.clone());
        }
        run.state = state;
        run.error = error.clone();
        app2.agent.finish(state, error, &run_id).await;
        run.updated_at = now();
        let _ = app2.store.put(&run);

        match result {
            // The user asked to apply when they started it: that's the
            // approval, recorded like any other, through the same gate.
            Ok(application)
                if apply_now && application.status == ApplicationStatus::ReadyForReview =>
            {
                if let Err(e) =
                    orchestrator::approve_application(app2.clone(), &application.id, true).await
                {
                    notify_failure(
                        &app2,
                        &format!("Couldn't start applying: {}", e.user_message()),
                    );
                }
            }
            Ok(application) => {
                app2.notify(
                    Notification::new(
                        &application.user_id,
                        EventLevel::Success,
                        "JOB_LINK_READY",
                        "Application ready for your review",
                        "The job from your link is analysed and the resume is ready. Approve it to apply.",
                    )
                    .link("application", Some(&application.id)),
                );
            }
            Err(CoreError::Cancelled) => {}
            Err(e) => notify_failure(&app2, &e.user_message()),
        }
    });
    app.agent.set_task(task).await;
    Ok(run)
}

fn notify_failure(app: &AppContext, message: &str) {
    app.notify(
        Notification::new(
            &app.user_id(),
            EventLevel::Error,
            "JOB_LINK_FAILED",
            "Couldn't apply from your link",
            message,
        )
        .link("jobs", None),
    );
}

async fn prepare(step: &StepCtx, url: &str) -> CoreResult<Application> {
    let app = &step.app;
    let pre = steps::preflight(step, true).await?;

    // The same posting pasted again: carry on with what's known.
    let existing = app.store.list::<Job>()?.into_iter().find(|j| j.url == url);
    let mut job = match existing {
        Some(job) => job,
        None => {
            let mut job = Job::new(&pre.truth.profile.user_id, &source_for(url), url, "", "");
            job.run_id = Some(step.run_id.clone());
            job
        }
    };
    if let Some(application) = app
        .store
        .find::<Application>(|a| a.job_id == job.id)?
        .into_iter()
        .next()
    {
        if !matches!(
            application.status,
            ApplicationStatus::Discovered | ApplicationStatus::Analyzed
        ) {
            return Ok(application);
        }
    }

    app.agent
        .transition(AgentState::Extracting, &step.run_id)
        .await?;
    step.activity("Opening the job link").await;
    if !steps::extract_details(step, &mut job).await? {
        return Err(CoreError::Validation(
            "Couldn't read a job posting at that link (it may need you to sign in, or the posting is closed).".into(),
        ));
    }
    if job.title.trim().is_empty() || job.company.trim().is_empty() {
        return Err(CoreError::Validation(
            "That link doesn't show a job title and company.".into(),
        ));
    }
    dedup::prepare(&mut job);
    job.touch();
    app.store.put(&job)?;
    step.event_with(
        EventLevel::Info,
        "JOB_DISCOVERED",
        format!("{} at {} (from your link)", job.title, job.company),
        json!({ "jobId": job.id }),
    );

    app.agent
        .transition(AgentState::Analyzing, &step.run_id)
        .await?;
    let mut analyses = steps::analyze_jobs(step, &pre.truth, std::slice::from_ref(&job)).await?;
    let analysis = analyses.pop();
    if let Some(a) = &analysis {
        steps::record_analysis(step, &mut job, a).await?;
    }
    app.agent
        .transition(AgentState::PreparingApplications, &step.run_id)
        .await?;
    let generated = steps::generate_resume(step, &pre.truth, &job, analysis.as_ref()).await?;
    steps::prepare_application(step, &pre.truth, &mut job, analysis.as_ref(), &generated).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_checked_and_named_after_their_site() {
        assert!(normalize_link("  https://www.linkedin.com/jobs/view/123/ ").is_ok());
        assert!(normalize_link("linkedin.com/jobs/view/123").is_err());
        assert!(normalize_link("ftp://example.com/job").is_err());
        assert_eq!(
            source_for("https://www.naukri.com/job-listings-x-123"),
            "Naukri"
        );
        assert_eq!(source_for("https://in.indeed.com/viewjob?jk=1"), "Indeed");
        assert_eq!(
            source_for("https://www.workatastartup.com/jobs/1"),
            "Y Combinator"
        );
        assert_eq!(
            source_for("https://careers.acme.com/jobs/9"),
            "careers.acme.com"
        );
    }
}
