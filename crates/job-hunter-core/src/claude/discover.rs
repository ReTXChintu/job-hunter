//! Locate the Claude CLI on any platform and query its version / auth status.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub authenticated: bool,
    pub auth_method: Option<String>,
    pub account: Option<String>,
    pub subscription: Option<String>,
    pub error: Option<String>,
}

/// A resolved Claude executable. On Windows the npm `.cmd` shim is unwrapped
/// to the real binary (or `node cli.js`) so we can spawn it directly with
/// proper argument quoting and kill it reliably.
#[derive(Debug, Clone)]
pub struct ClaudeCli {
    pub program: PathBuf,
    pub prefix_args: Vec<String>,
    pub display_path: PathBuf,
}

impl ClaudeCli {
    pub fn locate(override_path: Option<&str>) -> CoreResult<Self> {
        let candidate =
            find_executable(override_path).ok_or_else(|| CoreError::ClaudeCliUnavailable {
                details: "searched PATH and common install locations".into(),
            })?;
        Self::from_path(candidate)
    }

    pub fn from_path(path: PathBuf) -> CoreResult<Self> {
        let (program, prefix_args) = unwrap_shim(&path)?;
        Ok(Self {
            program,
            prefix_args,
            display_path: path,
        })
    }

    pub fn command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.prefix_args);
        // Never let an API key leak into the child: Job Hunter must use the
        // user's Claude Code login, not the Anthropic API.
        cmd.env_remove("ANTHROPIC_API_KEY");
        cmd.env_remove("ANTHROPIC_AUTH_TOKEN");
        cmd.env("CLAUDE_CODE_DISABLE_TERMINAL_TITLE", "1");
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000);
        cmd
    }

    pub async fn version(&self) -> CoreResult<String> {
        let out =
            run_with_timeout(self.command().arg("--version"), Duration::from_secs(20)).await?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() || text.is_empty() {
            return Err(CoreError::ClaudeCliUnavailable {
                details: format!(
                    "`claude --version` failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ),
            });
        }
        Ok(text)
    }

    pub async fn auth_status(&self) -> CoreResult<AuthStatus> {
        let out = run_with_timeout(
            self.command().args(["auth", "status"]),
            Duration::from_secs(30),
        )
        .await?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let json = crate::util::extract_json(&text).unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<AuthStatus>(&json) {
            return Ok(parsed);
        }
        // Older CLIs print plain text; treat "logged in" heuristically.
        let lower = text.to_lowercase();
        Ok(AuthStatus {
            logged_in: lower.contains("logged in") && !lower.contains("not logged in"),
            auth_method: None,
            email: None,
            subscription_type: None,
        })
    }

    pub async fn status(&self) -> ClaudeStatus {
        let mut status = ClaudeStatus {
            installed: true,
            path: Some(self.display_path.to_string_lossy().to_string()),
            version: None,
            authenticated: false,
            auth_method: None,
            account: None,
            subscription: None,
            error: None,
        };
        match self.version().await {
            Ok(v) => status.version = Some(v),
            Err(e) => {
                status.installed = false;
                status.error = Some(e.details().unwrap_or_else(|| e.to_string()));
                return status;
            }
        }
        match self.auth_status().await {
            Ok(a) => {
                status.authenticated = a.logged_in;
                status.auth_method = a.auth_method;
                status.account = a.email;
                status.subscription = a.subscription_type;
            }
            Err(e) => status.error = Some(e.to_string()),
        }
        status
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    #[serde(default)]
    pub logged_in: bool,
    #[serde(default)]
    pub auth_method: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub subscription_type: Option<String>,
}

async fn run_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
) -> CoreResult<std::process::Output> {
    cmd.stdin(Stdio::null());
    let child = cmd.spawn().map_err(|e| CoreError::ClaudeCliUnavailable {
        details: format!("spawn failed: {e}"),
    })?;
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(out),
        Ok(Err(e)) => Err(CoreError::ClaudeCliUnavailable {
            details: format!("process failed: {e}"),
        }),
        Err(_) => Err(CoreError::ClaudeCliUnavailable {
            details: "claude did not respond in time".into(),
        }),
    }
}

/// Search order: explicit override, PATH, then well-known install locations.
pub fn find_executable(override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path.map(str::trim).filter(|p| !p.is_empty()) {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
        if let Ok(found) = which::which(p) {
            return Some(found);
        }
    }
    let names: &[&str] = if cfg!(windows) {
        &["claude.exe", "claude.cmd", "claude"]
    } else {
        &["claude"]
    };
    for name in names {
        if let Ok(found) = which::which(name) {
            return Some(found);
        }
    }
    known_locations()
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn known_locations() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let home = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf());
    if let Some(home) = &home {
        for name in ["claude", "claude.exe", "claude.cmd"] {
            out.push(home.join(".local").join("bin").join(name));
            out.push(home.join(".claude").join("local").join(name));
            out.push(home.join(".npm-global").join("bin").join(name));
            out.push(home.join(".yarn").join("bin").join(name));
            out.push(home.join("bin").join(name));
        }
        // nvm (unix)
        if let Ok(entries) = std::fs::read_dir(home.join(".nvm").join("versions").join("node")) {
            for e in entries.flatten() {
                out.push(e.path().join("bin").join("claude"));
            }
        }
    }
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            out.push(Path::new(&appdata).join("npm").join("claude.cmd"));
            out.push(Path::new(&appdata).join("npm").join("claude.exe"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            out.push(
                Path::new(&local)
                    .join("Programs")
                    .join("claude")
                    .join("claude.exe"),
            );
        }
        for var in ["NVM_SYMLINK", "NVM_HOME"] {
            if let Ok(p) = std::env::var(var) {
                out.push(Path::new(&p).join("claude.cmd"));
                out.push(Path::new(&p).join("claude.exe"));
            }
        }
        out.push(PathBuf::from(r"C:\Program Files\nodejs\claude.cmd"));
    } else {
        for p in [
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
            "/usr/bin/claude",
            "/snap/bin/claude",
        ] {
            out.push(PathBuf::from(p));
        }
    }
    out
}

/// Windows npm shims are `.cmd` files that cannot be spawned directly with
/// safe quoting. Read the shim and resolve the real target.
fn unwrap_shim(path: &Path) -> CoreResult<(PathBuf, Vec<String>)> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    if ext.as_deref() != Some("cmd") && ext.as_deref() != Some("bat") {
        return Ok((path.to_path_buf(), vec![]));
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let content = std::fs::read_to_string(path).unwrap_or_default();
    // Find quoted paths in the shim, e.g. "%dp0%\node_modules\@anthropic-ai\claude-code\bin\claude.exe"
    let re = regex::Regex::new(r#""([^"]+)""#).unwrap();
    let mut js_target: Option<PathBuf> = None;
    for cap in re.captures_iter(&content) {
        let raw = cap[1]
            .replace("%dp0%", &dir.to_string_lossy())
            .replace("%~dp0", &dir.to_string_lossy());
        let candidate = PathBuf::from(raw.replace('/', "\\"));
        let lower = candidate.to_string_lossy().to_lowercase();
        if lower.ends_with(".exe") && candidate.is_file() {
            return Ok((candidate, vec![]));
        }
        if lower.ends_with(".js") && candidate.is_file() {
            js_target = Some(candidate);
        }
    }
    if let Some(js) = js_target {
        let node = dir.join("node.exe");
        let node = if node.is_file() {
            node
        } else {
            which::which("node").map_err(|_| CoreError::ClaudeCliUnavailable {
                details: "node.exe not found for the claude shim".into(),
            })?
        };
        return Ok((node, vec![js.to_string_lossy().to_string()]));
    }
    // Fall back to running through cmd.exe.
    Ok((
        PathBuf::from("cmd.exe"),
        vec!["/C".into(), path.to_string_lossy().to_string()],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shim_unwrap_prefers_exe() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("node_modules").join("bin");
        std::fs::create_dir_all(&exe).unwrap();
        let exe_file = exe.join("claude.exe");
        std::fs::write(&exe_file, b"MZ").unwrap();
        let shim = dir.path().join("claude.cmd");
        std::fs::write(
            &shim,
            "@ECHO off\r\n\"%dp0%\\node_modules\\bin\\claude.exe\"   %*\r\n",
        )
        .unwrap();
        let (program, args) = unwrap_shim(&shim).unwrap();
        assert_eq!(program, exe_file);
        assert!(args.is_empty());
    }

    #[test]
    fn non_shim_passes_through() {
        let p = PathBuf::from("/usr/local/bin/claude");
        let (program, args) = unwrap_shim(&p).unwrap();
        assert_eq!(program, p);
        assert!(args.is_empty());
    }
}
