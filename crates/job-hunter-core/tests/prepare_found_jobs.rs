//! A search-only hunt finds and scores jobs but prepares nothing; "Prepare
//! applications" then prepares the best of them without searching again.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::domain::*;
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

async fn wait_for_notification(ctx: &Arc<AppContext>, title_starts: &str) -> Notification {
    for _ in 0..200 {
        if let Some(n) = ctx
            .store
            .list::<Notification>()
            .unwrap()
            .into_iter()
            .find(|n| n.title.starts_with(title_starts))
        {
            return n;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no notification starting with {title_starts:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_only_then_prepare_the_best_matches() {
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

    let search = orchestrator::start_job_hunt(
        ctx.clone(),
        JobHuntOptions {
            discover_only: true,
            sources: vec!["LinkedIn".into()],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let search: AgentRun = ctx.store.require(&search.id).unwrap();
    assert!(search.discover_only);
    assert!(
        search.stats.relevant > 0,
        "the mock search finds relevant jobs"
    );
    assert!(ctx.store.list::<Application>().unwrap().is_empty());
    let told = wait_for_notification(&ctx, "Found ").await;
    assert!(told.body.contains("Prepare applications"), "{}", told.body);

    let prepare = orchestrator::start_job_hunt(
        ctx.clone(),
        JobHuntOptions {
            prepare_only: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let prepare: AgentRun = ctx.store.require(&prepare.id).unwrap();
    assert!(prepare.prepare_only);
    assert_eq!(prepare.stats.jobs_discovered, 0, "no new search");
    assert_eq!(prepare.state, AgentState::WaitingForApproval);
    let ready: Vec<_> = ctx
        .list_applications()
        .unwrap()
        .into_iter()
        .filter(|a| a.application.status == ApplicationStatus::ReadyForReview)
        .collect();
    assert!(!ready.is_empty());
    assert_eq!(ready.len() as u32, prepare.stats.awaiting_approval);
    let settings = ctx.settings().await;
    assert!(ready.len() <= settings.max_applications_per_run as usize);

    // Running it again doesn't prepare the same jobs twice.
    let again = orchestrator::start_job_hunt(
        ctx.clone(),
        JobHuntOptions {
            prepare_only: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let again: AgentRun = ctx.store.require(&again.id).unwrap();
    assert!(again
        .job_ids
        .iter()
        .all(|id| ready.iter().all(|r| &r.application.job_id != id)));
}
