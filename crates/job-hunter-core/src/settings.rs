use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CoreResult;

/// Non-secret application settings, persisted as JSON in the app data dir.
/// Secrets (device tokens) never live here; see `secrets`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_sources")]
    pub job_sources: Vec<JobSourceConfig>,
    #[serde(default = "default_max_jobs_per_source")]
    pub max_jobs_per_source: u32,
    #[serde(default = "default_max_applications_per_run")]
    pub max_applications_per_run: u32,
    /// Only `REVIEW_REQUIRED` is implemented on purpose: the agent never
    /// submits without explicit approval.
    #[serde(default)]
    pub approval_mode: ApprovalMode,
    /// Jobs older than this are skipped during discovery.
    #[serde(default = "default_recency_days")]
    pub recency_days: u32,
    #[serde(default)]
    pub resume: ResumeSettings,
    #[serde(default)]
    pub browser: BrowserSettings,
    #[serde(default)]
    pub claude: ClaudeSettings,
    #[serde(default)]
    pub mock_mode: bool,
    #[serde(default)]
    pub setup_completed: bool,
    #[serde(default)]
    pub minimum_match_score: u8,
    #[serde(default)]
    pub remote: RemoteSettings,
    #[serde(default)]
    pub backend: BackendSettings,
}

/// Non-secret configuration for the mobile companion app connection. The
/// device token issued by the relay lives in the OS credential store (see
/// `secrets::REMOTE_DEVICE_TOKEN_KEY`), never here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSettings {
    /// Base URL of a self-hosted `job-hunter-relay` instance, e.g.
    /// `https://relay.example.com`. Empty means the mobile companion app is
    /// not configured.
    #[serde(default)]
    pub relay_url: String,
    /// Display-only: the signed-in account's email, shown in Settings.
    #[serde(default)]
    pub account_email: String,
    /// This desktop's own device id on the relay, once registered.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Name this desktop registered itself under (shown in the phone's
    /// presence banner and this desktop's own device list).
    #[serde(default = "default_device_name")]
    pub device_name: String,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            relay_url: String::new(),
            account_email: String::new(),
            device_id: None,
            device_name: default_device_name(),
        }
    }
}

/// Non-secret configuration for the self-hosted `@job-hunter/backend`
/// connection (see `apps/backend`, `docs/backend.md`). This desktop's own
/// device token lives in the OS credential store (see
/// `secrets::BACKEND_DEVICE_TOKEN_KEY`), never here. Empty `backend_url`
/// means the desktop has not signed in and keeps working entirely
/// local-first, exactly as before this existed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackendSettings {
    #[serde(default)]
    pub backend_url: String,
    /// Display-only: the signed-in account's email, shown in Settings.
    #[serde(default)]
    pub account_email: String,
    /// This desktop's own device id on the backend, once registered.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Name this desktop registered itself under.
    #[serde(default = "default_device_name")]
    pub device_name: String,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            backend_url: String::new(),
            account_email: String::new(),
            device_id: None,
            device_name: default_device_name(),
        }
    }
}

fn default_device_name() -> String {
    hostname_or_default()
}

fn hostname_or_default() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "My Computer".into())
}

fn default_max_jobs_per_source() -> u32 {
    10
}
fn default_max_applications_per_run() -> u32 {
    10
}
fn default_recency_days() -> u32 {
    7
}
fn default_sources() -> Vec<JobSourceConfig> {
    ["LinkedIn", "Naukri", "Indeed", "Wellfound"]
        .iter()
        .enumerate()
        .map(|(i, p)| JobSourceConfig {
            platform: p.to_string(),
            enabled: i < 2,
        })
        .collect()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            job_sources: default_sources(),
            max_jobs_per_source: default_max_jobs_per_source(),
            max_applications_per_run: default_max_applications_per_run(),
            approval_mode: ApprovalMode::ReviewRequired,
            recency_days: default_recency_days(),
            resume: ResumeSettings::default(),
            browser: BrowserSettings::default(),
            claude: ClaudeSettings::default(),
            mock_mode: false,
            setup_completed: false,
            minimum_match_score: 60,
            remote: RemoteSettings::default(),
            backend: BackendSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalMode {
    #[default]
    ReviewRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobSourceConfig {
    pub platform: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeSettings {
    #[serde(default = "default_true")]
    pub generate_cover_letter: bool,
    #[serde(default = "default_ats_iterations")]
    pub ats_max_iterations: u8,
    #[serde(default = "default_min_coverage")]
    pub minimum_keyword_coverage: u8,
    #[serde(default = "default_true")]
    pub generate_pdf: bool,
    #[serde(default = "default_true")]
    pub generate_docx: bool,
}

fn default_true() -> bool {
    true
}
fn default_ats_iterations() -> u8 {
    3
}
fn default_min_coverage() -> u8 {
    70
}

impl Default for ResumeSettings {
    fn default() -> Self {
        Self {
            generate_cover_letter: true,
            ats_max_iterations: 3,
            minimum_keyword_coverage: 70,
            generate_pdf: true,
            generate_docx: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSettings {
    /// Optional explicit path to the Chrome executable.
    #[serde(default)]
    pub chrome_path: Option<String>,
    #[serde(default)]
    pub keep_tabs_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSettings {
    /// Optional explicit path to the Claude CLI executable.
    #[serde(default)]
    pub cli_path: Option<String>,
    /// Model alias passed to `claude --model` (empty = CLI default).
    #[serde(default)]
    pub model: Option<String>,
    /// Passed to `--max-budget-usd` per Claude invocation. 0 disables.
    #[serde(default = "default_budget")]
    pub max_budget_usd_per_call: f64,
    #[serde(default = "default_max_turns")]
    pub max_turns_browser: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_budget() -> f64 {
    3.0
}
fn default_max_turns() -> u32 {
    80
}
fn default_timeout() -> u64 {
    900
}

impl Default for ClaudeSettings {
    fn default() -> Self {
        Self {
            cli_path: None,
            model: None,
            max_budget_usd_per_call: default_budget(),
            max_turns_browser: default_max_turns(),
            timeout_seconds: default_timeout(),
        }
    }
}

impl AppSettings {
    pub fn load(path: &Path) -> CoreResult<Self> {
        if !path.exists() {
            let mut s = Self::default();
            s.apply_env();
            return Ok(s);
        }
        let raw = std::fs::read_to_string(path)?;
        let mut s: AppSettings = serde_json::from_str(&raw).unwrap_or_default();
        s.apply_env();
        Ok(s)
    }

    pub fn save(&self, path: &Path) -> CoreResult<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Environment variables override persisted settings (useful in dev).
    fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("MOCK_MODE") {
            self.mock_mode = matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(v) = std::env::var("CLAUDE_CLI_PATH") {
            if !v.trim().is_empty() {
                self.claude.cli_path = Some(v);
            }
        }
        if let Ok(v) = std::env::var("CHROME_PATH") {
            if !v.trim().is_empty() {
                self.browser.chrome_path = Some(v);
            }
        }
    }

    pub fn enabled_sources(&self) -> Vec<String> {
        self.job_sources
            .iter()
            .filter(|s| s.enabled)
            .map(|s| s.platform.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_conservative() {
        let s = AppSettings::default();
        assert_eq!(s.approval_mode, ApprovalMode::ReviewRequired);
        assert!(!s.mock_mode);
        assert_eq!(s.resume.ats_max_iterations, 3);
        assert!(s.enabled_sources().contains(&"LinkedIn".to_string()));
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = AppSettings {
            max_jobs_per_source: 3,
            ..Default::default()
        };
        s.save(&path).unwrap();
        let loaded = AppSettings::load(&path).unwrap();
        assert_eq!(loaded.max_jobs_per_source, 3);
    }

    #[test]
    fn unknown_fields_and_missing_fields_are_tolerated() {
        let s: AppSettings =
            serde_json::from_str(r#"{"maxJobsPerSource": 4, "future": 1}"#).unwrap();
        assert_eq!(s.max_jobs_per_source, 4);
        assert_eq!(s.recency_days, 7);
    }
}
