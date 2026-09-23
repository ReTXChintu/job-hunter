//! Google Chrome and Claude in Chrome detection, plus headless PDF printing.
//! No custom browser automation lives here: all page interaction is done by
//! Claude Code through the Claude in Chrome extension.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Chrome Web Store id of the "Claude" (Claude in Chrome) extension.
pub const CLAUDE_IN_CHROME_EXTENSION_ID: &str = "fcoeoabgfenejglbffodgkkbkcdhcgfn";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChromeStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub extension_installed: bool,
    pub extension_version: Option<String>,
    pub profiles_checked: Vec<String>,
    pub error: Option<String>,
}

pub fn find_chrome(override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path.map(str::trim).filter(|p| !p.is_empty()) {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    for candidate in candidate_paths() {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let names: &[&str] = if cfg!(windows) {
        &["chrome.exe"]
    } else if cfg!(target_os = "macos") {
        &["Google Chrome"]
    } else {
        &[
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "chrome",
        ]
    };
    for name in names {
        if let Ok(p) = which::which(name) {
            return Some(p);
        }
    }
    None
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Ok(base) = std::env::var(var) {
                out.push(
                    Path::new(&base)
                        .join("Google")
                        .join("Chrome")
                        .join("Application")
                        .join("chrome.exe"),
                );
            }
        }
    } else if cfg!(target_os = "macos") {
        out.push(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
            out.push(home.join("Applications/Google Chrome.app/Contents/MacOS/Google Chrome"));
        }
    } else {
        for p in [
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/opt/google/chrome/chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
        ] {
            out.push(PathBuf::from(p));
        }
    }
    out
}

/// Chrome's user data directory for the default profile location.
pub fn user_data_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            out.push(
                Path::new(&local)
                    .join("Google")
                    .join("Chrome")
                    .join("User Data"),
            );
        }
    } else if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
        if cfg!(target_os = "macos") {
            out.push(home.join("Library/Application Support/Google/Chrome"));
        } else {
            out.push(home.join(".config/google-chrome"));
            out.push(home.join(".config/chromium"));
        }
    }
    out
}

/// Version string: on Windows read the versioned folder next to chrome.exe
/// (running `chrome --version` there just focuses a running instance).
pub async fn chrome_version(path: &Path) -> Option<String> {
    if cfg!(windows) {
        let dir = path.parent()?;
        let mut versions: Vec<String> = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .filter(|n| {
                n.chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                    && n.contains('.')
            })
            .collect();
        versions.sort();
        return versions.pop();
    }
    let mut cmd = tokio::process::Command::new(path);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let out = tokio::time::timeout(Duration::from_secs(10), cmd.output())
        .await
        .ok()?
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    text.split_whitespace()
        .find(|t| {
            t.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })
        .map(String::from)
}

/// Look for the Claude in Chrome extension in every Chrome profile.
pub fn claude_in_chrome_extension() -> (bool, Option<String>, Vec<String>) {
    let mut checked = Vec::new();
    let mut best: Option<String> = None;
    for base in user_data_dirs() {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !(name == "Default" || name.starts_with("Profile ")) {
                continue;
            }
            checked.push(name.clone());
            let ext_dir = e
                .path()
                .join("Extensions")
                .join(CLAUDE_IN_CHROME_EXTENSION_ID);
            if let Ok(versions) = std::fs::read_dir(&ext_dir) {
                for v in versions.flatten() {
                    let manifest = v.path().join("manifest.json");
                    if manifest.is_file() {
                        let version = v
                            .file_name()
                            .to_string_lossy()
                            .split('_')
                            .next()
                            .unwrap_or("")
                            .to_string();
                        if best.as_ref().map(|b| version > *b).unwrap_or(true) {
                            best = Some(version);
                        }
                    }
                }
            }
        }
    }
    (best.is_some(), best, checked)
}

pub async fn status(override_path: Option<&str>) -> ChromeStatus {
    let path = find_chrome(override_path);
    let (extension_installed, extension_version, profiles_checked) = claude_in_chrome_extension();
    match path {
        Some(p) => ChromeStatus {
            installed: true,
            version: chrome_version(&p).await,
            path: Some(p.to_string_lossy().to_string()),
            extension_installed,
            extension_version,
            profiles_checked,
            error: None,
        },
        None => ChromeStatus {
            installed: false,
            path: None,
            version: None,
            extension_installed,
            extension_version,
            profiles_checked,
            error: Some("Chrome executable not found".into()),
        },
    }
}

/// Open a URL in the user's default browser (Chrome when configured).
pub fn open_url(chrome: Option<&Path>, url: &str) -> CoreResult<()> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(CoreError::Validation(
            "Only http(s) URLs can be opened".into(),
        ));
    }
    if let Some(chrome) = chrome {
        let mut cmd = std::process::Command::new(chrome);
        cmd.arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if cmd.spawn().is_ok() {
            return Ok(());
        }
    }
    open::that_detached(url).map_err(|e| CoreError::other(format!("could not open browser: {e}")))
}

pub fn open_path(path: &Path) -> CoreResult<()> {
    if !path.exists() {
        return Err(CoreError::NotFound {
            entity: "file",
            id: path.to_string_lossy().to_string(),
        });
    }
    open::that_detached(path).map_err(|e| CoreError::other(format!("could not open file: {e}")))
}

/// Render an HTML file to PDF with headless Chrome using a throw-away profile
/// so the user's running browser is untouched.
pub async fn print_to_pdf(
    chrome: &Path,
    html: &Path,
    pdf: &Path,
    temp_dir: &Path,
) -> CoreResult<()> {
    let profile = tempfile::Builder::new()
        .prefix("jh-print-")
        .tempdir_in(temp_dir)?;
    let url = url::Url::from_file_path(html)
        .map_err(|_| CoreError::DocumentGeneration("invalid html path".into()))?;
    let mut cmd = tokio::process::Command::new(chrome);
    cmd.args([
        "--headless=new",
        "--disable-gpu",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-extensions",
        "--no-pdf-header-footer",
        "--run-all-compositor-stages-before-draw",
    ])
    .arg(format!("--user-data-dir={}", profile.path().display()))
    .arg(format!("--print-to-pdf={}", pdf.display()))
    .arg(url.as_str())
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let child = cmd.spawn().map_err(|e| {
        CoreError::DocumentGeneration(format!("could not start Chrome for PDF export: {e}"))
    })?;
    let out = tokio::time::timeout(Duration::from_secs(90), child.wait_with_output())
        .await
        .map_err(|_| CoreError::DocumentGeneration("Chrome PDF export timed out".into()))?
        .map_err(|e| CoreError::DocumentGeneration(e.to_string()))?;
    if !pdf.is_file() {
        return Err(CoreError::DocumentGeneration(format!(
            "Chrome did not produce a PDF: {}",
            crate::util::truncate(&String::from_utf8_lossy(&out.stderr), 400)
        )));
    }
    Ok(())
}
