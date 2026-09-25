//! "Is there a newer desktop build?" -- answered by the same server the app
//! syncs with (`GET /v1/downloads`, see `apps/backend/src/routes/static.ts`).
//! Installing it is the Tauri shell's job (`tauri-plugin-updater`); this
//! module only discovers and compares versions, so it stays testable
//! without a window.

use std::cmp::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// One platform's latest build, as the server reports it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerBuild {
    pub file_name: String,
    pub version: Option<String>,
    pub build: Option<u64>,
    pub size_bytes: u64,
    pub updated_at: String,
    pub url: String,
    #[serde(default)]
    pub signed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ServerBuilds {
    pub android: Option<ServerBuild>,
    pub windows: Option<ServerBuild>,
}

/// Result of an update check, shaped for the desktop UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub available: bool,
    /// Absolute URL of the installer, for a manual download.
    pub download_url: Option<String>,
    /// True when the installer is signed and this build carries the
    /// updater's public key, so it can be downloaded, verified and
    /// installed from inside the app.
    pub can_install_in_app: bool,
    pub size_bytes: Option<u64>,
    pub published_at: Option<String>,
}

fn parse(v: &str) -> Option<(Vec<u64>, Option<&str>)> {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let nums: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    let nums = nums?;
    (nums.len() == 3).then_some((nums, pre))
}

/// Semver ordering of `a` versus `b`. A prerelease sorts before its
/// release (`1.0.0-rc.1 < 1.0.0`). Unparseable versions compare as equal,
/// so garbage on the server never triggers an "update".
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (Some((an, ap)), Some((bn, bp))) = (parse(a), parse(b)) else {
        return Ordering::Equal;
    };
    an.cmp(&bn).then_with(|| match (ap, bp) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => compare_prerelease(x, y),
    })
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    for (x, y) in a.split('.').zip(b.split('.')) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a.split('.').count().cmp(&b.split('.').count())
}

/// Turn the server's build list into an answer for this desktop.
pub fn evaluate(
    server_url: &str,
    current_version: &str,
    builds: &ServerBuilds,
    updater_enabled: bool,
) -> UpdateCheck {
    let windows = builds.windows.as_ref();
    let latest = windows.and_then(|w| w.version.clone());
    let available = latest
        .as_deref()
        .is_some_and(|l| compare_versions(l, current_version) == Ordering::Greater);
    UpdateCheck {
        current_version: current_version.to_string(),
        latest_version: latest,
        available,
        download_url: windows.map(|w| format!("{}{}", server_url.trim_end_matches('/'), w.url)),
        can_install_in_app: available && updater_enabled && windows.is_some_and(|w| w.signed),
        size_bytes: windows.map(|w| w.size_bytes),
        published_at: windows.map(|w| w.updated_at.clone()),
    }
}

pub async fn fetch_builds(server_url: &str) -> CoreResult<ServerBuilds> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let resp = http
        .get(format!("{}/v1/downloads", server_url.trim_end_matches('/')))
        .send()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(CoreError::BackendUnavailable {
            message: format!("server returned {}", resp.status()),
        });
    }
    resp.json()
        .await
        .map_err(|e| CoreError::Other(format!("unexpected update info from the server: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_like_semver() {
        assert_eq!(compare_versions("0.2.0", "0.1.9"), Ordering::Greater);
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("v1.2.3", "1.2.3"), Ordering::Equal);
        assert_eq!(compare_versions("1.0.0-rc.1", "1.0.0"), Ordering::Less);
        assert_eq!(
            compare_versions("1.0.0-rc.2", "1.0.0-rc.10"),
            Ordering::Less
        );
        assert_eq!(
            compare_versions("1.0.0-alpha", "1.0.0-beta"),
            Ordering::Less
        );
        assert_eq!(compare_versions("garbage", "1.0.0"), Ordering::Equal);
    }

    fn windows(version: &str, signed: bool) -> ServerBuilds {
        ServerBuilds {
            android: None,
            windows: Some(ServerBuild {
                file_name: format!("Job Hunter_{version}_x64-setup.exe"),
                version: Some(version.into()),
                build: None,
                size_bytes: 5,
                updated_at: "2026-09-25T00:00:00Z".into(),
                url: "/downloads/windows".into(),
                signed,
            }),
        }
    }

    #[test]
    fn a_newer_signed_build_can_install_in_app_only_with_the_updater_key() {
        let c = evaluate(
            "http://10.0.0.5:8788/",
            "0.1.0",
            &windows("0.2.0", true),
            true,
        );
        assert!(c.available && c.can_install_in_app);
        assert_eq!(
            c.download_url.as_deref(),
            Some("http://10.0.0.5:8788/downloads/windows")
        );
        assert!(!evaluate("http://s", "0.1.0", &windows("0.2.0", true), false).can_install_in_app);
        assert!(!evaluate("http://s", "0.1.0", &windows("0.2.0", false), true).can_install_in_app);
    }

    #[test]
    fn same_older_or_missing_builds_are_not_updates() {
        assert!(!evaluate("http://s", "0.2.0", &windows("0.2.0", true), true).available);
        assert!(!evaluate("http://s", "0.3.0", &windows("0.2.0", true), true).available);
        let none = evaluate("http://s", "0.1.0", &ServerBuilds::default(), true);
        assert!(!none.available && none.latest_version.is_none() && none.download_url.is_none());
    }
}
