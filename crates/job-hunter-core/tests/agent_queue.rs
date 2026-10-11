//! Starting work while the agent is busy queues it; queued work starts by
//! itself, in order, as soon as the agent is free. Approving an application
//! while the agent is busy queues the apply instead of failing.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::{orchestrator, queue};
use job_hunter_core::claude::MockClaudeRunner;
use job_hunter_core::domain::*;
use job_hunter_core::error::CoreError;
use job_hunter_core::AppContext;

async fn context() -> (Arc<AppContext>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    // Slow enough for the agent to still be busy when the next request comes.
    ctx.set_runner(Arc::new(MockClaudeRunner {
        delay: Duration::from_millis(100),
    }))
    .await;
    let fixture = MockClaudeRunner::fixture_candidate();
    let mut profile: CandidateProfile = serde_json::from_value(fixture["profile"].clone()).unwrap();
    profile.user_id = LOCAL_USER_ID.into();
    ctx.save_profile(profile).unwrap();
    for e in fixture["experiences"].as_array().unwrap() {
        let mut e: Experience = serde_json::from_value(e.clone()).unwrap();
        e.user_id = LOCAL_USER_ID.into();
        ctx.save_experience(e).unwrap();
    }
    (ctx, dir)
}

fn hunt(sources: &[&str]) -> AgentTask {
    AgentTask::JobHunt {
        discover_only: false,
        prepare_only: false,
        sources: sources.iter().map(|s| s.to_string()).collect(),
    }
}

fn search(sources: &[&str]) -> AgentTask {
    AgentTask::JobHunt {
        discover_only: true,
        prepare_only: false,
        sources: sources.iter().map(|s| s.to_string()).collect(),
    }
}

async fn wait_until(
    ctx: &Arc<AppContext>,
    what: &str,
    pred: impl Fn(&AgentStatus, &[QueuedTask]) -> bool,
) {
    for _ in 0..900 {
        let s = ctx.agent.status().await;
        let q = queue::list(ctx).unwrap();
        if pred(&s, &q) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for {what}");
}

fn idle(s: &AgentStatus) -> bool {
    !s.state.is_running() && !s.paused
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_started_while_busy_waits_its_turn_then_runs_by_itself() {
    let (ctx, _dir) = context().await;
    let first = queue::submit(ctx.clone(), hunt(&["LinkedIn"]))
        .await
        .unwrap();
    assert!(first.run.is_some() && first.queued.is_none());
    assert!(ctx.agent.is_busy().await);

    let second = queue::submit(ctx.clone(), search(&["Naukri"]))
        .await
        .unwrap();
    assert!(second.run.is_none());
    assert_eq!(second.position, 1);
    // The same request twice takes one place.
    let again = queue::submit(ctx.clone(), search(&["Naukri"]))
        .await
        .unwrap();
    assert_eq!(again.queued.unwrap().id, second.queued.as_ref().unwrap().id);
    let third = queue::submit(
        ctx.clone(),
        AgentTask::JobHunt {
            discover_only: false,
            prepare_only: true,
            sources: vec![],
        },
    )
    .await
    .unwrap();
    assert_eq!(third.position, 2);
    assert_eq!(ctx.agent.status().await.queued, 2);
    let titles: Vec<String> = queue::list(&ctx)
        .unwrap()
        .into_iter()
        .map(|q| q.title)
        .collect();
    assert_eq!(titles, ["Search only: Naukri", "Prepare applications"]);

    wait_until(&ctx, "the queue to drain", |s, q| q.is_empty() && idle(s)).await;
    let runs: Vec<AgentRun> = ctx.store.list().unwrap();
    assert_eq!(
        runs.iter().filter(|r| r.kind == RunKind::JobHunt).count(),
        3,
        "{runs:?}"
    );
    assert!(runs
        .iter()
        .any(|r| r.discover_only && r.sources == ["Naukri"] && r.finished_at.is_some()));
    assert!(runs
        .iter()
        .any(|r| r.prepare_only && r.finished_at.is_some()));
    assert_eq!(ctx.agent.status().await.queued, 0);
    let events: Vec<AgentEvent> = ctx.store.list().unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message.starts_with("Started from the queue"))
            .count(),
        2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approving_while_busy_queues_the_application() {
    let (ctx, _dir) = context().await;
    queue::submit(ctx.clone(), hunt(&["LinkedIn"]))
        .await
        .unwrap();
    wait_until(&ctx, "the hunt to finish", |s, _| idle(s)).await;
    let ready = ctx
        .list_applications()
        .unwrap()
        .into_iter()
        .find(|a| a.application.status == ApplicationStatus::ReadyForReview)
        .expect("an application to review")
        .application;

    // Keep the agent busy, then approve.
    queue::submit(ctx.clone(), search(&["Naukri"]))
        .await
        .unwrap();
    assert!(ctx.agent.is_busy().await);
    let approved = orchestrator::approve_application(ctx.clone(), &ready.id, true)
        .await
        .unwrap();
    assert_eq!(approved.status, ApplicationStatus::Approved);
    let q = queue::list(&ctx).unwrap();
    assert_eq!(q.len(), 1);
    assert!(q[0].title.starts_with("Apply: "), "{}", q[0].title);

    wait_until(&ctx, "the application to run", |s, q| {
        q.is_empty() && idle(s)
    })
    .await;
    let runs: Vec<AgentRun> = ctx.store.list().unwrap();
    assert!(runs.iter().any(|r| r.kind == RunKind::Application
        && r.application_id.as_deref() == Some(&ready.id)
        && r.finished_at.is_some()));
    let application: Application = ctx.store.require(&ready.id).unwrap();
    assert_ne!(
        application.status,
        ApplicationStatus::Approved,
        "the apply ran"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bad_requests_are_refused_and_queued_items_can_be_removed() {
    let (ctx, _dir) = context().await;
    let err = queue::submit(
        ctx.clone(),
        AgentTask::Application {
            application_id: "nope".into(),
            resume: false,
            simulate: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }), "{err:?}");

    queue::submit(ctx.clone(), hunt(&["LinkedIn"]))
        .await
        .unwrap();
    let queued = queue::submit(ctx.clone(), search(&["Naukri"]))
        .await
        .unwrap()
        .queued
        .unwrap();
    assert_eq!(ctx.agent.status().await.queued, 1);
    assert!(queue::remove(&ctx, &queued.id).await.unwrap());
    assert_eq!(ctx.agent.status().await.queued, 0);
    assert!(queue::list(&ctx).unwrap().is_empty());
    ctx.agent.request_stop().await;
    wait_until(&ctx, "the stop", |s, _| idle(s)).await;
}
