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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_shared_job_becomes_a_review_ready_application_and_a_gmail_draft() {
    use job_hunter_core::agent::shared_job::{self, SharedJobInput};
    let (ctx, dir) = fixture_context().await;
    let shot = dir.path().join("whatsapp forward.png");
    std::fs::write(&shot, b"not really a png").unwrap();

    // Nothing to read is refused up front.
    let empty = SharedJobInput {
        text: "hi".into(),
        files: vec![],
        draft_email: true,
    };
    assert!(shared_job::start_shared_job(ctx.clone(), empty)
        .await
        .is_err());
    let wrong = SharedJobInput {
        text: String::new(),
        files: vec![dir.path().join("a.exe")],
        draft_email: true,
    };
    assert!(shared_job::start_shared_job(ctx.clone(), wrong)
        .await
        .is_err());

    let run = shared_job::start_shared_job(
        ctx.clone(),
        SharedJobInput {
            text: "Hiring Node.js developer, send CV to jobs@acmelabs.example".into(),
            files: vec![shot],
            draft_email: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(run.kind, RunKind::SharedJob);
    wait_idle(&ctx).await;

    let shared_copy = ctx
        .paths
        .run_dir(&run.id)
        .join("shared")
        .join("01-whatsapp-forward.png");
    assert!(shared_copy.is_file(), "the screenshot is kept with the run");
    let job = ctx
        .store
        .list::<Job>()
        .unwrap()
        .into_iter()
        .find(|j| j.source == "Shared")
        .expect("job saved");
    assert_eq!(job.apply_email.as_deref(), Some("jobs@acmelabs.example"));
    let app = ctx
        .store
        .list::<Application>()
        .unwrap()
        .into_iter()
        .find(|a| a.job_id == job.id)
        .expect("application prepared");
    assert_eq!(
        app.status,
        ApplicationStatus::ReadyForReview,
        "drafting never sends"
    );
    assert!(app.email_drafted_at.is_some());
    assert!(app.resume_id.is_some());
    let told = ctx.list_notifications(5).unwrap();
    assert_eq!(told[0].kind, "EMAIL_DRAFTED");
    assert!(told[0].body.contains("jobs@acmelabs.example"));
}

#[test]
fn a_draft_needs_no_address() {
    let job = Job::new(LOCAL_USER_ID, "Shared", "shared://1", "Acme", "Dev");
    let mut profile = CandidateProfile::new(LOCAL_USER_ID);
    profile.personal.name = "Biswajit Panda".into();
    let truth = CandidateTruth {
        profile,
        experiences: vec![],
        projects: vec![],
        master_resume_text: None,
    };
    let email = prompts::draft_email_message(&job, &truth, Some("Dear team,"));
    assert_eq!(email.to, "");
    assert_eq!(email.subject, "Application for Dev - Biswajit Panda");
    let prompt = prompts::email_draft_prompt(&email, Some("C:/x/Biswajit_Resume.pdf"));
    assert!(prompt.contains("NEVER click Send"));
    assert!(prompt.contains("Biswajit_Resume.pdf"));
}

async fn wait_for(ctx: &Arc<AppContext>, done: impl Fn(&AppContext) -> bool) {
    for _ in 0..600 {
        if done(ctx) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition not reached");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pasted_link_is_read_prepared_and_applied_to_when_asked() {
    use job_hunter_core::agent::job_link;
    let (ctx, _dir) = fixture_context().await;
    assert!(job_link::start_job_link(ctx.clone(), "not a link", true)
        .await
        .is_err());

    let url = "https://www.naukri.com/job-listings-full-stack-developer-linkline-123";
    let run = job_link::start_job_link(ctx.clone(), url, true)
        .await
        .unwrap();
    assert_eq!(run.kind, RunKind::JobLink);
    let job_url = url.to_string();
    // Prepared, then approved and applied in its own run (the mock never
    // submits, so it ends in manual action).
    wait_for(&ctx, |c| {
        c.store
            .list::<Application>()
            .unwrap()
            .iter()
            .any(|a| a.status == ApplicationStatus::ManualActionRequired)
    })
    .await;
    wait_idle(&ctx).await;
    let job = ctx
        .store
        .list::<Job>()
        .unwrap()
        .into_iter()
        .find(|j| j.url == job_url)
        .unwrap();
    assert_eq!(
        (
            job.source.as_str(),
            job.title.as_str(),
            job.company.as_str()
        ),
        ("Naukri", "Full Stack Developer", "Linkline Labs")
    );
    let app = ctx
        .store
        .list::<Application>()
        .unwrap()
        .into_iter()
        .find(|a| a.job_id == job.id)
        .unwrap();
    assert!(
        app.approved_at.is_some(),
        "approved through the normal gate"
    );
    assert!(app
        .status_history
        .iter()
        .any(|h| h.status == ApplicationStatus::Approved));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_apply_now_a_link_stops_at_review() {
    use job_hunter_core::agent::job_link;
    let (ctx, _dir) = fixture_context().await;
    job_link::start_job_link(ctx.clone(), "https://www.linkedin.com/jobs/view/42/", false)
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let apps = ctx.store.list::<Application>().unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].status, ApplicationStatus::ReadyForReview);
    // Sent right after the run finishes.
    wait_for(&ctx, |c| {
        c.list_notifications(5)
            .unwrap()
            .iter()
            .any(|n| n.kind == "JOB_LINK_READY")
    })
    .await;
}
