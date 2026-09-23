//! Opt-in smoke tests against the real Claude CLI. They are `#[ignore]` so
//! `cargo test` never spends the user's Claude quota; run explicitly with
//!
//!   cargo test -p job-hunter-core --test real_claude_smoke -- --ignored --nocapture
//!
//! `JOB_HUNTER_REAL_CHROME=1` additionally runs a read-only LinkedIn discovery
//! (max 3 jobs) through Claude in Chrome. Nothing is ever applied to.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::orchestrator;
use job_hunter_core::domain::*;
use job_hunter_core::logging::LogBuffer;
use job_hunter_core::paths::AppPaths;
use job_hunter_core::AppContext;

async fn real_context() -> (Arc<AppContext>, tempfile::TempDir) {
    let dir = match std::env::var("JOB_HUNTER_SMOKE_DIR") {
        Ok(d) => tempfile::Builder::new()
            .prefix("smoke-")
            .tempdir_in(d)
            .unwrap(),
        Err(_) => tempfile::tempdir().unwrap(),
    };
    std::env::set_var("MOCK_MODE", "false");
    let paths = AppPaths::at(dir.path().join("data")).unwrap();
    eprintln!("smoke data dir: {}", paths.root.display());
    let ctx = AppContext::init(paths, LogBuffer::new(200, None))
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
    for p in fixture["projects"].as_array().unwrap() {
        let mut p: Project = serde_json::from_value(p.clone()).unwrap();
        p.user_id = LOCAL_USER_ID.into();
        ctx.save_project(p).unwrap();
    }
    let mut settings = ctx.settings().await;
    settings.mock_mode = false;
    settings.max_jobs_per_source = 3;
    settings.claude.model = Some("sonnet".into());
    settings.claude.max_budget_usd_per_call = 2.0;
    let source = std::env::var("JOB_HUNTER_SMOKE_SOURCE").unwrap_or_else(|_| "LinkedIn".into());
    settings.job_sources = vec![job_hunter_core::settings::JobSourceConfig {
        platform: source,
        enabled: true,
    }];
    ctx.save_settings(settings).await.unwrap();
    if std::env::var("JOB_HUNTER_SMOKE_DIR").is_ok() {
        // Keep the directory for inspection; `TempDir` would delete it on drop.
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        return (ctx, tempfile::Builder::new().prefix("keep-").tempdir_in(path).unwrap());
    }
    (ctx, dir)
}

async fn wait_idle(ctx: &Arc<AppContext>, max: Duration) -> AgentStatus {
    let start = std::time::Instant::now();
    loop {
        let s = ctx.agent.status().await;
        if !s.state.is_running() {
            return s;
        }
        assert!(start.elapsed() < max, "agent did not finish in time");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn real_claude_analyzes_a_fixture_job_without_tools() {
    let (ctx, _dir) = real_context().await;
    let status = ctx.claude_status().await;
    assert!(
        status.installed && status.authenticated,
        "claude must be installed and signed in: {status:?}"
    );

    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../agent/fixtures/discovered-jobs.json")).unwrap();
    let d: DiscoveredJob = serde_json::from_value(fixtures["LinkedIn"][0].clone()).unwrap();
    let mut job = Job::new(LOCAL_USER_ID, "LinkedIn", &d.url, &d.company, &d.title);
    job.description = d.description;
    job.requirements = d.requirements;
    job.skills = d.skills;
    job.location = d.location;
    ctx.store.put(&job).unwrap();

    let mut events = ctx.agent.bus.subscribe_events();
    orchestrator::analyze_job(ctx.clone(), job.id.clone())
        .await
        .unwrap();
    let status = wait_idle(&ctx, Duration::from_secs(300)).await;
    assert_eq!(
        status.state,
        AgentState::Completed,
        "error: {:?}",
        status.error
    );
    let detail = ctx.job_detail(&job.id).unwrap();
    let analysis = detail.analysis.expect("analysis stored");
    println!(
        "REAL ANALYSIS: score={} relevant={} matched={:?} missing={:?}\n{}",
        analysis.match_score,
        analysis.relevant,
        analysis.matched_skills,
        analysis.missing_skills,
        analysis.summary
    );
    assert!(
        analysis.relevant,
        "fixture full-stack job should be relevant for the fixture candidate"
    );
    assert!(analysis.match_score >= 60);
    assert!(status.stats.claude_cost_usd > 0.0);
    let mut n = 0;
    while events.try_recv().is_ok() {
        n += 1;
    }
    assert!(n > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn real_claude_generates_and_validates_a_resume() {
    let (ctx, _dir) = real_context().await;
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../agent/fixtures/discovered-jobs.json")).unwrap();
    let d: DiscoveredJob = serde_json::from_value(fixtures["LinkedIn"][0].clone()).unwrap();
    let mut job = Job::new(LOCAL_USER_ID, "LinkedIn", &d.url, &d.company, &d.title);
    job.description = d.description;
    job.requirements = d.requirements;
    job.skills = d.skills;
    ctx.store.put(&job).unwrap();
    orchestrator::generate_resume(ctx.clone(), job.id.clone())
        .await
        .unwrap();
    let status = wait_idle(&ctx, Duration::from_secs(900)).await;
    assert_eq!(
        status.state,
        AgentState::Completed,
        "error: {:?}",
        status.error
    );
    let detail = ctx.job_detail(&job.id).unwrap();
    let resume = detail.resume.expect("resume");
    let v = resume.validation.clone().expect("validation");
    println!(
        "REAL RESUME: {:?} coverage={} unsupported={:?} pdf={:?}",
        v.status, v.keyword_coverage, v.unsupported_claims, resume.pdf_path
    );
    assert!(std::path::Path::new(resume.pdf_path.as_ref().unwrap()).is_file());
    assert_eq!(
        detail.application.unwrap().status,
        ApplicationStatus::ReadyForReview
    );
    let content = resume.content.unwrap();
    // Truthfulness: every listed skill must exist in the fixture candidate data.
    let truth = ctx.candidate_truth().await.unwrap();
    let mut known: Vec<String> = truth.profile.skills.all();
    for e in &truth.experiences {
        known.extend(e.technologies.clone());
    }
    for p in &truth.projects {
        known.extend(p.technologies.clone());
    }
    let known: Vec<String> = known.iter().map(|s| s.to_lowercase()).collect();
    for section in &content.skills {
        for item in &section.items {
            assert!(
                known
                    .iter()
                    .any(|k| k == &item.to_lowercase() || item.to_lowercase().contains(k)),
                "resume lists a skill not in the candidate data: {item}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn real_chrome_discovery_is_read_only() {
    if std::env::var("JOB_HUNTER_REAL_CHROME").is_err() {
        eprintln!(
            "skipped: set JOB_HUNTER_REAL_CHROME=1 to run discovery through Claude in Chrome"
        );
        return;
    }
    let (ctx, _dir) = real_context().await;
    let mut events = ctx.agent.bus.subscribe_events();
    orchestrator::start_job_hunt(
        ctx.clone(),
        orchestrator::JobHuntOptions {
            discover_only: true,
            sources: vec![],
        },
    )
    .await
    .unwrap();
    let status = wait_idle(&ctx, Duration::from_secs(1200)).await;
    while let Ok(ev) = events.try_recv() {
        println!("EVENT [{:?}] {}: {}", ev.level, ev.kind, ev.message);
    }
    println!(
        "REAL DISCOVERY: state={:?} stats={:?} error={:?}",
        status.state, status.stats, status.error
    );
    let jobs = ctx.list_jobs().unwrap();
    for j in &jobs {
        println!(
            "  - {} @ {} [{}] {} → {}",
            j.job.title,
            j.job.company,
            j.job.location,
            j.job.url,
            j.analysis
                .as_ref()
                .map(|a| format!("{}%", a.match_score))
                .unwrap_or_default()
        );
    }
    assert_eq!(
        status.state,
        AgentState::Completed,
        "error: {:?}",
        status.error
    );
    assert!(
        ctx.list_applications().unwrap().is_empty(),
        "discovery must never create applications"
    );
}
