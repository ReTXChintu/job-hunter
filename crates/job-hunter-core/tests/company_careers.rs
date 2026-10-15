//! The "Company careers" source searches each switched-on company's own
//! careers site, one run each, and the jobs found go through the same
//! pipeline (analysis, applications for review) as any other source.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::domain::*;
use job_hunter_core::settings::{TargetCompany, COMPANY_CAREERS};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn company_careers_searches_each_enabled_company() {
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

    let mut settings = ctx.settings().await;
    // The defaults include the big names the user asked for.
    for name in ["Google", "Microsoft", "Apple", "Accenture", "Deloitte"] {
        assert!(
            settings
                .target_companies
                .iter()
                .any(|c| c.name == name && c.enabled),
            "{name} is on by default"
        );
    }
    assert!(settings
        .job_sources
        .iter()
        .any(|s| s.platform == COMPANY_CAREERS && s.enabled));
    settings.target_companies = vec![
        TargetCompany {
            name: "Google".into(),
            careers_url: "https://careers.google.test".into(),
            enabled: true,
        },
        TargetCompany {
            name: "Deloitte".into(),
            careers_url: "https://careers.deloitte.test/".into(),
            enabled: true,
        },
        TargetCompany {
            name: "Wipro".into(),
            careers_url: "https://careers.wipro.test".into(),
            enabled: false,
        },
    ];
    ctx.save_settings(settings).await.unwrap();

    let run = orchestrator::start_job_hunt(
        ctx.clone(),
        JobHuntOptions {
            sources: vec![COMPANY_CAREERS.into()],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;

    let jobs: Vec<Job> = ctx.store.list().unwrap();
    let mut companies: Vec<&str> = jobs.iter().map(|j| j.company.as_str()).collect();
    companies.sort();
    assert_eq!(
        companies,
        ["Deloitte", "Google"],
        "one job per enabled company, none for Wipro"
    );
    assert!(jobs.iter().all(|j| j.source == COMPANY_CAREERS));
    assert!(jobs
        .iter()
        .any(|j| j.url == "https://careers.deloitte.test/jobs/mock-deloitte-1"));

    assert_eq!(
        ctx.agent.status().await.run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(ctx.agent.status().await.stats.jobs_discovered, 2);
    // Found jobs go through analysis and into Applications for review.
    let apps = ctx.list_applications().unwrap();
    assert!(!apps.is_empty());
    assert!(apps
        .iter()
        .all(|a| a.application.status == ApplicationStatus::ReadyForReview));
}
