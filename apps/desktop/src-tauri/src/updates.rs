//! About info and app updates.
//!
//! Update discovery goes through the Job Hunter server
//! (`job_hunter_core::updates`). Installing uses `tauri-plugin-updater`,
//! which downloads the installer, verifies its signature against the public
//! key built into this binary, then runs it (on Windows that exits the app
//! and the installer relaunches it). Builds without that key -- local
//! builds, or CI without the signing secrets -- still find updates, but the
//! UI offers a plain download instead of installing in place.

use std::sync::Arc;

use job_hunter_core::backend_url;
use job_hunter_core::error::{CoreError, UserFacingError};
use job_hunter_core::updates::{self, UpdateCheck};
use job_hunter_core::AppContext;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

type R<T> = Result<T, UserFacingError>;

/// Minisign public key for verifying updates, baked in by CI from the
/// `TAURI_UPDATER_PUBKEY` secret (see .github/workflows/build.yml).
const UPDATER_PUBKEY: Option<&str> = option_env!("JOB_HUNTER_UPDATER_PUBKEY");

pub const EVENT_UPDATE_PROGRESS: &str = "update:progress";

fn updater_pubkey() -> Option<&'static str> {
    UPDATER_PUBKEY.map(str::trim).filter(|k| !k.is_empty())
}

fn err(message: impl std::fmt::Display) -> UserFacingError {
    CoreError::Other(message.to_string()).into()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    name: String,
    version: String,
    server_url: String,
    server_built_in: bool,
    updater_enabled: bool,
    data_dir: String,
    platform: String,
}

#[tauri::command]
pub async fn get_about_info(app: AppHandle, ctx: State<'_, Arc<AppContext>>) -> R<AboutInfo> {
    let info = app.package_info();
    Ok(AboutInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
        server_url: backend_url::configured(),
        server_built_in: backend_url::is_built_in(),
        updater_enabled: updater_pubkey().is_some(),
        data_dir: ctx.paths.root.display().to_string(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    })
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> R<UpdateCheck> {
    let server = backend_url::configured();
    let builds = updates::fetch_builds(&server)
        .await
        .map_err(UserFacingError::from)?;
    let current = app.package_info().version.to_string();
    let can_install = updater_pubkey().is_some() && cfg!(windows);
    Ok(updates::evaluate(&server, &current, &builds, can_install))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
    finished: bool,
}

/// Download, verify and install the latest signed build. On Windows the app
/// exits while the installer runs and is relaunched afterwards, so a
/// successful call normally never returns.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> R<()> {
    let pubkey = updater_pubkey().ok_or_else(|| {
        err("This build can't install updates by itself. Download the new installer instead.")
    })?;
    let endpoint = format!("{}/updates/windows/latest.json", backend_url::configured());
    let endpoint = endpoint.parse().map_err(err)?;
    let update = app
        .updater_builder()
        .pubkey(pubkey)
        .endpoints(vec![endpoint])
        .map_err(err)?
        .build()
        .map_err(err)?
        .check()
        .await
        .map_err(err)?
        .ok_or_else(|| err("You're already on the latest version."))?;

    tracing::info!(version = %update.version, "downloading update");
    let mut downloaded = 0u64;
    let progress_app = app.clone();
    let finished_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress_app.emit(
                    EVENT_UPDATE_PROGRESS,
                    Progress {
                        downloaded,
                        total,
                        finished: false,
                    },
                );
            },
            move || {
                let _ = finished_app.emit(
                    EVENT_UPDATE_PROGRESS,
                    Progress {
                        downloaded: 0,
                        total: None,
                        finished: true,
                    },
                );
            },
        )
        .await
        .map_err(err)?;
    tracing::info!("update installed; restarting");
    app.restart();
}
