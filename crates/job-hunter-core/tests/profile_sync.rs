//! Job-site profile updates on the mock runner: choosing projects, the
//! confirmation gate, a site that needs an answer, automatic re-sync after a
//! profile edit, and marking a "missing" skill as known.

use std::sync::Arc;
use std::time::Duration;

use job_hunter_core::agent::profile_sync::{self as ps, AUTO_SYNC_QUIET};
use job_hunter_core::domain::*;
use job_hunter_core::error::CoreError;
use job_hunter_core::settings::ProfileSyncMode;
use job_hunter_core::util::now;
use job_hunter_core::AppContext;

async fn wait_idle(ctx: &Arc<AppContext>) -> AgentStatus {
    for _ in 0..600 {
        let s = ctx.agent.status().await;
        if !s.state.is_running() && !s.paused {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("agent did not become idle");
}

fn project(name: &str) -> Project {
    Project {
        id: String::new(),
        user_id: String::new(),
        experience_id: None,
        name: name.into(),
        description: format!("{name} description"),
        role: "Full Stack Developer".into(),
        technologies: vec!["Node.js".into()],
        responsibilities: vec![],
        achievements: vec![],
        url: String::new(),
        created_at: now(),
        updated_at: now(),
    }
}

async fn ctx_with_profile() -> (Arc<AppContext>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let mut profile = ctx.profile().unwrap();
    profile.personal.name = "Ada Lovelace".into();
    profile.personal.email = "ada@example.com".into();
    profile.personal.current_title = "Technical Team Lead".into();
    profile.preferences.target_roles = vec!["Backend Developer".into()];
    profile.skills.backend = vec!["Node.js".into()];
    ctx.save_profile(profile).unwrap();
    for name in [
        "DNALog",
        "Sort-A-Snap",
        "Signisure",
        "Clump CRM",
        "Mediniv",
        "Lens Translate",
    ] {
        ctx.save_project(project(name)).unwrap();
    }
    (ctx, dir)
}

/// Ask Claude for picks and confirm them as-is, like clicking Proceed.
async fn confirm_suggested_projects(ctx: &Arc<AppContext>) -> PublishingPlan {
    let picks = ps::suggest_project_picks(ctx).await.unwrap();
    let ids = picks
        .iter()
        .filter(|p| p.selected)
        .map(|p| p.project_id.clone())
        .collect();
    ps::save_publishing_plan(ctx, ids, picks).unwrap()
}

fn platform(ctx: &AppContext, slug: &str) -> PlatformProfile {
    ctx.store.require::<PlatformProfile>(slug).unwrap()
}

/// Midday and 1 a.m., local time: outside and inside the default night window.
const NOON: u8 = 12;
const ONE_AM: u8 = 1;

async fn set_mode(ctx: &AppContext, mode: ProfileSyncMode) {
    let mut settings = ctx.settings().await;
    settings.profile_sync.mode = mode;
    ctx.save_settings(settings).await.unwrap();
}

/// LinkedIn updated once, then the profile changed (and settled): LinkedIn
/// is behind.
async fn linkedin_behind(ctx: &Arc<AppContext>) {
    confirm_suggested_projects(ctx).await;
    ps::start_profile_sync(ctx.clone(), "LinkedIn", vec![], false)
        .await
        .unwrap();
    wait_idle(ctx).await;
    let mut profile = ctx.profile().unwrap();
    profile.summary = "Changed".into();
    ctx.save_profile(profile).unwrap();
    settle_edits(ctx);
}

/// Backdate every profile edit so the auto-sync quiet period has passed.
fn settle_edits(ctx: &AppContext) {
    let long_ago = now() - chrono::Duration::hours(1);
    let mut profile = ctx.profile().unwrap();
    profile.updated_at = long_ago;
    ctx.store.put(&profile).unwrap();
    for mut p in ctx.projects().unwrap() {
        p.updated_at = long_ago;
        ctx.store.put(&p).unwrap();
    }
    let mut plan = ps::publishing_plan(ctx).unwrap();
    plan.updated_at = long_ago;
    ctx.store.put(&plan).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claude_suggests_a_few_projects_and_nothing_is_published_before_proceed() {
    let (ctx, _dir) = ctx_with_profile().await;

    let picks = ps::suggest_project_picks(&ctx).await.unwrap();
    assert_eq!(picks.len(), 6, "every project gets a verdict");
    assert_eq!(picks.iter().filter(|p| p.selected).count(), 4);
    assert!(picks.iter().all(|p| !p.reason.is_empty()));
    assert!(
        ps::publishing_plan(&ctx).unwrap().confirmed_at.is_none(),
        "a suggestion saves nothing"
    );

    let err = ps::start_profile_sync(ctx.clone(), "LinkedIn", vec![], false)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)), "{err:?}");
    assert!(ctx
        .store
        .get::<PlatformProfile>("linkedin")
        .unwrap()
        .is_none());

    // The user unticks one of Claude's picks before proceeding.
    let mut ids: Vec<String> = picks
        .iter()
        .filter(|p| p.selected)
        .map(|p| p.project_id.clone())
        .collect();
    ids.pop();
    let plan = ps::save_publishing_plan(&ctx, ids.clone(), picks).unwrap();
    assert!(plan.confirmed_at.is_some());
    assert_eq!(plan.featured_project_ids, ids);
    assert!(ps::save_publishing_plan(&ctx, vec!["nope".into()], vec![]).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_update_publishes_only_the_featured_projects() {
    let (ctx, _dir) = ctx_with_profile().await;
    let plan = confirm_suggested_projects(&ctx).await;

    let run = ps::start_profile_sync(ctx.clone(), "linkedin", vec![], false)
        .await
        .unwrap();
    assert_eq!(run.kind, RunKind::ProfileSync);
    assert_eq!(wait_idle(&ctx).await.state, AgentState::Completed);

    let li = platform(&ctx, "linkedin");
    assert_eq!(li.status, PlatformSyncStatus::Synced);
    assert!(li.last_synced_at.is_some());
    assert!(!li.changes.is_empty());
    let truth = ctx.candidate_truth().await.unwrap();
    let featured: Vec<String> = ps::featured_projects(&truth, &plan)
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert_eq!(li.synced_project_names, featured);
    assert_eq!(featured.len(), 4);

    let views = ps::platform_profiles(&ctx).await.unwrap();
    assert_eq!(views.len(), PROFILE_PLATFORMS.len());
    assert!(!views.iter().any(|v| v.out_of_date));
    assert!(
        !ctx.store
            .pending_ops()
            .iter()
            .any(|op| op.collection == "platform_profiles" || op.collection == "publishing_plans"),
        "platform state stays on this computer"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_site_question_is_answered_once_and_remembered() {
    let (ctx, _dir) = ctx_with_profile().await;
    confirm_suggested_projects(&ctx).await;

    ps::start_profile_sync(ctx.clone(), "Naukri", vec![], false)
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let naukri = platform(&ctx, "naukri");
    assert_eq!(naukri.status, PlatformSyncStatus::NeedsInput);
    assert_eq!(naukri.pending_questions.len(), 1);
    let question = naukri.pending_questions[0].question.clone();

    ps::answer_platform_questions(
        ctx.clone(),
        "Naukri",
        vec![ApplicationAnswer {
            question: question.clone(),
            answer: "1200000".into(),
            source: AnswerSource::User,
        }],
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let naukri = platform(&ctx, "naukri");
    assert_eq!(naukri.status, PlatformSyncStatus::Synced);
    assert!(naukri.pending_questions.is_empty());
    assert!(
        ctx.list_answers()
            .unwrap()
            .iter()
            .any(|a| a.question == question && a.answer == "1200000"),
        "the answer is saved for applications and later updates"
    );

    // A later update needs no answer: it's known now.
    ps::start_profile_sync(ctx.clone(), "Naukri", vec![], false)
        .await
        .unwrap();
    wait_idle(&ctx).await;
    assert_eq!(platform(&ctx, "naukri").status, PlatformSyncStatus::Synced);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_the_profile_re_syncs_only_the_sites_already_updated() {
    let (ctx, _dir) = ctx_with_profile().await;
    set_mode(&ctx, ProfileSyncMode::Immediate).await;
    // Nothing happens before the user has confirmed a plan.
    settle_edits(&ctx);
    assert!(ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
        .await
        .unwrap()
        .is_none());

    confirm_suggested_projects(&ctx).await;
    ps::start_profile_sync(ctx.clone(), "LinkedIn", vec![], false)
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let first_sync = platform(&ctx, "linkedin").last_synced_at;
    settle_edits(&ctx);
    assert!(
        ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
            .await
            .unwrap()
            .is_none(),
        "nothing changed"
    );

    // The user drops a featured project and adds a skill.
    let mut plan = ps::publishing_plan(&ctx).unwrap();
    let dropped = plan.featured_project_ids.remove(0);
    let dropped_name = ctx
        .projects()
        .unwrap()
        .into_iter()
        .find(|p| p.id == dropped)
        .unwrap()
        .name;
    ps::save_publishing_plan(&ctx, plan.featured_project_ids.clone(), plan.picks).unwrap();
    let mut profile = ctx.profile().unwrap();
    profile.skills.frontend.push("Next.js".into());
    ctx.save_profile(profile).unwrap();

    assert!(ps::platform_profiles(&ctx)
        .await
        .unwrap()
        .iter()
        .any(|v| v.profile.platform == "LinkedIn" && v.out_of_date));
    assert!(
        ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
            .await
            .unwrap()
            .is_none(),
        "waits for the edits to settle"
    );
    settle_edits(&ctx);
    let run = ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
        .await
        .unwrap()
        .expect("LinkedIn is behind and gets updated");
    assert_eq!(run.sources, ["LinkedIn"]);
    wait_idle(&ctx).await;
    let li = platform(&ctx, "linkedin");
    assert!(li.last_synced_at > first_sync);
    assert!(!li.synced_project_names.contains(&dropped_name));
    assert!(
        ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
            .await
            .unwrap()
            .is_none(),
        "Naukri and the rest were never updated by the user, so they stay untouched"
    );

    // Turning auto-sync off for LinkedIn stops it.
    ps::set_auto_sync(&ctx, "LinkedIn", false).unwrap();
    let mut profile = ctx.profile().unwrap();
    profile.summary = "Changed again".into();
    ctx.save_profile(profile).unwrap();
    settle_edits(&ctx);
    assert!(ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn content_hash_is_stable_and_tracks_changes() {
    let (ctx, _dir) = ctx_with_profile().await;
    let plan = PublishingPlan::new(LOCAL_USER_ID);
    let truth = ctx.candidate_truth().await.unwrap();
    let a = ps::content_hash(&ps::platform_content(&truth, &plan));
    assert_eq!(a, ps::content_hash(&ps::platform_content(&truth, &plan)));
    assert_eq!(a.len(), 16);
    let mut changed = truth.clone();
    changed.profile.summary = "New summary".into();
    assert_ne!(a, ps::content_hash(&ps::platform_content(&changed, &plan)));
    // Timestamps alone don't count as a change.
    let mut touched = truth.clone();
    touched.profile.updated_at = now() + chrono::Duration::days(1);
    assert_eq!(a, ps::content_hash(&ps::platform_content(&touched, &plan)));
}

#[tokio::test]
async fn a_missing_skill_marked_as_known_joins_the_profile_and_every_analysis() {
    let (ctx, _dir) = ctx_with_profile().await;
    let job = Job::new(LOCAL_USER_ID, "LinkedIn", "https://x/1", "Acme", "Dev");
    ctx.store.put(&job).unwrap();
    let analysis = JobAnalysis::from_result(
        &job,
        AnalysisResult {
            relevant: true,
            match_score: 80,
            matched_skills: vec!["Node.js".into()],
            missing_skills: vec!["NextJS".into(), "GraphQL".into()],
            ..Default::default()
        },
        None,
    );
    ctx.store.put(&analysis).unwrap();

    let profile = ctx
        .mark_skills_known(&["nextjs".into(), "Node.js".into()], "frontend")
        .unwrap();
    assert_eq!(profile.skills.frontend, ["nextjs"]);
    assert_eq!(
        profile.skills.backend,
        ["Node.js"],
        "already known skills aren't duplicated"
    );

    let a: JobAnalysis = ctx.store.require(&analysis.id).unwrap();
    assert_eq!(a.missing_skills, ["GraphQL"]);
    assert_eq!(a.matched_skills, ["Node.js", "NextJS"]);

    assert!(ctx.mark_skills_known(&[], "frontend").is_err());
    assert!(ctx.mark_skills_known(&["Rust".into()], "kitchen").is_err());
}

fn notification_kinds(ctx: &AppContext) -> Vec<String> {
    ctx.list_notifications(50)
        .unwrap()
        .into_iter()
        .map(|n| n.kind)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_update_that_stalls_resumes_by_itself_and_everyone_is_told() {
    let (ctx, _dir) = ctx_with_profile().await;
    confirm_suggested_projects(&ctx).await;
    let mut events = ctx.agent.bus.subscribe_events();

    // The mock stalls Indeed's first run; the update resumes that session.
    ps::start_profile_sync(ctx.clone(), "Indeed", vec![], false)
        .await
        .unwrap();
    assert_eq!(wait_idle(&ctx).await.state, AgentState::Completed);
    let indeed = platform(&ctx, "indeed");
    assert_eq!(indeed.status, PlatformSyncStatus::Synced);
    assert!(indeed.resume_session_id.is_none(), "nothing left to resume");

    let mut saw_resume = false;
    let mut saw_notification = false;
    while let Ok(ev) = events.try_recv() {
        saw_resume |= ev.message.contains("resuming where it left off");
        saw_notification |= ev.kind == "NOTIFICATION" && ev.message == "Indeed profile updated";
    }
    assert!(saw_resume, "the stall is reported and resumed");
    assert!(saw_notification, "the desktop is told");

    let stored = ctx.list_notifications(10).unwrap();
    assert_eq!(stored[0].kind, "PROFILE_UPDATED");
    assert_eq!(stored[0].link_page, "job-sites");
    assert!(
        ctx.store
            .pending_ops()
            .iter()
            .any(|op| op.collection == "notifications"),
        "notifications sync to the web app and phone"
    );
    ctx.mark_notifications_read(None).unwrap();
    assert!(ctx.list_notifications(10).unwrap().iter().all(|n| n.read));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_site_asking_for_details_notifies_and_answering_resumes_the_session() {
    let (ctx, _dir) = ctx_with_profile().await;
    confirm_suggested_projects(&ctx).await;
    ps::start_profile_sync(ctx.clone(), "Naukri", vec![], false)
        .await
        .unwrap();
    wait_idle(&ctx).await;
    let naukri = platform(&ctx, "naukri");
    assert_eq!(naukri.status, PlatformSyncStatus::NeedsInput);
    assert!(
        naukri.resume_session_id.is_some(),
        "the session is kept to resume"
    );
    assert!(notification_kinds(&ctx).contains(&"PROFILE_NEEDS_INPUT".to_string()));

    let question = naukri.pending_questions[0].question.clone();
    let mut events = ctx.agent.bus.subscribe_events();
    ps::answer_platform_questions(
        ctx.clone(),
        "Naukri",
        vec![ApplicationAnswer {
            question,
            answer: "1200000".into(),
            source: AnswerSource::User,
        }],
    )
    .await
    .unwrap();
    wait_idle(&ctx).await;
    let mut resumed = false;
    while let Ok(ev) = events.try_recv() {
        resumed |= ev
            .message
            .starts_with("Resuming the update of your Naukri profile");
    }
    assert!(resumed, "answers continue the stopped session");
    assert_eq!(platform(&ctx, "naukri").status, PlatformSyncStatus::Synced);
}

#[test]
fn linkedin_updates_never_notify_the_network() {
    let prompt = job_hunter_core::prompts::profile_sync_prompt(
        &job_hunter_core::prompts::ProfileSyncParams {
            platform: "LinkedIn",
            profile: &serde_json::json!({}),
            remove_projects: &[],
            known_answers: &[],
            previously_answered: &[],
            resume_path: None,
            resuming: false,
        },
    );
    assert!(prompt.contains("https://www.linkedin.com/mypreferences/d/share-profile-updates"));
    assert!(prompt.contains("off before every Save"));
    assert!(prompt.contains("Never save with it on"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn by_default_profiles_update_only_at_night() {
    let (ctx, _dir) = ctx_with_profile().await;
    assert_eq!(
        ctx.settings().await.profile_sync.mode,
        ProfileSyncMode::Nightly
    );
    linkedin_behind(&ctx).await;
    assert!(
        ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
            .await
            .unwrap()
            .is_none(),
        "not during the day, when applications run"
    );
    assert!(ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, ONE_AM)
        .await
        .unwrap()
        .is_some());
    wait_idle(&ctx).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ask_mode_notifies_once_and_waits_for_approval() {
    let (ctx, _dir) = ctx_with_profile().await;
    set_mode(&ctx, ProfileSyncMode::Ask).await;
    linkedin_behind(&ctx).await;
    for hour in [NOON, ONE_AM, NOON] {
        assert!(ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, hour)
            .await
            .unwrap()
            .is_none());
    }
    let notices: Vec<_> = ctx
        .list_notifications(20)
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == "PROFILES_BEHIND")
        .collect();
    assert_eq!(notices.len(), 1, "announced once, not every minute");
    assert!(notices[0].body.contains("LinkedIn"));

    // "Update all now" is the approval: it runs at once, even at noon.
    assert_eq!(ps::queue_out_of_date(&ctx).await.unwrap(), 1);
    let run = ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
        .await
        .unwrap();
    assert!(run.is_some());
    wait_idle(&ctx).await;
    assert!(platform(&ctx, "linkedin").queued_at.is_none());
    assert!(ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, NOON)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn off_mode_never_updates_by_itself() {
    let (ctx, _dir) = ctx_with_profile().await;
    set_mode(&ctx, ProfileSyncMode::Off).await;
    linkedin_behind(&ctx).await;
    for hour in [NOON, ONE_AM] {
        assert!(ps::auto_sync_tick(&ctx, AUTO_SYNC_QUIET, hour)
            .await
            .unwrap()
            .is_none());
    }
}

#[test]
fn the_night_window_wraps_past_midnight() {
    let mut s = job_hunter_core::settings::ProfileSyncSettings::default();
    assert!(s.in_night_window(0) && s.in_night_window(5));
    assert!(!s.in_night_window(6) && !s.in_night_window(23));
    s.nightly_hour = 22;
    assert!(s.in_night_window(22) && s.in_night_window(23) && s.in_night_window(3));
    assert!(!s.in_night_window(4) && !s.in_night_window(21));
}
