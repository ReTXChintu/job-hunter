//! Tauri commands. Each is a thin, typed wrapper around the core context or
//! orchestrator; errors are converted to `UserFacingError` for the UI.

use std::path::PathBuf;
use std::sync::Arc;

use job_hunter_core::agent::orchestrator::{self, JobHuntOptions};
use job_hunter_core::agent::profile_sync::{self, PlatformProfileView};
use job_hunter_core::backend_url;
use job_hunter_core::context::{
    ApplicationDetail, ApplicationListItem, Dashboard, JobDetail, JobListItem, SetupStatus,
};
use job_hunter_core::domain::*;
use job_hunter_core::error::UserFacingError;
use job_hunter_core::logging::{LogEntry, LogLevel};
use job_hunter_core::remote::{RemoteClient, RemoteDevice, RemoteStatus};
use job_hunter_core::settings::AppSettings;
use job_hunter_core::store::SyncStatus;
use job_hunter_core::AppContext;
use serde::Serialize;
use tauri::State;

type Ctx<'a> = State<'a, Arc<AppContext>>;
type RemoteCtx<'a> = State<'a, Arc<RemoteClient>>;
type R<T> = Result<T, UserFacingError>;

// ---- setup / settings -------------------------------------------------------

#[tauri::command]
pub async fn get_setup_status(ctx: Ctx<'_>) -> R<SetupStatus> {
    Ok(ctx.setup_status().await)
}

#[tauri::command]
pub async fn get_settings(ctx: Ctx<'_>) -> R<AppSettings> {
    Ok(ctx.settings().await)
}

#[tauri::command]
pub async fn save_settings(ctx: Ctx<'_>, settings: AppSettings) -> R<AppSettings> {
    ctx.save_settings(settings).await.map_err(Into::into)
}

async fn set_backend_device_name(ctx: &Arc<AppContext>, name: Option<String>) -> R<()> {
    let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else {
        return Ok(());
    };
    let mut settings = ctx.settings().await;
    settings.backend.device_name = name;
    ctx.save_settings(settings)
        .await
        .map_err(UserFacingError::from)?;
    Ok(())
}

/// Persist which account and server this desktop signed in to, so both
/// survive a restart (the device token itself lives in the OS keychain).
async fn remember_backend_account(ctx: &Arc<AppContext>, email: &str) -> R<()> {
    let mut settings = ctx.settings().await;
    let server = backend_url::configured();
    if settings.backend.account_email != email || settings.backend.backend_url != server {
        settings.backend.account_email = email.to_string();
        settings.backend.backend_url = server;
        ctx.save_settings(settings)
            .await
            .map_err(UserFacingError::from)?;
    }
    Ok(())
}

/// The server this build talks to (fixed at build time, see
/// `job_hunter_core::backend_url`). Shown read-only in Settings.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    url: String,
    built_in: bool,
}

#[tauri::command]
pub async fn get_server_info() -> R<ServerInfo> {
    Ok(ServerInfo {
        url: backend_url::configured(),
        built_in: backend_url::is_built_in(),
    })
}

#[tauri::command]
pub async fn test_backend(ctx: Ctx<'_>) -> R<()> {
    ctx.sync
        .test_connection(&backend_url::configured(), true)
        .await
        .map_err(Into::into)
}

async fn backend_sign_in(
    ctx: &Arc<AppContext>,
    email: String,
    password: String,
    device_name: Option<String>,
    is_register: bool,
) -> R<SyncStatus> {
    set_backend_device_name(ctx, device_name).await?;
    let device_name = ctx.settings().await.backend.device_name;
    let status = ctx
        .sync
        .configure(
            &backend_url::configured(),
            &email,
            &password,
            is_register,
            &device_name,
            true,
        )
        .await
        .map_err(UserFacingError::from)?;
    remember_backend_account(ctx, &email).await?;
    Ok(status)
}

#[tauri::command]
pub async fn backend_register(
    ctx: Ctx<'_>,
    remote: RemoteCtx<'_>,
    email: String,
    password: String,
    device_name: Option<String>,
) -> R<SyncStatus> {
    let status = backend_sign_in(&ctx, email, password, device_name, true).await?;
    // The same sign-in also connects this desktop for the phone.
    remote.credentials_changed().await;
    Ok(status)
}

#[tauri::command]
pub async fn backend_login(
    ctx: Ctx<'_>,
    remote: RemoteCtx<'_>,
    email: String,
    password: String,
    device_name: Option<String>,
) -> R<SyncStatus> {
    let status = backend_sign_in(&ctx, email, password, device_name, false).await?;
    remote.credentials_changed().await;
    Ok(status)
}

#[tauri::command]
pub async fn backend_logout(ctx: Ctx<'_>, remote: RemoteCtx<'_>) -> R<SyncStatus> {
    ctx.sync.sign_out().await.map_err(UserFacingError::from)?;
    let mut settings = ctx.settings().await;
    settings.backend = Default::default();
    ctx.save_settings(settings)
        .await
        .map_err(UserFacingError::from)?;
    remote.credentials_changed().await;
    Ok(ctx.sync.status().await)
}

#[tauri::command]
pub async fn get_sync_status(ctx: Ctx<'_>) -> R<SyncStatus> {
    Ok(ctx.sync.status().await)
}

// ---- candidate ---------------------------------------------------------------

#[tauri::command]
pub async fn get_candidate_profile(ctx: Ctx<'_>) -> R<CandidateProfile> {
    ctx.profile().map_err(Into::into)
}

#[tauri::command]
pub async fn save_candidate_profile(
    ctx: Ctx<'_>,
    profile: CandidateProfile,
) -> R<CandidateProfile> {
    ctx.save_profile(profile).map_err(Into::into)
}

#[tauri::command]
pub async fn list_experiences(ctx: Ctx<'_>) -> R<Vec<Experience>> {
    ctx.experiences().map_err(Into::into)
}

#[tauri::command]
pub async fn save_experience(ctx: Ctx<'_>, experience: Experience) -> R<Experience> {
    ctx.save_experience(experience).map_err(Into::into)
}

#[tauri::command]
pub async fn delete_experience(ctx: Ctx<'_>, id: String) -> R<()> {
    ctx.delete_experience(&id).map_err(Into::into)
}

#[tauri::command]
pub async fn list_projects(ctx: Ctx<'_>) -> R<Vec<Project>> {
    ctx.projects().map_err(Into::into)
}

/// Draft a project from the candidate's own description (not saved; the UI
/// opens it in the project editor for review).
#[tauri::command]
pub async fn draft_project(
    ctx: Ctx<'_>,
    description: String,
    experience_id: Option<String>,
) -> R<Project> {
    ctx.draft_project(&description, experience_id.as_deref())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn save_project(ctx: Ctx<'_>, project: Project) -> R<Project> {
    ctx.save_project(project).map_err(Into::into)
}

#[tauri::command]
pub async fn delete_project(ctx: Ctx<'_>, id: String) -> R<()> {
    ctx.delete_project(&id).map_err(Into::into)
}

/// "I know this": add skills a job analysis listed as missing to the profile.
#[tauri::command]
pub async fn mark_skills_known(
    ctx: Ctx<'_>,
    skills: Vec<String>,
    group: String,
) -> R<CandidateProfile> {
    ctx.mark_skills_known(&skills, &group).map_err(Into::into)
}

/// Look for employers' replies in Gmail (inbox and spam) now.
#[tauri::command]
pub async fn check_inbox_now(ctx: Ctx<'_>) -> R<AgentRun> {
    job_hunter_core::agent::inbox::start_inbox_check(ctx.inner().clone())
        .await
        .map_err(Into::into)
}

/// Re-create an application's resume and cover-letter files (deleted after
/// it was sent) from their saved content.
#[tauri::command]
pub async fn restore_application_files(ctx: Ctx<'_>, id: String) -> R<()> {
    job_hunter_core::agent::steps::restore_application_files(&ctx, &id)
        .await
        .map_err(Into::into)
}

// ---- notifications ---------------------------------------------------------------

#[tauri::command]
pub async fn list_notifications(ctx: Ctx<'_>, limit: Option<usize>) -> R<Vec<Notification>> {
    ctx.list_notifications(limit.unwrap_or(50))
        .map_err(Into::into)
}

/// Mark these notifications read, or all of them when `ids` is omitted.
#[tauri::command]
pub async fn mark_notifications_read(ctx: Ctx<'_>, ids: Option<Vec<String>>) -> R<()> {
    ctx.mark_notifications_read(ids.as_deref())
        .map_err(Into::into)
}

// ---- job-site profiles -------------------------------------------------------------

#[tauri::command]
pub async fn list_platform_profiles(ctx: Ctx<'_>) -> R<Vec<PlatformProfileView>> {
    profile_sync::platform_profiles(&ctx)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_publishing_plan(ctx: Ctx<'_>) -> R<PublishingPlan> {
    profile_sync::publishing_plan(&ctx).map_err(Into::into)
}

/// Claude's suggestion of projects to feature; nothing is saved.
#[tauri::command]
pub async fn suggest_project_picks(ctx: Ctx<'_>) -> R<Vec<ProjectPick>> {
    profile_sync::suggest_project_picks(ctx.inner())
        .await
        .map_err(Into::into)
}

/// The user confirmed which projects to feature ("Proceed").
#[tauri::command]
pub async fn save_publishing_plan(
    ctx: Ctx<'_>,
    featured_project_ids: Vec<String>,
    picks: Vec<ProjectPick>,
) -> R<PublishingPlan> {
    profile_sync::save_publishing_plan(&ctx, featured_project_ids, picks).map_err(Into::into)
}

/// `resume` continues the Claude session of an update that stopped part-way.
#[tauri::command]
pub async fn sync_platform_profile(
    ctx: Ctx<'_>,
    platform: String,
    resume: Option<bool>,
) -> R<AgentRun> {
    profile_sync::start_profile_sync(
        ctx.inner().clone(),
        &platform,
        vec![],
        resume.unwrap_or(false),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn answer_platform_questions(
    ctx: Ctx<'_>,
    platform: String,
    answers: Vec<ApplicationAnswer>,
) -> R<AgentRun> {
    profile_sync::answer_platform_questions(ctx.inner().clone(), &platform, answers)
        .await
        .map_err(Into::into)
}

/// A job the user found: pasted text and/or screenshots/PDFs. Claude reads
/// it, the resume and cover letter are generated, and (with `draft_email`)
/// the email is saved as a Gmail draft for the user to send.
#[tauri::command]
pub async fn add_shared_job(
    ctx: Ctx<'_>,
    text: String,
    files: Vec<String>,
    draft_email: Option<bool>,
) -> R<AgentRun> {
    job_hunter_core::agent::shared_job::start_shared_job(
        ctx.inner().clone(),
        job_hunter_core::agent::shared_job::SharedJobInput {
            text,
            files: files.into_iter().map(PathBuf::from).collect(),
            draft_email: draft_email.unwrap_or(true),
        },
    )
    .await
    .map_err(Into::into)
}

/// "Update all now": queue every site that's behind; they run one after
/// another as soon as the agent is free. Returns how many.
#[tauri::command]
pub async fn queue_profile_updates(ctx: Ctx<'_>) -> R<usize> {
    profile_sync::queue_out_of_date(&ctx)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn set_platform_auto_sync(
    ctx: Ctx<'_>,
    platform: String,
    enabled: bool,
) -> R<PlatformProfile> {
    profile_sync::set_auto_sync(&ctx, &platform, enabled).map_err(Into::into)
}

#[tauri::command]
pub async fn import_master_resume(ctx: Ctx<'_>, path: String) -> R<MasterResumeRef> {
    let ctx = ctx.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ctx.import_master_resume(&PathBuf::from(path)))
        .await
        .map_err(|e| UserFacingError {
            code: "IO".into(),
            message: e.to_string(),
            details: None,
            recoverable: true,
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn parse_master_resume(ctx: Ctx<'_>) -> R<CandidateProfile> {
    ctx.inner().parse_master_resume().await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_master_resume_text(ctx: Ctx<'_>) -> R<Option<String>> {
    ctx.master_resume_text().map_err(Into::into)
}

// ---- jobs ---------------------------------------------------------------------

#[tauri::command]
pub async fn list_jobs(ctx: Ctx<'_>) -> R<Vec<JobListItem>> {
    ctx.list_jobs().map_err(Into::into)
}

#[tauri::command]
pub async fn get_job(ctx: Ctx<'_>, id: String) -> R<JobDetail> {
    ctx.job_detail(&id).map_err(Into::into)
}

#[tauri::command]
pub async fn set_job_saved(ctx: Ctx<'_>, id: String, saved: bool) -> R<Job> {
    ctx.set_job_saved(&id, saved).map_err(Into::into)
}

#[tauri::command]
pub async fn reject_job(ctx: Ctx<'_>, id: String) -> R<Job> {
    ctx.reject_job(&id).map_err(Into::into)
}

#[tauri::command]
pub async fn delete_job(ctx: Ctx<'_>, id: String) -> R<()> {
    ctx.delete_job(&id).map_err(Into::into)
}

// ---- applications ---------------------------------------------------------------

#[tauri::command]
pub async fn list_applications(ctx: Ctx<'_>) -> R<Vec<ApplicationListItem>> {
    ctx.list_applications().map_err(Into::into)
}

#[tauri::command]
pub async fn get_application(ctx: Ctx<'_>, id: String) -> R<ApplicationDetail> {
    ctx.application_detail(&id).map_err(Into::into)
}

#[tauri::command]
pub async fn approve_application(
    ctx: Ctx<'_>,
    id: String,
    apply_now: Option<bool>,
) -> R<Application> {
    orchestrator::approve_application(ctx.inner().clone(), &id, apply_now.unwrap_or(true))
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn reject_application(
    ctx: Ctx<'_>,
    id: String,
    reason: Option<String>,
) -> R<Application> {
    orchestrator::reject_application(ctx.inner().clone(), &id, reason.as_deref().unwrap_or(""))
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn apply_application(ctx: Ctx<'_>, id: String, simulate: Option<String>) -> R<AgentRun> {
    // `simulate` is honoured only by the mock runner.
    orchestrator::apply_application(ctx.inner().clone(), &id, false, simulate)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn answer_application_questions(
    ctx: Ctx<'_>,
    id: String,
    answers: Vec<ApplicationAnswer>,
    save_for_reuse: Option<bool>,
) -> R<Application> {
    orchestrator::answer_questions(
        ctx.inner().clone(),
        &id,
        answers,
        save_for_reuse.unwrap_or(true),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn mark_manual_application_complete(
    ctx: Ctx<'_>,
    id: String,
    note: Option<String>,
) -> R<Application> {
    orchestrator::mark_manual_complete(ctx.inner().clone(), &id, note.as_deref().unwrap_or(""))
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn set_application_status(
    ctx: Ctx<'_>,
    id: String,
    status: ApplicationStatus,
    note: Option<String>,
) -> R<Application> {
    orchestrator::set_application_status(
        ctx.inner().clone(),
        &id,
        status,
        note.as_deref().unwrap_or(""),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn add_application_note(ctx: Ctx<'_>, id: String, note: String) -> R<Application> {
    ctx.add_application_note(&id, &note).map_err(Into::into)
}

// ---- resumes ----------------------------------------------------------------------

#[tauri::command]
pub async fn list_resumes(ctx: Ctx<'_>) -> R<Vec<Resume>> {
    ctx.list_resumes().map_err(Into::into)
}

#[tauri::command]
pub async fn update_resume_content(ctx: Ctx<'_>, id: String, content: ResumeDocument) -> R<Resume> {
    ctx.update_resume_content(&id, content)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_cover_letter(ctx: Ctx<'_>, id: String, text: String) -> R<CoverLetter> {
    ctx.update_cover_letter(&id, text).await.map_err(Into::into)
}

/// Read a generated text/HTML artifact for preview. Restricted to the app
/// data directory so the UI cannot read arbitrary files.
#[tauri::command]
pub async fn read_text_file(ctx: Ctx<'_>, path: String) -> R<String> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err(UserFacingError {
            code: "NOT_FOUND".into(),
            message: "file not found".into(),
            details: Some(path.clone()),
            recoverable: true,
        });
    }
    if !job_hunter_core::util::path_is_inside(&ctx.paths.root, &p) {
        return Err(UserFacingError {
            code: "VALIDATION".into(),
            message: "Only files inside the Job Hunter data directory can be read".into(),
            details: Some(path.clone()),
            recoverable: true,
        });
    }
    std::fs::read_to_string(&p).map_err(|e| UserFacingError {
        code: "IO".into(),
        message: e.to_string(),
        details: None,
        recoverable: true,
    })
}

// ---- answers ------------------------------------------------------------------------

#[tauri::command]
pub async fn list_answers(ctx: Ctx<'_>) -> R<Vec<AnswerRecord>> {
    ctx.list_answers().map_err(Into::into)
}

#[tauri::command]
pub async fn save_answer(ctx: Ctx<'_>, record: AnswerRecord) -> R<AnswerRecord> {
    ctx.save_answer(record).map_err(Into::into)
}

#[tauri::command]
pub async fn delete_answer(ctx: Ctx<'_>, id: String) -> R<()> {
    ctx.delete_answer(&id).map_err(Into::into)
}

// ---- agent ----------------------------------------------------------------------------

#[tauri::command]
pub async fn get_agent_status(ctx: Ctx<'_>) -> R<AgentStatus> {
    Ok(ctx.agent.status().await)
}

#[tauri::command]
pub async fn start_job_hunt(ctx: Ctx<'_>, options: Option<JobHuntOptions>) -> R<AgentRun> {
    orchestrator::start_job_hunt(ctx.inner().clone(), options.unwrap_or_default())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn stop_job_hunt(ctx: Ctx<'_>) -> R<bool> {
    Ok(ctx.agent.request_stop().await)
}

#[tauri::command]
pub async fn pause_agent(ctx: Ctx<'_>) -> R<()> {
    ctx.agent.pause().await.map_err(Into::into)
}

#[tauri::command]
pub async fn resume_agent(ctx: Ctx<'_>) -> R<()> {
    ctx.agent.resume().await.map_err(Into::into)
}

#[tauri::command]
pub async fn discover_jobs(ctx: Ctx<'_>, sources: Option<Vec<String>>) -> R<AgentRun> {
    orchestrator::start_job_hunt(
        ctx.inner().clone(),
        JobHuntOptions {
            discover_only: true,
            sources: sources.unwrap_or_default(),
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analyze_job(ctx: Ctx<'_>, job_id: String) -> R<AgentRun> {
    orchestrator::analyze_job(ctx.inner().clone(), job_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn generate_resume(ctx: Ctx<'_>, job_id: String) -> R<AgentRun> {
    orchestrator::generate_resume(ctx.inner().clone(), job_id)
        .await
        .map_err(Into::into)
}

/// Cover letters are produced together with the resume; regenerating one
/// regenerates the bundle (new version).
#[tauri::command]
pub async fn generate_cover_letter(ctx: Ctx<'_>, job_id: String) -> R<AgentRun> {
    orchestrator::generate_resume(ctx.inner().clone(), job_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_agent_runs(ctx: Ctx<'_>, limit: Option<usize>) -> R<Vec<AgentRun>> {
    ctx.list_runs(limit.unwrap_or(20)).map_err(Into::into)
}

#[tauri::command]
pub async fn get_run_events(
    ctx: Ctx<'_>,
    run_id: String,
    limit: Option<usize>,
) -> R<Vec<AgentEvent>> {
    ctx.run_events(&run_id, limit.unwrap_or(500))
        .map_err(Into::into)
}

// ---- logs -------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_logs(
    ctx: Ctx<'_>,
    min_level: Option<LogLevel>,
    limit: Option<usize>,
) -> R<Vec<LogEntry>> {
    Ok(ctx
        .logs
        .entries(min_level.unwrap_or(LogLevel::Debug), limit.unwrap_or(1000)))
}

#[tauri::command]
pub async fn clear_logs(ctx: Ctx<'_>) -> R<()> {
    ctx.logs.clear();
    Ok(())
}

#[tauri::command]
pub async fn export_logs(ctx: Ctx<'_>) -> R<String> {
    let path = ctx
        .paths
        .exports_dir()
        .join(format!("job-hunter-logs-{}.txt", chrono_stamp()));
    std::fs::write(&path, ctx.logs.export_text()).map_err(|e| UserFacingError {
        code: "IO".into(),
        message: e.to_string(),
        details: None,
        recoverable: true,
    })?;
    Ok(path.to_string_lossy().to_string())
}

fn chrono_stamp() -> String {
    job_hunter_core::util::now()
        .format("%Y%m%d-%H%M%S")
        .to_string()
}

// ---- misc ---------------------------------------------------------------------------------

#[tauri::command]
pub async fn open_url(ctx: Ctx<'_>, url: String) -> R<()> {
    ctx.open_url(&url).await.map_err(Into::into)
}

#[tauri::command]
pub async fn open_path(ctx: Ctx<'_>, path: String) -> R<()> {
    let p = PathBuf::from(&path);
    if !job_hunter_core::util::path_is_inside(&ctx.paths.root, &p) {
        return Err(UserFacingError {
            code: "VALIDATION".into(),
            message: "Only files inside the Job Hunter data directory can be opened".into(),
            details: Some(path.clone()),
            recoverable: true,
        });
    }
    job_hunter_core::chrome::open_path(&p).map_err(Into::into)
}

#[tauri::command]
pub async fn get_dashboard(ctx: Ctx<'_>) -> R<Dashboard> {
    ctx.dashboard().await.map_err(Into::into)
}

// ---- mobile companion app (relay) ------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    code: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

async fn set_device_name(ctx: &Arc<AppContext>, name: Option<String>) -> R<()> {
    let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else {
        return Ok(());
    };
    let mut settings = ctx.settings().await;
    settings.remote.device_name = name;
    ctx.save_settings(settings)
        .await
        .map_err(UserFacingError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn get_remote_status(remote: RemoteCtx<'_>) -> R<RemoteStatus> {
    Ok(remote.status().await)
}

#[tauri::command]
pub async fn remote_register(
    ctx: Ctx<'_>,
    remote: RemoteCtx<'_>,
    email: String,
    password: String,
    device_name: Option<String>,
) -> R<RemoteStatus> {
    set_device_name(&ctx, device_name).await?;
    // The mobile app connects through the same server as data sync: the
    // backend speaks the relay's protocol.
    remote
        .register(&backend_url::configured(), &email, &password, true)
        .await
        .map_err(UserFacingError::from)?;
    Ok(remote.status().await)
}

#[tauri::command]
pub async fn remote_login(
    ctx: Ctx<'_>,
    remote: RemoteCtx<'_>,
    email: String,
    password: String,
    device_name: Option<String>,
) -> R<RemoteStatus> {
    set_device_name(&ctx, device_name).await?;
    // The mobile app connects through the same server as data sync: the
    // backend speaks the relay's protocol.
    remote
        .login(&backend_url::configured(), &email, &password, true)
        .await
        .map_err(UserFacingError::from)?;
    Ok(remote.status().await)
}

#[tauri::command]
pub async fn remote_logout(remote: RemoteCtx<'_>) -> R<RemoteStatus> {
    remote.logout().await.map_err(UserFacingError::from)?;
    Ok(remote.status().await)
}

#[tauri::command]
pub async fn remote_create_pairing_code(remote: RemoteCtx<'_>) -> R<PairingCode> {
    let (code, expires_at) = remote
        .create_pairing_code()
        .await
        .map_err(UserFacingError::from)?;
    Ok(PairingCode { code, expires_at })
}

#[tauri::command]
pub async fn remote_list_devices(remote: RemoteCtx<'_>) -> R<Vec<RemoteDevice>> {
    remote.list_devices().await.map_err(Into::into)
}

#[tauri::command]
pub async fn remote_revoke_device(remote: RemoteCtx<'_>, device_id: String) -> R<RemoteStatus> {
    remote
        .revoke_device(&device_id)
        .await
        .map_err(UserFacingError::from)?;
    Ok(remote.status().await)
}
