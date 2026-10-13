//! Recording an interview call by hand: only on sent applications, moves
//! them to the interview stage, keeps every round, and shows up on the
//! dashboard while it's upcoming.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::domain::*;
use job_hunter_core::error::CoreError;
use job_hunter_core::AppContext;

async fn wait_idle(ctx: &Arc<AppContext>) {
    for _ in 0..600 {
        let s = ctx.agent.status().await;
        if !s.state.is_running() && !s.paused {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("agent did not become idle");
}

/// A mock hunt, then one application approved and sent.
async fn sent_application() -> (Arc<AppContext>, tempfile::TempDir, Application) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let fixture = job_hunter_core::claude::MockClaudeRunner::fixture_candidate();
    let mut profile: CandidateProfile = serde_json::from_value(fixture["profile"].clone()).unwrap();
    profile.user_id = LOCAL_USER_ID.into();
    ctx.save_profile(profile).unwrap();
    for e in fixture["experiences"].as_array().unwrap() {
        let mut e: Experience = serde_json::from_value(e.clone()).unwrap();
        e.user_id = LOCAL_USER_ID.into();
        ctx.save_experience(e).unwrap();
    }
    orchestrator::start_job_hunt(
        ctx.clone(),
        JobHuntOptions {
            sources: vec!["LinkedIn".into()],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let item = ctx
        .list_applications()
        .unwrap()
        .into_iter()
        .find(|a| a.application.status == ApplicationStatus::ReadyForReview)
        .expect("an application ready for review");
    let id = item.application.id.clone();
    orchestrator::approve_application(ctx.clone(), &id, false)
        .await
        .unwrap();
    orchestrator::apply_application(ctx.clone(), &id, false, Some("SUBMITTED".into()))
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let application: Application = ctx.store.require(&id).unwrap();
    assert_eq!(application.status, ApplicationStatus::Applied);
    (ctx, dir, application)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_interview_call_moves_a_sent_application_to_the_interview_stage() {
    let (ctx, _dir, application) = sent_application().await;
    let when = Utc::now() + chrono::Duration::days(2);
    let updated = orchestrator::record_interview(
        ctx.clone(),
        &application.id,
        InterviewInput {
            scheduled_at: Some(when),
            mode: "video".into(),
            round: "HR screening".into(),
            contact: "Priya (HR)".into(),
            notes: "They'll send a Meet link".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.status, ApplicationStatus::Interview);
    assert_eq!(updated.interviews.len(), 1);
    assert_eq!(updated.interviews[0].mode, "VIDEO");
    assert_eq!(updated.interviews[0].round, "HR screening");
    let job: Job = ctx.store.require(&updated.job_id).unwrap();
    assert_eq!(job.status, JobStatus::Interview);

    // A second round keeps the first.
    let again = orchestrator::record_interview(
        ctx.clone(),
        &application.id,
        InterviewInput {
            scheduled_at: Some(when + chrono::Duration::days(5)),
            mode: "ON_SITE".into(),
            round: "Technical".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(again.status, ApplicationStatus::Interview);
    assert_eq!(again.interviews.len(), 2);

    let dashboard = ctx.dashboard().await.unwrap();
    assert_eq!(dashboard.upcoming_interviews.len(), 2);
    assert_eq!(
        dashboard.upcoming_interviews[0].interview.round,
        "HR screening"
    );
    assert_eq!(
        dashboard.upcoming_interviews[0].application_id,
        application.id
    );
    assert_eq!(dashboard.total.interviews, 1);

    let bad = orchestrator::record_interview(
        ctx.clone(),
        &application.id,
        InterviewInput {
            mode: "telepathy".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(bad, CoreError::Validation(_)), "{bad:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interviews_need_a_sent_application() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let mut application = Application::new(LOCAL_USER_ID, "job-1", "https://x", "LinkedIn");
    application
        .transition(ApplicationStatus::Analyzed, "test")
        .unwrap();
    application
        .transition(ApplicationStatus::ReadyForReview, "test")
        .unwrap();
    ctx.store.put(&application).unwrap();
    let err =
        orchestrator::record_interview(ctx.clone(), &application.id, InterviewInput::default())
            .await
            .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)), "{err:?}");
    let unchanged: Application = ctx.store.require(&application.id).unwrap();
    assert_eq!(unchanged.status, ApplicationStatus::ReadyForReview);
    assert!(unchanged.interviews.is_empty());
}
