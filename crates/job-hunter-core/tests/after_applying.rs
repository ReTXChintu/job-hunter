//! What happens once an application is sent: its generated files are
//! deleted (content kept, files re-creatable), and Gmail is watched for the
//! employer's reply, spam included.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::inbox;
use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::agent::steps;
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

/// A mock job hunt on LinkedIn, then one application approved and sent
/// (the mock "submits" only when told to simulate it).
async fn applied_application() -> (Arc<AppContext>, tempfile::TempDir, Application) {
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
            discover_only: false,
            sources: vec!["LinkedIn".into()],
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
async fn sent_applications_lose_their_files_but_can_get_them_back() {
    let (ctx, _dir, application) = applied_application().await;
    let resume: Resume = ctx
        .store
        .require(application.resume_id.as_ref().unwrap())
        .unwrap();
    assert!(
        resume.pdf_path.is_none() && resume.docx_path.is_none(),
        "files are gone after applying"
    );
    assert!(resume.files_deleted_at.is_some());
    assert!(resume.content.is_some(), "the content stays");
    let letter: CoverLetter = ctx
        .store
        .require(application.cover_letter_id.as_ref().unwrap())
        .unwrap();
    assert!(letter.pdf_path.is_none() && letter.files_deleted_at.is_some());
    let job: Job = ctx.store.require(&application.job_id).unwrap();
    let folder = steps::document_dir(&ctx, &job.company, &job.title, &job.id, resume.version);
    assert!(
        !folder.exists(),
        "the sent job's folder is removed: {}",
        folder.display()
    );
    let others: Vec<_> = ctx
        .list_applications()
        .unwrap()
        .into_iter()
        .filter(|a| a.application.id != application.id && a.application.resume_id.is_some())
        .collect();
    assert!(!others.is_empty());
    for other in others {
        let r: Resume = ctx
            .store
            .require(other.application.resume_id.as_ref().unwrap())
            .unwrap();
        assert!(
            r.pdf_path
                .as_deref()
                .is_some_and(|p| Path::new(p).is_file()),
            "applications not sent yet keep their files"
        );
    }

    steps::restore_application_files(&ctx, &application.id)
        .await
        .unwrap();
    let resume: Resume = ctx
        .store
        .require(application.resume_id.as_ref().unwrap())
        .unwrap();
    let pdf = resume.pdf_path.expect("re-created");
    assert!(Path::new(&pdf).is_file());
    assert!(pdf.ends_with("Asha_Resume.pdf"), "{pdf}");
    assert!(resume.files_deleted_at.is_none());
    assert!(!resume.user_edited, "re-creating isn't an edit");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_stay_when_the_user_turns_deletion_off() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let mut settings = ctx.settings().await;
    settings.resume.delete_files_after_applied = false;
    ctx.save_settings(settings).await.unwrap();
    let mut application = Application::new(LOCAL_USER_ID, "job-1", "https://x/1", "LinkedIn");
    application.status = ApplicationStatus::Applied;
    let file = ctx
        .paths
        .generated_dir()
        .join("acme")
        .join("dev-job1")
        .join("v1")
        .join("Asha_Resume.pdf");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"pdf").unwrap();
    let mut resume = Resume::new_generated(LOCAL_USER_ID, "job-1", "Acme", "Dev", 1);
    resume.pdf_path = Some(file.to_string_lossy().to_string());
    ctx.store.put(&resume).unwrap();
    steps::cleanup_after_applied(&ctx, &application).await;
    assert!(file.is_file());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replies_are_found_in_gmail_spam_included_and_reported_once() {
    let (ctx, _dir, application) = applied_application().await;
    assert_eq!(inbox::tracked_applications(&ctx).unwrap().len(), 1);

    inbox::start_inbox_check(ctx.clone()).await.unwrap();
    wait_idle(&ctx).await;
    let updated: Application = ctx.store.require(&application.id).unwrap();
    assert_eq!(updated.replies.len(), 1);
    let reply = &updated.replies[0];
    assert_eq!(
        (reply.kind.as_str(), reply.folder.as_str()),
        ("INTERVIEW", "SPAM")
    );
    let told = ctx.list_notifications(5).unwrap();
    assert_eq!(told[0].kind, "APPLICATION_REPLY");
    assert!(
        told[0].title.ends_with("wants to interview you (in Spam)"),
        "{}",
        told[0].title
    );
    assert_eq!(told[0].link_id.as_deref(), Some(application.id.as_str()));

    // The same email found again isn't added or announced twice.
    inbox::start_inbox_check(ctx.clone()).await.unwrap();
    wait_idle(&ctx).await;
    let updated: Application = ctx.store.require(&application.id).unwrap();
    assert_eq!(updated.replies.len(), 1);
    assert_eq!(
        ctx.list_notifications(20)
            .unwrap()
            .iter()
            .filter(|n| n.kind == "APPLICATION_REPLY")
            .count(),
        1
    );

    // Just checked, so the scheduler waits; turned off, it never runs.
    assert!(inbox::inbox_tick(&ctx).await.unwrap().is_none());
    let mut settings = ctx.settings().await;
    settings.inbox_check_hours = 0;
    ctx.save_settings(settings).await.unwrap();
    assert!(inbox::inbox_tick(&ctx).await.unwrap().is_none());
}

#[tokio::test]
async fn with_nothing_sent_there_is_nothing_to_watch() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    assert!(inbox::tracked_applications(&ctx).unwrap().is_empty());
    assert!(inbox::inbox_tick(&ctx).await.unwrap().is_none());
    assert!(inbox::start_inbox_check(ctx.clone()).await.is_err());
}
