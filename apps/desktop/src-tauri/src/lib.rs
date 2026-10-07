//! Tauri shell: creates the core `AppContext`, exposes commands to the React
//! UI and forwards agent events, status, sync status, log entries and data
//! change notifications as Tauri events.

mod commands;
mod updates;

use std::sync::Arc;

use job_hunter_core::logging::{self, LogBuffer};
use job_hunter_core::paths::AppPaths;
use job_hunter_core::remote::RemoteClient;
use job_hunter_core::AppContext;
use tauri::{AppHandle, Emitter, Manager};

pub const EVENT_AGENT: &str = "agent:event";
pub const EVENT_AGENT_STATUS: &str = "agent:status";
pub const EVENT_SYNC_STATUS: &str = "sync:status";
pub const EVENT_LOG: &str = "log:entry";
pub const EVENT_DATA_CHANGED: &str = "data:changed";
pub const EVENT_REMOTE_STATUS: &str = "remote:status";

fn forward_events(app: AppHandle, ctx: Arc<AppContext>) {
    let mut events = ctx.agent.bus.subscribe_events();
    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(ev) => {
                    if ev.kind == "NOTIFICATION" {
                        show_os_notification(&h, &ev.data);
                    }
                    let _ = h.emit(EVENT_AGENT, &ev);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
    let mut status = ctx.agent.bus.subscribe_status();
    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match status.recv().await {
                Ok(s) => {
                    let _ = h.emit(EVENT_AGENT_STATUS, &s);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
    let mut sync = ctx.sync.subscribe();
    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match sync.recv().await {
                Ok(s) => {
                    let _ = h.emit(EVENT_SYNC_STATUS, &s);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
    let mut logs = ctx.logs.subscribe();
    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match logs.recv().await {
                Ok(entry) => {
                    let _ = h.emit(EVENT_LOG, &entry);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
    let mut changes = ctx.store.subscribe();
    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        // Coalesce bursts of writes into one notification per collection.
        loop {
            match changes.recv().await {
                Ok(collection) => {
                    let mut set = std::collections::BTreeSet::new();
                    set.insert(collection);
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                    while let Ok(c) = changes.try_recv() {
                        set.insert(c);
                    }
                    let _ = h.emit(EVENT_DATA_CHANGED, set.into_iter().collect::<Vec<_>>());
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
}

/// A Windows notification for things the user should hear about even when
/// the window is in the background (see `AppContext::notify`).
fn show_os_notification(app: &AppHandle, data: &serde_json::Value) {
    use tauri_plugin_notification::NotificationExt;
    let text = |key: &str| {
        data.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let title = text("title");
    if title.is_empty() {
        return;
    }
    let mut builder = app.notification().builder().title(title);
    let body = text("body");
    if !body.is_empty() {
        builder = builder.body(body);
    }
    if let Err(e) = builder.show() {
        tracing::warn!(error = %e, "could not show a desktop notification");
    }
}

fn forward_remote_events(app: AppHandle, remote: Arc<RemoteClient>) {
    let mut status = remote.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match status.recv().await {
                Ok(s) => {
                    let _ = app.emit(EVENT_REMOTE_STATUS, &s);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Development convenience: a `.env` in the repo (or any parent directory)
    // can set JOB_HUNTER_DATA_DIR, MOCK_MODE, CLAUDE_CLI_PATH, ...
    let _ = dotenvy::dotenv();
    let paths = match AppPaths::detect() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Job Hunter could not create its data directory: {e}");
            std::process::exit(1);
        }
    };
    let logs = LogBuffer::new(5000, Some(paths.logs_dir()));
    logging::init(logs.clone());
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Job Hunter starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            let ctx =
                tauri::async_runtime::block_on(AppContext::init(paths.clone(), logs.clone()))?;
            forward_events(handle.clone(), ctx.clone());
            tauri::async_runtime::spawn(job_hunter_core::agent::profile_sync::run_auto_sync(
                ctx.clone(),
            ));
            tauri::async_runtime::spawn(job_hunter_core::agent::inbox::run_inbox_checks(
                ctx.clone(),
            ));
            let remote = RemoteClient::new(ctx.clone());
            tauri::async_runtime::spawn(remote.clone().run());
            forward_remote_events(handle.clone(), remote.clone());
            app.manage(ctx);
            app.manage(remote);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_status,
            commands::get_settings,
            commands::save_settings,
            commands::get_server_info,
            updates::get_about_info,
            updates::check_for_update,
            updates::install_update,
            commands::test_backend,
            commands::backend_register,
            commands::backend_login,
            commands::backend_logout,
            commands::get_sync_status,
            commands::get_candidate_profile,
            commands::save_candidate_profile,
            commands::list_experiences,
            commands::save_experience,
            commands::delete_experience,
            commands::list_projects,
            commands::draft_project,
            commands::save_project,
            commands::delete_project,
            commands::mark_skills_known,
            commands::list_platform_profiles,
            commands::get_publishing_plan,
            commands::suggest_project_picks,
            commands::save_publishing_plan,
            commands::sync_platform_profile,
            commands::answer_platform_questions,
            commands::set_platform_auto_sync,
            commands::queue_profile_updates,
            commands::add_shared_job,
            commands::apply_from_link,
            commands::list_notifications,
            commands::check_inbox_now,
            commands::restore_application_files,
            commands::mark_notifications_read,
            commands::import_master_resume,
            commands::parse_master_resume,
            commands::get_master_resume_text,
            commands::list_jobs,
            commands::get_job,
            commands::set_job_saved,
            commands::reject_job,
            commands::delete_job,
            commands::list_applications,
            commands::get_application,
            commands::approve_application,
            commands::reject_application,
            commands::apply_application,
            commands::answer_application_questions,
            commands::mark_manual_application_complete,
            commands::set_application_status,
            commands::add_application_note,
            commands::list_resumes,
            commands::update_resume_content,
            commands::update_cover_letter,
            commands::read_text_file,
            commands::list_answers,
            commands::save_answer,
            commands::delete_answer,
            commands::get_agent_status,
            commands::start_job_hunt,
            commands::stop_job_hunt,
            commands::pause_agent,
            commands::resume_agent,
            commands::discover_jobs,
            commands::prepare_applications,
            commands::analyze_job,
            commands::generate_resume,
            commands::generate_cover_letter,
            commands::list_agent_runs,
            commands::get_run_events,
            commands::get_logs,
            commands::clear_logs,
            commands::export_logs,
            commands::open_url,
            commands::open_path,
            commands::get_dashboard,
            commands::get_remote_status,
            commands::remote_register,
            commands::remote_login,
            commands::remote_logout,
            commands::remote_create_pairing_code,
            commands::remote_list_devices,
            commands::remote_revoke_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Job Hunter");
}
