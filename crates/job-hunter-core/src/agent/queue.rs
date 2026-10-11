//! The agent's work queue. Every start request goes through [`submit`]: it
//! runs at once when the agent is free, otherwise it waits in the
//! `agent_queue` collection and [`run_worker`] starts it, in order, as soon
//! as the agent stops being busy, including when a run stops for the
//! user's input. A task that can no longer run (its application was
//! rejected meanwhile, ...) is dropped with a notification.

use std::sync::Arc;

use tokio::sync::broadcast::error::RecvError;

use super::{inbox, job_link, orchestrator, profile_sync, shared_job};
use crate::context::AppContext;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};

/// Start `task` now, or queue it if the agent is busy.
pub async fn submit(app: Arc<AppContext>, task: AgentTask) -> CoreResult<Submission> {
    validate(&app, &task)?;
    match start(app.clone(), task.clone()).await {
        Ok(run) => Ok(Submission {
            run: Some(run),
            queued: None,
            position: 0,
        }),
        Err(CoreError::AgentBusy(_)) => enqueue(&app, task).await,
        Err(e) => Err(e),
    }
}

/// Waiting tasks, first to run first.
pub fn list(app: &AppContext) -> CoreResult<Vec<QueuedTask>> {
    let mut queue = app.store.list::<QueuedTask>()?;
    queue.sort_by_key(|q| q.requested_at);
    Ok(queue)
}

pub async fn remove(app: &AppContext, id: &str) -> CoreResult<bool> {
    let removed = app.store.delete::<QueuedTask>(id)?;
    if removed {
        publish_count(app).await?;
    }
    Ok(removed)
}

pub async fn clear(app: &AppContext) -> CoreResult<usize> {
    let queue = list(app)?;
    for item in &queue {
        app.store.delete::<QueuedTask>(&item.id)?;
    }
    publish_count(app).await?;
    Ok(queue.len())
}

/// Start queued tasks whenever the agent becomes free. Runs for the life of
/// the app; also picks up whatever was left queued when it last closed.
pub async fn run_worker(app: Arc<AppContext>) {
    let mut rx = app.agent.bus.subscribe_status();
    if let Err(e) = publish_count(&app).await {
        tracing::warn!(error = %e, "could not read the agent queue");
    }
    drain(&app).await;
    loop {
        match rx.recv().await {
            Ok(status) => {
                if !status.state.is_running() && !status.paused {
                    drain(&app).await;
                }
            }
            Err(RecvError::Lagged(_)) => drain(&app).await,
            Err(RecvError::Closed) => return,
        }
    }
}

/// Start the next task that can run, if the agent is free.
pub async fn start_next(app: &Arc<AppContext>) -> CoreResult<Option<AgentRun>> {
    loop {
        if app.agent.is_busy().await {
            return Ok(None);
        }
        let Some(item) = list(app)?.into_iter().next() else {
            return Ok(None);
        };
        match start(app.clone(), item.task.clone()).await {
            Ok(run) => {
                app.store.delete::<QueuedTask>(&item.id)?;
                publish_count(app).await?;
                app.emit_event(AgentEvent::new(
                    &run.id,
                    EventLevel::Info,
                    "MESSAGE",
                    format!("Started from the queue: {}", item.title),
                ));
                return Ok(Some(run));
            }
            // Something else got there first; the task keeps its place.
            Err(CoreError::AgentBusy(_)) => return Ok(None),
            Err(e) => {
                app.store.delete::<QueuedTask>(&item.id)?;
                publish_count(app).await?;
                app.notify(
                    Notification::new(
                        &app.user_id(),
                        EventLevel::Warn,
                        "QUEUED_TASK_SKIPPED",
                        format!("Skipped: {}", item.title),
                        e.user_message(),
                    )
                    .link("agent", None),
                );
            }
        }
    }
}

async fn drain(app: &Arc<AppContext>) {
    if let Err(e) = start_next(app).await {
        tracing::warn!(error = %e, "could not start the next queued task");
    }
}

async fn enqueue(app: &Arc<AppContext>, task: AgentTask) -> CoreResult<Submission> {
    let mut queue = list(app)?;
    if let Some(pos) = queue.iter().position(|q| q.task == task) {
        return Ok(Submission {
            run: None,
            queued: Some(queue[pos].clone()),
            position: pos + 1,
        });
    }
    let item = QueuedTask::new(&app.user_id(), task.clone(), title(app, &task).await);
    app.store.put(&item)?;
    queue.push(item.clone());
    publish_count(app).await?;
    let run_id = app
        .agent
        .status()
        .await
        .run_id
        .unwrap_or_else(|| "queue".into());
    app.emit_event(AgentEvent::new(
        &run_id,
        EventLevel::Info,
        "TASK_QUEUED",
        format!(
            "Queued: {} (position {}; starts when the agent is free)",
            item.title,
            queue.len()
        ),
    ));
    Ok(Submission {
        run: None,
        queued: Some(item),
        position: queue.len(),
    })
}

/// Keep the live status' queue length current, so every screen can show it.
async fn publish_count(app: &AppContext) -> CoreResult<()> {
    let n = app.store.count(QueuedTask::COLLECTION) as u32;
    app.agent.set_queued(n).await;
    Ok(())
}

/// Run one task now. Fails with `AgentBusy` when the agent is taken.
/// Boxed: a job-link run approves and applies through [`submit`], which
/// leads back here, and a recursive future needs a fixed size.
fn start(
    app: Arc<AppContext>,
    task: AgentTask,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = CoreResult<AgentRun>> + Send>> {
    Box::pin(start_inner(app, task))
}

async fn start_inner(app: Arc<AppContext>, task: AgentTask) -> CoreResult<AgentRun> {
    match task {
        AgentTask::JobHunt {
            discover_only,
            prepare_only,
            sources,
        } => {
            orchestrator::start_job_hunt(
                app,
                orchestrator::JobHuntOptions {
                    discover_only,
                    prepare_only,
                    sources,
                },
            )
            .await
        }
        AgentTask::Application {
            application_id,
            resume,
            simulate,
        } => orchestrator::apply_application(app, &application_id, resume, simulate).await,
        AgentTask::AnalyzeJob { job_id } => orchestrator::analyze_job(app, job_id).await,
        AgentTask::GenerateResume { job_id } => orchestrator::generate_resume(app, job_id).await,
        AgentTask::ProfileSync {
            platform,
            resume,
            answers,
        } => profile_sync::start_profile_sync(app, &platform, answers, resume).await,
        AgentTask::JobLink { url, apply_now } => {
            job_link::start_job_link(app, &url, apply_now).await
        }
        AgentTask::SharedJob {
            text,
            files,
            draft_email,
        } => {
            shared_job::start_shared_job(
                app,
                shared_job::SharedJobInput {
                    text,
                    files: files.into_iter().map(Into::into).collect(),
                    draft_email,
                },
            )
            .await
        }
        AgentTask::InboxCheck => inbox::start_inbox_check(app).await,
    }
}

/// Refuse what could never run, before it waits in the queue. The same
/// checks the start functions make.
fn validate(app: &AppContext, task: &AgentTask) -> CoreResult<()> {
    match task {
        AgentTask::JobHunt { .. } => Ok(()),
        AgentTask::Application { application_id, .. } => {
            let application: Application = app.store.require(application_id)?;
            if !application.is_approved() {
                return Err(CoreError::InvalidTransition(
                    "This application has not been approved. Approve it first.".into(),
                ));
            }
            if application.status == ApplicationStatus::Applied {
                return Err(CoreError::InvalidTransition(
                    "This application was already submitted.".into(),
                ));
            }
            Ok(())
        }
        AgentTask::AnalyzeJob { job_id } | AgentTask::GenerateResume { job_id } => {
            app.store.require::<Job>(job_id).map(|_| ())
        }
        AgentTask::ProfileSync { platform, .. } => {
            canonical_platform(platform).ok_or_else(|| {
                CoreError::Validation(format!("Job Hunter can't update profiles on {platform}."))
            })?;
            if profile_sync::publishing_plan(app)?.confirmed_at.is_none() {
                return Err(CoreError::Validation(
                    "Choose which projects to show on your profiles first.".into(),
                ));
            }
            Ok(())
        }
        AgentTask::JobLink { url, .. } => job_link::normalize_link(url).map(|_| ()),
        AgentTask::SharedJob { text, files, .. } => {
            if text.trim().len() < 30 && files.is_empty() {
                return Err(CoreError::Validation(
                    "Paste the job description or add a screenshot or PDF of it.".into(),
                ));
            }
            Ok(())
        }
        AgentTask::InboxCheck => {
            if inbox::tracked_applications(app)?.is_empty() {
                return Err(CoreError::Validation(
                    "No sent applications to look for replies to yet.".into(),
                ));
            }
            Ok(())
        }
    }
}

async fn title(app: &AppContext, task: &AgentTask) -> String {
    let job_label = |job_id: &str| -> String {
        app.store
            .get::<Job>(job_id)
            .ok()
            .flatten()
            .map(|j| format!("{} at {}", j.title, j.company))
            .unwrap_or_else(|| "a job".into())
    };
    match task {
        AgentTask::JobHunt {
            prepare_only: true, ..
        } => "Prepare applications".into(),
        AgentTask::JobHunt {
            discover_only,
            sources,
            ..
        } => {
            let what = if *discover_only {
                "Search only"
            } else {
                "Job hunt"
            };
            let sources = if sources.is_empty() {
                app.settings().await.enabled_sources()
            } else {
                sources.clone()
            };
            if sources.is_empty() {
                what.to_string()
            } else {
                format!("{what}: {}", sources.join(", "))
            }
        }
        AgentTask::Application { application_id, .. } => {
            let job = app
                .store
                .get::<Application>(application_id)
                .ok()
                .flatten()
                .map(|a| job_label(&a.job_id))
                .unwrap_or_else(|| "a job".into());
            format!("Apply: {job}")
        }
        AgentTask::AnalyzeJob { job_id } => format!("Analyze: {}", job_label(job_id)),
        AgentTask::GenerateResume { job_id } => format!("Resume: {}", job_label(job_id)),
        AgentTask::ProfileSync {
            platform, resume, ..
        } => {
            let name = canonical_platform(platform).unwrap_or(platform);
            if *resume {
                format!("Continue updating the {name} profile")
            } else {
                format!("Update the {name} profile")
            }
        }
        AgentTask::JobLink { url, .. } => format!("Apply from link: {}", job_link::source_for(url)),
        AgentTask::SharedJob { text, .. } => {
            let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
            if first.trim().is_empty() {
                "A job I found".into()
            } else {
                format!("A job I found: {}", crate::util::truncate(first.trim(), 60))
            }
        }
        AgentTask::InboxCheck => "Check Gmail for replies".into(),
    }
}
