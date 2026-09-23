//! End-to-end test of the job hunt and application workflows against the
//! deterministic mock runner: discover → dedupe → analyze → resume → review →
//! approve → apply (never submits) → human input → manual fallback.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::domain::*;
use job_hunter_core::AppContext;

async fn wait_idle(ctx: &Arc<AppContext>) -> AgentStatus {
    for _ in 0..600 {
        let s = ctx.agent.status().await;
        if !s.state.is_running() && !s.paused {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("agent did not become idle");
}

async fn fixture_context() -> (Arc<AppContext>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    // Load the fixture candidate so the pipeline has a complete profile.
    let fixture = job_hunter_core::claude::MockClaudeRunner::fixture_candidate();
    let mut profile: CandidateProfile = serde_json::from_value(fixture["profile"].clone()).unwrap();
    profile.user_id = LOCAL_USER_ID.into();
    ctx.save_profile(profile).unwrap();
    for e in fixture["experiences"].as_array().unwrap() {
        let mut e: Experience = serde_json::from_value(e.clone()).unwrap();
        e.user_id = LOCAL_USER_ID.into();
        ctx.save_experience(e).unwrap();
    }
    for p in fixture["projects"].as_array().unwrap() {
        let mut p: Project = serde_json::from_value(p.clone()).unwrap();
        p.user_id = LOCAL_USER_ID.into();
        ctx.save_project(p).unwrap();
    }
    let mut settings = ctx.settings().await;
    settings.mock_mode = true;
    settings.resume.generate_pdf = true;
    settings.resume.generate_docx = true;
    ctx.save_settings(settings).await.unwrap();
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_job_hunt_produces_reviewable_applications_and_never_submits() {
    let (ctx, _dir) = fixture_context().await;
    let mut events = ctx.agent.bus.subscribe_events();

    let run = orchestrator::start_job_hunt(ctx.clone(), JobHuntOptions::default())
        .await
        .unwrap();
    assert!(
        orchestrator::start_job_hunt(ctx.clone(), JobHuntOptions::default())
            .await
            .is_err(),
        "second run must be refused while busy"
    );
    let status = wait_idle(&ctx).await;
    assert_eq!(
        status.state,
        AgentState::WaitingForApproval,
        "error: {:?}",
        status.error
    );

    // Fixtures: 3 LinkedIn + 2 Naukri jobs, one duplicated across platforms.
    let jobs = ctx.list_jobs().unwrap();
    assert_eq!(jobs.len(), 4, "duplicate job must be merged");
    let merged = jobs
        .iter()
        .find(|j| j.job.company.starts_with("ABC Technologies"))
        .unwrap();
    assert_eq!(merged.job.sources.len(), 2);
    assert!(jobs.iter().all(|j| j.analysis.is_some()));
    let ml = jobs
        .iter()
        .find(|j| j.job.company == "Quantessence AI")
        .unwrap();
    assert!(
        !ml.analysis.as_ref().unwrap().relevant,
        "staff ML role must be irrelevant"
    );
    assert_eq!(ml.job.status, JobStatus::NotRelevant);

    let apps = ctx.list_applications().unwrap();
    assert!(!apps.is_empty());
    assert!(apps
        .iter()
        .all(|a| a.application.status == ApplicationStatus::ReadyForReview));
    let first = &apps[0];
    let detail = ctx.application_detail(&first.application.id).unwrap();
    let resume = detail.resume.expect("resume generated");
    assert!(std::path::Path::new(resume.pdf_path.as_ref().unwrap()).is_file());
    assert!(std::path::Path::new(resume.docx_path.as_ref().unwrap()).is_file());
    assert_eq!(resume.validation.as_ref().unwrap().status, AtsStatus::Pass);
    assert!(detail.cover_letter.is_some());

    // Applying before approval is refused.
    assert!(
        orchestrator::apply_application(ctx.clone(), &first.application.id, false, None)
            .await
            .is_err()
    );

    // Approve → mock apply reports MANUAL_ACTION_REQUIRED (mock never submits).
    orchestrator::approve_application(ctx.clone(), &first.application.id, true)
        .await
        .unwrap();
    let status = wait_idle(&ctx).await;
    assert_eq!(status.state, AgentState::ManualActionRequired);
    let app: Application = ctx.store.require(&first.application.id).unwrap();
    assert_eq!(app.status, ApplicationStatus::ManualActionRequired);
    assert!(app.failure_reason.is_some());
    assert!(app.approved_at.is_some());

    // Manual fallback.
    let app =
        orchestrator::mark_manual_complete(ctx.clone(), &first.application.id, "done by hand")
            .await
            .unwrap();
    assert_eq!(app.status, ApplicationStatus::Applied);
    assert!(app.manual_completed);
    let job: Job = ctx.store.require(&app.job_id).unwrap();
    assert_eq!(job.status, JobStatus::Applied);

    // Events were streamed and persisted.
    let mut seen_kinds = std::collections::HashSet::new();
    while let Ok(ev) = events.try_recv() {
        seen_kinds.insert(ev.kind);
    }
    for k in [
        "STATE_CHANGED",
        "STEP_DONE",
        "JOB_DISCOVERED",
        "JOB_ANALYZED",
        "RESUME_GENERATED",
        "APPLICATION_READY",
        "RUN_FINISHED",
    ] {
        assert!(seen_kinds.contains(k), "missing event kind {k}");
    }
    assert!(!ctx.run_events(&run.id, 500).unwrap().is_empty());

    // Running again discovers nothing new (seen URLs are skipped) and completes.
    orchestrator::start_job_hunt(ctx.clone(), JobHuntOptions::default())
        .await
        .unwrap();
    let status = wait_idle(&ctx).await;
    assert_eq!(status.state, AgentState::Completed);
    assert_eq!(ctx.list_jobs().unwrap().len(), 4);

    let dashboard = ctx.dashboard().await.unwrap();
    assert_eq!(dashboard.total.applied, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn human_input_flow_stores_answers_and_resumes() {
    let (ctx, _dir) = fixture_context().await;
    orchestrator::start_job_hunt(ctx.clone(), JobHuntOptions::default())
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let app_id = ctx.list_applications().unwrap()[0].application.id.clone();
    orchestrator::approve_application(ctx.clone(), &app_id, false)
        .await
        .unwrap();
    orchestrator::apply_application(
        ctx.clone(),
        &app_id,
        false,
        Some("HUMAN_INPUT_REQUIRED".into()),
    )
    .await
    .unwrap();
    let status = wait_idle(&ctx).await;
    assert_eq!(status.state, AgentState::WaitingForUser);
    let app: Application = ctx.store.require(&app_id).unwrap();
    assert_eq!(app.status, ApplicationStatus::WaitingForUser);
    assert_eq!(app.pending_questions.len(), 2);

    let answers: Vec<ApplicationAnswer> = app
        .pending_questions
        .iter()
        .map(|q| ApplicationAnswer {
            question: q.question.clone(),
            answer: "Yes".into(),
            source: AnswerSource::User,
        })
        .collect();
    orchestrator::answer_questions(ctx.clone(), &app_id, answers, true)
        .await
        .unwrap();
    let status = wait_idle(&ctx).await;
    // Mock resumes and (by default) still refuses to submit → manual action.
    assert_eq!(status.state, AgentState::ManualActionRequired);
    let app: Application = ctx.store.require(&app_id).unwrap();
    assert!(app.pending_questions.is_empty());
    assert!(app.answers.iter().any(|a| a.source == AnswerSource::User));
    assert_eq!(
        ctx.list_answers().unwrap().len(),
        2,
        "answers saved for reuse"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_cancels_a_running_hunt() {
    let (ctx, _dir) = fixture_context().await;
    ctx.set_runner(Arc::new(job_hunter_core::claude::MockClaudeRunner {
        delay: Duration::from_millis(400),
    }))
    .await;
    orchestrator::start_job_hunt(ctx.clone(), JobHuntOptions::default())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(ctx.agent.request_stop().await);
    let status = wait_idle(&ctx).await;
    assert_eq!(status.state, AgentState::Completed);
    assert!(ctx.list_applications().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_jobs_are_never_applied_to() {
    let (ctx, _dir) = fixture_context().await;
    orchestrator::start_job_hunt(ctx.clone(), JobHuntOptions::default())
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let app_id = ctx.list_applications().unwrap()[0].application.id.clone();
    orchestrator::reject_application(ctx.clone(), &app_id, "not interested")
        .await
        .unwrap();
    let err = orchestrator::apply_application(ctx.clone(), &app_id, false, None)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        job_hunter_core::CoreError::InvalidTransition(_)
    ));
}
