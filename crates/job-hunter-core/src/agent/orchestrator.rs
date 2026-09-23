//! Runs the job-hunt pipeline and individual application tasks as background
//! tokio tasks, driving the state machine and persisting after every step.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::steps::{self, StepCtx};
use crate::context::AppContext;
use crate::dedup;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::util::now;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHuntOptions {
    /// Stop after discovery + analysis (no resume generation).
    #[serde(default)]
    pub discover_only: bool,
    /// Restrict to these sources (defaults to enabled sources in settings).
    #[serde(default)]
    pub sources: Vec<String>,
}

async fn persist_run(app: &AppContext, run: &mut AgentRun) {
    run.updated_at = now();
    if let Err(e) = app.store.put(run) {
        tracing::error!(error = %e, "failed to persist agent run");
    }
}

fn step_ctx(
    app: &Arc<AppContext>,
    run_id: &str,
    cancel: tokio_util::sync::CancellationToken,
) -> StepCtx {
    StepCtx {
        app: app.clone(),
        run_id: run_id.into(),
        cancel,
    }
}

/// Start a job hunt. Returns the run immediately; the pipeline continues in
/// the background and streams events.
pub async fn start_job_hunt(app: Arc<AppContext>, options: JobHuntOptions) -> CoreResult<AgentRun> {
    let settings = app.settings().await;
    let mut run = AgentRun::new(&app.user_id(), RunKind::JobHunt, app.is_mock().await);
    run.sources = if options.sources.is_empty() {
        settings.enabled_sources()
    } else {
        options.sources.clone()
    };
    let cancel = app.agent.begin(&run).await?;
    persist_run(&app, &mut run).await;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = step_ctx(&app2, &run_id, cancel);
        let outcome = run_job_hunt(&step, options).await;
        let mut run: AgentRun = app2
            .store
            .get(&run_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| AgentRun::new(&app2.user_id(), RunKind::JobHunt, false));
        let status = app2.agent.status().await;
        run.stats = status.stats.clone();
        run.finished_at = Some(now());
        match outcome {
            Ok(final_state) => {
                run.state = final_state;
                app2.agent.finish(final_state, None, &run_id).await;
            }
            Err(CoreError::Cancelled) => {
                run.state = AgentState::Completed;
                run.current_activity = Some("Stopped by user".into());
                app2.agent.bus.emit(AgentEvent::new(
                    &run_id,
                    EventLevel::Warn,
                    "MESSAGE",
                    "Job hunt stopped by user",
                ));
                app2.agent
                    .finish(AgentState::Completed, None, &run_id)
                    .await;
            }
            Err(e) => {
                let msg = e.user_message();
                tracing::error!(error = %e, "job hunt failed");
                run.state = AgentState::Failed;
                run.error = Some(msg.clone());
                app2.agent
                    .finish(AgentState::Failed, Some(msg), &run_id)
                    .await;
            }
        }
        persist_run(&app2, &mut run).await;
    });
    app.agent.set_task(task).await;
    Ok(run)
}

async fn run_job_hunt(step: &StepCtx, options: JobHuntOptions) -> CoreResult<AgentState> {
    let app = &step.app;
    let run_id = &step.run_id;
    let settings = app.settings().await;
    step.event(EventLevel::Info, "STEP_STARTED", "Initializing job hunt");

    let pre = steps::preflight(step, true).await?;
    let sources = if options.sources.is_empty() {
        pre.sources.clone()
    } else {
        options.sources.clone()
    };
    let truth = pre.truth;

    // ---- Discovery -------------------------------------------------------------
    app.agent
        .transition(AgentState::Discovering, run_id)
        .await?;
    let mut discovered: Vec<Job> = Vec::new();
    let total_sources = sources.len().max(1);
    for (i, source) in sources.iter().enumerate() {
        app.agent.checkpoint(&step.cancel).await?;
        step.activity(&format!("Searching {source}")).await;
        step.progress(((i * 100) / total_sources) as u8).await;
        step.event_with(
            EventLevel::Info,
            "STEP_STARTED",
            format!("Searching {source}"),
            json!({ "source": source }),
        );
        match steps::discover_source(step, &truth, source).await {
            Ok(jobs) => {
                step.event_with(
                    if jobs.is_empty() {
                        EventLevel::Warn
                    } else {
                        EventLevel::Success
                    },
                    "STEP_DONE",
                    format!("Found {} jobs on {source}", jobs.len()),
                    json!({ "source": source, "count": jobs.len() }),
                );
                let n = jobs.len() as u32;
                app.agent.update(|s| s.stats.jobs_discovered += n).await;
                discovered.extend(jobs);
            }
            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
            Err(e) => {
                app.agent.update(|s| s.stats.errors += 1).await;
                step.event(
                    EventLevel::Error,
                    "STEP_FAILED",
                    format!("{source}: {}", e.user_message()),
                );
                if matches!(
                    e,
                    CoreError::ClaudeNotAuthenticated { .. }
                        | CoreError::ClaudeCliUnavailable { .. }
                        | CoreError::ClaudeInChromeUnavailable { .. }
                ) {
                    return Err(e);
                }
            }
        }
    }
    step.progress(100).await;

    // ---- Extraction ------------------------------------------------------------
    let incomplete: Vec<usize> = discovered
        .iter()
        .enumerate()
        .filter(|(_, j)| !j.details_complete)
        .map(|(i, _)| i)
        .collect();
    if !incomplete.is_empty() {
        app.agent.transition(AgentState::Extracting, run_id).await?;
        step.event(
            EventLevel::Info,
            "STEP_STARTED",
            format!("Extracting full details for {} jobs", incomplete.len()),
        );
        let n = incomplete.len();
        for (k, idx) in incomplete.into_iter().enumerate() {
            app.agent.checkpoint(&step.cancel).await?;
            let job = &mut discovered[idx];
            step.activity(&format!("Reading {} at {}", job.title, job.company))
                .await;
            step.progress(((k * 100) / n) as u8).await;
            match steps::extract_details(step, job).await {
                Ok(_) => {}
                Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
                Err(e) => {
                    app.agent.update(|s| s.stats.errors += 1).await;
                    step.event(
                        EventLevel::Warn,
                        "STEP_FAILED",
                        format!(
                            "Could not extract {} at {}: {}",
                            job.title,
                            job.company,
                            e.user_message()
                        ),
                    );
                }
            }
        }
        step.event(EventLevel::Success, "STEP_DONE", "Extraction finished");
    }

    // ---- Deduplication ---------------------------------------------------------
    app.agent
        .transition(AgentState::Deduplicating, run_id)
        .await?;
    step.activity("Removing duplicates").await;
    let existing = app.store.list::<Job>()?;
    let outcome = dedup::deduplicate(&existing, discovered);
    for merged in &outcome.merged {
        app.store.put(merged)?;
    }
    let mut new_jobs = outcome.new_jobs;
    for job in &new_jobs {
        app.store.put(job)?;
        step.event_with(
            EventLevel::Info,
            "JOB_DISCOVERED",
            format!("{} at {} ({})", job.title, job.company, job.source),
            json!({ "jobId": job.id }),
        );
    }
    let new_count = new_jobs.len() as u32;
    let dup_count = outcome.duplicates_removed as u32;
    app.agent
        .update(|s| {
            s.stats.jobs_new += new_count;
            s.stats.duplicates_removed += dup_count;
        })
        .await;
    step.event(
        EventLevel::Success,
        "STEP_DONE",
        format!("{} new jobs, {} duplicates merged", new_count, dup_count),
    );
    {
        let mut run: AgentRun = app.store.require(run_id)?;
        run.job_ids = new_jobs.iter().map(|j| j.id.clone()).collect();
        persist_run(app, &mut run).await;
    }
    if new_jobs.is_empty() {
        step.event(EventLevel::Info, "MESSAGE", "No new jobs to analyze");
        return Ok(AgentState::Completed);
    }

    // ---- Analysis --------------------------------------------------------------
    app.agent.transition(AgentState::Analyzing, run_id).await?;
    let total = new_jobs.len();
    let mut analyses: Vec<JobAnalysis> = Vec::new();
    for (bi, chunk) in new_jobs.chunks_mut(steps::ANALYSIS_BATCH).enumerate() {
        app.agent.checkpoint(&step.cancel).await?;
        step.activity(&format!(
            "Analyzing jobs {}–{} of {total}",
            bi * steps::ANALYSIS_BATCH + 1,
            (bi * steps::ANALYSIS_BATCH + chunk.len()).min(total)
        ))
        .await;
        step.progress(((bi * steps::ANALYSIS_BATCH * 100) / total) as u8)
            .await;
        let snapshot: Vec<Job> = chunk.to_vec();
        match steps::analyze_jobs(step, &truth, &snapshot).await {
            Ok(batch) => {
                for a in batch {
                    if let Some(job) = chunk.iter_mut().find(|j| j.id == a.job_id) {
                        steps::record_analysis(step, job, &a).await?;
                        let relevant = a.relevant;
                        app.agent
                            .update(|s| {
                                s.stats.jobs_analyzed += 1;
                                if relevant {
                                    s.stats.relevant += 1;
                                }
                            })
                            .await;
                        analyses.push(a);
                    }
                }
            }
            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
            Err(e) => {
                app.agent.update(|s| s.stats.errors += 1).await;
                step.event(
                    EventLevel::Error,
                    "STEP_FAILED",
                    format!("Analysis failed for a batch: {}", e.user_message()),
                );
            }
        }
    }
    step.progress(100).await;
    step.event(
        EventLevel::Success,
        "STEP_DONE",
        format!(
            "Analyzed {} jobs, {} relevant",
            analyses.len(),
            analyses.iter().filter(|a| a.relevant).count()
        ),
    );
    if options.discover_only {
        return Ok(AgentState::Completed);
    }

    // ---- Prepare applications --------------------------------------------------
    app.agent
        .transition(AgentState::PreparingApplications, run_id)
        .await?;
    let mut candidates: Vec<(Job, JobAnalysis)> = new_jobs
        .iter()
        .filter_map(|j| {
            analyses
                .iter()
                .find(|a| a.job_id == j.id)
                .map(|a| (j.clone(), a.clone()))
        })
        .filter(|(_, a)| a.relevant && a.match_score >= settings.minimum_match_score)
        .collect();
    candidates.sort_by_key(|(_, a)| std::cmp::Reverse(a.match_score));
    candidates.truncate(settings.max_applications_per_run as usize);
    let n = candidates.len();
    if n == 0 {
        step.event(
            EventLevel::Info,
            "MESSAGE",
            "No jobs met the minimum match score; nothing to prepare",
        );
        return Ok(AgentState::Completed);
    }
    step.event(
        EventLevel::Info,
        "STEP_STARTED",
        format!("Preparing {n} applications"),
    );
    let mut prepared = 0u32;
    for (i, (mut job, analysis)) in candidates.into_iter().enumerate() {
        app.agent.checkpoint(&step.cancel).await?;
        step.progress(((i * 100) / n) as u8).await;
        // Skip jobs that already have a live application.
        if let Some(existing) = app
            .store
            .find::<Application>(|a| a.job_id == job.id)?
            .into_iter()
            .next()
        {
            if !matches!(
                existing.status,
                ApplicationStatus::Discovered | ApplicationStatus::Analyzed
            ) {
                continue;
            }
        }
        match steps::generate_resume(step, &truth, &job, Some(&analysis)).await {
            Ok(generated) => match steps::prepare_application(
                step,
                &truth,
                &mut job,
                Some(&analysis),
                &generated,
            )
            .await
            {
                Ok(_) => {
                    prepared += 1;
                    app.agent.update(|s| s.stats.awaiting_approval += 1).await;
                }
                Err(e) => {
                    app.agent.update(|s| s.stats.errors += 1).await;
                    step.event(
                        EventLevel::Error,
                        "STEP_FAILED",
                        format!(
                            "Could not prepare {} at {}: {}",
                            job.title,
                            job.company,
                            e.user_message()
                        ),
                    );
                }
            },
            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
            Err(e) => {
                app.agent.update(|s| s.stats.errors += 1).await;
                step.event(
                    EventLevel::Error,
                    "STEP_FAILED",
                    format!(
                        "Resume generation failed for {} at {}: {}",
                        job.title,
                        job.company,
                        e.user_message()
                    ),
                );
            }
        }
    }
    step.progress(100).await;
    step.event(
        EventLevel::Success,
        "STEP_DONE",
        format!("{prepared} applications waiting for your approval"),
    );
    if prepared > 0 {
        app.agent
            .transition(AgentState::WaitingForApproval, run_id)
            .await?;
        Ok(AgentState::WaitingForApproval)
    } else {
        Ok(AgentState::Completed)
    }
}

// ---------------------------------------------------------------------------
// Single-job commands
// ---------------------------------------------------------------------------

/// Analyze (or re-analyze) one job in the background.
pub async fn analyze_job(app: Arc<AppContext>, job_id: String) -> CoreResult<AgentRun> {
    let job: Job = app.store.require(&job_id)?;
    let mut run = AgentRun::new(&app.user_id(), RunKind::Analysis, app.is_mock().await);
    run.job_ids = vec![job.id.clone()];
    let cancel = app.agent.begin(&run).await?;
    persist_run(&app, &mut run).await;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = step_ctx(&app2, &run_id, cancel);
        let result: CoreResult<()> = async {
            let pre = steps::preflight(&step, false).await?;
            app2.agent
                .transition(AgentState::Analyzing, &run_id)
                .await?;
            let mut job: Job = app2.store.require(&job_id)?;
            step.activity(&format!("Analyzing {} at {}", job.title, job.company))
                .await;
            let analyses =
                steps::analyze_jobs(&step, &pre.truth, std::slice::from_ref(&job)).await?;
            for a in analyses {
                steps::record_analysis(&step, &mut job, &a).await?;
            }
            Ok(())
        }
        .await;
        finish_simple(&app2, &run_id, result).await;
    });
    app.agent.set_task(task).await;
    Ok(run)
}

/// Generate (or regenerate) the resume and cover letter for one job and put
/// its application in review.
pub async fn generate_resume(app: Arc<AppContext>, job_id: String) -> CoreResult<AgentRun> {
    let job: Job = app.store.require(&job_id)?;
    let mut run = AgentRun::new(
        &app.user_id(),
        RunKind::ResumeGeneration,
        app.is_mock().await,
    );
    run.job_ids = vec![job.id.clone()];
    let cancel = app.agent.begin(&run).await?;
    persist_run(&app, &mut run).await;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = step_ctx(&app2, &run_id, cancel);
        let result: CoreResult<()> = async {
            let pre = steps::preflight(&step, false).await?;
            app2.agent
                .transition(AgentState::PreparingApplications, &run_id)
                .await?;
            let mut job: Job = app2.store.require(&job_id)?;
            let analysis = match &job.analysis_id {
                Some(id) => app2.store.get::<JobAnalysis>(id)?,
                None => None,
            };
            let analysis = match analysis {
                Some(a) => Some(a),
                None => {
                    step.activity(&format!("Analyzing {} at {}", job.title, job.company))
                        .await;
                    let mut list =
                        steps::analyze_jobs(&step, &pre.truth, std::slice::from_ref(&job)).await?;
                    if let Some(a) = list.pop() {
                        steps::record_analysis(&step, &mut job, &a).await?;
                        Some(a)
                    } else {
                        None
                    }
                }
            };
            let generated =
                steps::generate_resume(&step, &pre.truth, &job, analysis.as_ref()).await?;
            steps::prepare_application(&step, &pre.truth, &mut job, analysis.as_ref(), &generated)
                .await?;
            Ok(())
        }
        .await;
        finish_simple(&app2, &run_id, result).await;
    });
    app.agent.set_task(task).await;
    Ok(run)
}

async fn finish_simple(app: &Arc<AppContext>, run_id: &str, result: CoreResult<()>) {
    let mut run: AgentRun = app
        .store
        .get(run_id)
        .ok()
        .flatten()
        .unwrap_or_else(|| AgentRun::new(&app.user_id(), RunKind::Analysis, false));
    run.finished_at = Some(now());
    run.stats = app.agent.status().await.stats;
    match result {
        Ok(()) => {
            run.state = AgentState::Completed;
            app.agent.finish(AgentState::Completed, None, run_id).await;
        }
        Err(CoreError::Cancelled) => {
            run.state = AgentState::Completed;
            app.agent.finish(AgentState::Completed, None, run_id).await;
        }
        Err(e) => {
            let msg = e.user_message();
            run.state = AgentState::Failed;
            run.error = Some(msg.clone());
            app.agent
                .finish(AgentState::Failed, Some(msg), run_id)
                .await;
        }
    }
    persist_run(app, &mut run).await;
}

// ---------------------------------------------------------------------------
// Applications
// ---------------------------------------------------------------------------

/// Explicit user approval. Records the approval and starts the browser
/// application task.
pub async fn approve_application(
    app: Arc<AppContext>,
    application_id: &str,
    apply_now: bool,
) -> CoreResult<Application> {
    let mut application: Application = app.store.require(application_id)?;
    if application.status == ApplicationStatus::Rejected {
        application.transition(ApplicationStatus::ReadyForReview, "reopened")?;
    }
    application.transition(ApplicationStatus::Approved, "approved by user")?;
    app.store.put(&application)?;
    if let Ok(mut job) = app.store.require::<Job>(&application.job_id) {
        job.status = JobStatus::Approved;
        job.touch();
        app.store.put(&job)?;
    }
    app.emit_event(
        AgentEvent::new(
            application.run_id.as_deref().unwrap_or("manual"),
            EventLevel::Success,
            "APPLICATION_UPDATED",
            "Application approved",
        )
        .with_data(json!({ "applicationId": application.id, "status": "APPROVED" })),
    );
    if apply_now {
        apply_application(app, application_id, false, None).await?;
    }
    Ok(application)
}

pub async fn reject_application(
    app: Arc<AppContext>,
    application_id: &str,
    reason: &str,
) -> CoreResult<Application> {
    let mut application: Application = app.store.require(application_id)?;
    application.transition(
        ApplicationStatus::Rejected,
        if reason.is_empty() {
            "rejected by user"
        } else {
            reason
        },
    )?;
    if !reason.trim().is_empty() {
        if !application.notes.is_empty() {
            application.notes.push('\n');
        }
        application
            .notes
            .push_str(&format!("Rejected: {}", reason.trim()));
    }
    app.store.put(&application)?;
    if let Ok(mut job) = app.store.require::<Job>(&application.job_id) {
        job.status = JobStatus::Rejected;
        job.touch();
        app.store.put(&job)?;
    }
    Ok(application)
}

/// Run the browser application for an approved application in the background.
pub async fn apply_application(
    app: Arc<AppContext>,
    application_id: &str,
    resuming: bool,
    simulate: Option<String>,
) -> CoreResult<AgentRun> {
    let application: Application = app.store.require(application_id)?;
    if !application.is_approved() {
        return Err(CoreError::InvalidTransition(
            "This application has not been approved. Approve it first.".into(),
        ));
    }
    if matches!(application.status, ApplicationStatus::Applied) {
        return Err(CoreError::InvalidTransition(
            "This application was already submitted.".into(),
        ));
    }
    let mut run = AgentRun::new(&app.user_id(), RunKind::Application, app.is_mock().await);
    run.application_id = Some(application.id.clone());
    run.job_ids = vec![application.job_id.clone()];
    let cancel = app.agent.begin(&run).await?;
    persist_run(&app, &mut run).await;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let app_id = application_id.to_string();
    let task = tokio::spawn(async move {
        let step = step_ctx(&app2, &run_id, cancel);
        let result: CoreResult<AgentState> = async {
            let pre = steps::preflight(&step, true).await?;
            app2.agent.transition(AgentState::Applying, &run_id).await?;
            let mut application: Application = app2.store.require(&app_id)?;
            let mut job: Job = app2.store.require(&application.job_id)?;
            if application.status != ApplicationStatus::Applying {
                application.transition(ApplicationStatus::Applying, if resuming { "continuing after user input" } else { "browser application started" })?;
                app2.store.put(&application)?;
            }
            job.status = JobStatus::Applying;
            job.touch();
            app2.store.put(&job)?;
            step.activity(&format!("Applying to {} at {}", job.title, job.company)).await;
            step.event_with(EventLevel::Info, "STEP_STARTED", format!("Opening the application for {} at {}", job.title, job.company), json!({ "applicationId": application.id }));
            let result = match steps::apply(&step, &pre.truth, &mut application, &mut job, resuming, simulate.as_deref()).await {
                Ok(r) => r,
                Err(CoreError::Cancelled) => {
                    application.failure_reason = Some("Stopped by user before completion.".into());
                    application.transition(ApplicationStatus::ManualActionRequired, "stopped by user")?;
                    job.status = JobStatus::ManualActionRequired;
                    job.touch();
                    app2.store.put(&application)?;
                    app2.store.put(&job)?;
                    return Err(CoreError::Cancelled);
                }
                Err(e) => {
                    application.failure_reason = Some(e.user_message());
                    application.transition(ApplicationStatus::ManualActionRequired, "automatic application failed")?;
                    job.status = JobStatus::ManualActionRequired;
                    job.touch();
                    app2.store.put(&application)?;
                    app2.store.put(&job)?;
                    step.event_with(EventLevel::Error, "APPLICATION_UPDATED", format!("{}: automatic application failed - {}", job.company, e.user_message()), json!({ "applicationId": application.id, "status": "MANUAL_ACTION_REQUIRED" }));
                    return Err(e);
                }
            };
            steps::record_apply_result(&step, &mut application, &mut job, &result)?;
            let final_state = match application.status {
                ApplicationStatus::Applied => {
                    app2.agent.update(|s| s.stats.applied += 1).await;
                    AgentState::Completed
                }
                ApplicationStatus::WaitingForUser => AgentState::WaitingForUser,
                _ => {
                    app2.agent.update(|s| s.stats.manual_action += 1).await;
                    AgentState::ManualActionRequired
                }
            };
            Ok(final_state)
        }
        .await;
        let mut run: AgentRun = app2
            .store
            .get(&run_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| AgentRun::new(&app2.user_id(), RunKind::Application, false));
        run.finished_at = Some(now());
        run.stats = app2.agent.status().await.stats;
        match result {
            Ok(state) => {
                run.state = state;
                app2.agent.finish(state, None, &run_id).await;
            }
            Err(CoreError::Cancelled) => {
                run.state = AgentState::ManualActionRequired;
                app2.agent
                    .finish(AgentState::ManualActionRequired, None, &run_id)
                    .await;
            }
            Err(e) => {
                let msg = e.user_message();
                run.state = AgentState::ManualActionRequired;
                run.error = Some(msg.clone());
                app2.agent
                    .finish(AgentState::ManualActionRequired, Some(msg), &run_id)
                    .await;
            }
        }
        persist_run(&app2, &mut run).await;
    });
    app.agent.set_task(task).await;
    Ok(run)
}

/// The user answered the pending questions: store them (for reuse) and resume
/// the application.
pub async fn answer_questions(
    app: Arc<AppContext>,
    application_id: &str,
    answers: Vec<ApplicationAnswer>,
    save_for_reuse: bool,
) -> CoreResult<Application> {
    let mut application: Application = app.store.require(application_id)?;
    if application.status != ApplicationStatus::WaitingForUser {
        return Err(CoreError::InvalidTransition(
            "This application is not waiting for your input.".into(),
        ));
    }
    for mut a in answers {
        a.source = AnswerSource::User;
        if a.answer.trim().is_empty() {
            continue;
        }
        application
            .pending_questions
            .retain(|q| q.question != a.question);
        application.answers.retain(|x| x.question != a.question);
        if save_for_reuse {
            let existing = app.store.list::<AnswerRecord>()?;
            match crate::domain::find_answer(&existing, &a.question).cloned() {
                Some(mut rec) => {
                    rec.answer = a.answer.clone();
                    rec.updated_at = now();
                    app.store.put(&rec)?;
                }
                None => {
                    let rec = AnswerRecord::new(
                        &application.user_id,
                        &a.question,
                        &a.answer,
                        "application",
                    );
                    app.store.put(&rec)?;
                }
            }
        }
        application.answers.push(a);
    }
    application.updated_at = now();
    app.store.put(&application)?;
    let id = application.id.clone();
    apply_application(app, &id, true, None).await?;
    Ok(application)
}

pub async fn mark_manual_complete(
    app: Arc<AppContext>,
    application_id: &str,
    note: &str,
) -> CoreResult<Application> {
    let mut application: Application = app.store.require(application_id)?;
    application.transition(ApplicationStatus::Applied, "marked as applied by user")?;
    application.manual_completed = true;
    application.failure_reason = None;
    if !note.trim().is_empty() {
        if !application.notes.is_empty() {
            application.notes.push('\n');
        }
        application.notes.push_str(note.trim());
    }
    app.store.put(&application)?;
    if let Ok(mut job) = app.store.require::<Job>(&application.job_id) {
        job.status = JobStatus::Applied;
        job.touch();
        app.store.put(&job)?;
    }
    Ok(application)
}

/// Post-application tracking (interview, offer, withdrawn, rejected).
pub async fn set_application_status(
    app: Arc<AppContext>,
    application_id: &str,
    status: ApplicationStatus,
    note: &str,
) -> CoreResult<Application> {
    let mut application: Application = app.store.require(application_id)?;
    if matches!(
        status,
        ApplicationStatus::Applying | ApplicationStatus::Approved | ApplicationStatus::Applied
    ) {
        return Err(CoreError::Validation(
            "Use the approve / apply / mark-as-applied actions for that status.".into(),
        ));
    }
    application.transition(
        status,
        if note.is_empty() {
            "updated by user"
        } else {
            note
        },
    )?;
    if !note.trim().is_empty() {
        if !application.notes.is_empty() {
            application.notes.push('\n');
        }
        application.notes.push_str(note.trim());
    }
    app.store.put(&application)?;
    if let Ok(mut job) = app.store.require::<Job>(&application.job_id) {
        job.status = match status {
            ApplicationStatus::Interview => JobStatus::Interview,
            ApplicationStatus::Offer => JobStatus::Offer,
            ApplicationStatus::Withdrawn => JobStatus::Withdrawn,
            ApplicationStatus::Rejected => JobStatus::Rejected,
            ApplicationStatus::ReadyForReview => JobStatus::ReadyForReview,
            ApplicationStatus::ManualActionRequired => JobStatus::ManualActionRequired,
            _ => job.status,
        };
        job.touch();
        app.store.put(&job)?;
    }
    Ok(application)
}
