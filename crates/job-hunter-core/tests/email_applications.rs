//! Hiring posts that ask for the resume by email: discovered with the
//! address, and applied to (after approval) by email from Gmail.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::domain::*;
use job_hunter_core::prompts::{self, ApplyParams};
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

async fn fixture_context() -> (Arc<AppContext>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let fixture = job_hunter_core::claude::MockClaudeRunner::fixture_candidate();
    let mut profile: CandidateProfile = serde_json::from_value(fixture["profile"].clone()).unwrap();
    profile.user_id = LOCAL_USER_ID.into();
    ctx.save_profile(profile).unwrap();
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hiring_post_is_discovered_with_the_address_to_email() {
    let (ctx, _dir) = fixture_context().await;
    orchestrator::start_job_hunt(
        ctx.clone(),
        JobHuntOptions {
            discover_only: true,
            sources: vec!["LinkedIn Posts".into()],
        },
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let jobs = ctx.store.list::<Job>().unwrap();
    let post = jobs
        .iter()
        .find(|j| j.source == "LinkedIn Posts")
        .expect("the post is saved as a job");
    assert_eq!(
        post.apply_email.as_deref(),
        Some("careers@brightpath.example")
    );
    assert_eq!(post.contact_name.as_deref(), Some("Priya Sharma"));
}

#[test]
fn an_email_posting_is_applied_to_by_gmail_with_the_approved_letter() {
    let mut job = Job::new(
        LOCAL_USER_ID,
        "LinkedIn Posts",
        "https://www.linkedin.com/feed/update/urn:li:activity:1/",
        "Brightpath",
        "Node.js Developer",
    );
    job.apply_email = Some("careers@brightpath.example".into());
    job.contact_name = Some("Priya Sharma".into());
    let mut profile = CandidateProfile::new(LOCAL_USER_ID);
    profile.personal.name = "Biswajit Panda".into();
    profile.personal.email = "me@example.com".into();
    let truth = CandidateTruth {
        profile,
        experiences: vec![],
        projects: vec![],
        master_resume_text: None,
    };

    let with_letter =
        prompts::email_message(&job, &truth, Some("Dear Priya,\n\nI'd like to apply.")).unwrap();
    assert_eq!(with_letter.to, "careers@brightpath.example");
    assert_eq!(
        with_letter.subject,
        "Application for Node.js Developer - Biswajit Panda"
    );
    assert_eq!(with_letter.body, "Dear Priya,\n\nI'd like to apply.");
    let without = prompts::email_message(&job, &truth, None).unwrap();
    assert!(without.body.starts_with("Hi Priya,"), "{}", without.body);
    assert!(without.body.contains("me@example.com"));

    let mut app = Application::new(LOCAL_USER_ID, &job.id, &job.url, "LinkedIn Posts");
    app.transition(ApplicationStatus::Analyzed, "").unwrap();
    app.transition(ApplicationStatus::ReadyForReview, "")
        .unwrap();
    app.transition(ApplicationStatus::Approved, "").unwrap();
    let params = ApplyParams {
        application: &app,
        job: &job,
        truth: &truth,
        resume_pdf: Some("C:/data/resumes/generated/brightpath/v1/Biswajit_Resume.pdf"),
        resume_docx: None,
        cover_letter_pdf: None,
        cover_letter_text: Some("Dear Priya,\n\nI'd like to apply."),
        answers: &[],
        previously_answered: &[],
        resuming: false,
    };
    let prompt = prompts::apply_prompt(&params);
    assert!(prompt.contains("APPROVED = true"));
    assert!(prompt.contains("Skill: email-application"));
    assert!(prompt.contains("careers@brightpath.example"));
    assert!(prompt.contains("Biswajit_Resume.pdf"));
    assert!(!prompt.contains("Skill: chrome-application"));

    // A normal posting still uses the application form.
    let form_job = Job::new(LOCAL_USER_ID, "LinkedIn", "https://x/1", "Acme", "Dev");
    assert!(prompts::email_message(&form_job, &truth, None).is_none());
    let prompt = prompts::apply_prompt(&ApplyParams {
        job: &form_job,
        ..params
    });
    assert!(prompt.contains("Skill: chrome-application"));
}
