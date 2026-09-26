//! The candidate's own profiles on job sites (LinkedIn, Naukri, ...), which
//! Job Hunter fills in from the desktop profile so applications don't stop
//! for details the site already asks for.
//!
//! Both collections here are local-only (`Entity::SYNCED = false`): the state
//! belongs to this computer's browser sessions, and the server's data API
//! doesn't know these collections.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Entity, PendingQuestion};
use crate::util::{now, slugify};

/// Job sites whose profile Job Hunter can keep up to date.
pub const PROFILE_PLATFORMS: &[&str] = &["LinkedIn", "Naukri", "Indeed", "Wellfound"];

/// The canonical platform name for a loosely written one ("linkedin" → "LinkedIn").
pub fn canonical_platform(name: &str) -> Option<&'static str> {
    PROFILE_PLATFORMS
        .iter()
        .copied()
        .find(|p| p.eq_ignore_ascii_case(name.trim()))
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlatformSyncStatus {
    #[default]
    NeverSynced,
    Syncing,
    Synced,
    NeedsInput,
    ManualActionRequired,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformProfile {
    /// The platform's slug, e.g. "linkedin".
    pub id: String,
    pub user_id: String,
    pub platform: String,
    #[serde(default)]
    pub status: PlatformSyncStatus,
    /// Re-sync by itself when the desktop profile changes. Only acts after
    /// the user has run (and so approved) one update on this site.
    #[serde(default = "default_true")]
    pub auto_sync: bool,
    #[serde(default)]
    pub profile_url: String,
    #[serde(default)]
    pub last_synced_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_attempt_at: Option<DateTime<Utc>>,
    /// Fingerprint of the content last written successfully.
    #[serde(default)]
    pub synced_hash: Option<String>,
    /// Projects Job Hunter put on the site, so it can take down the ones the
    /// user later stops featuring (and nothing else).
    #[serde(default)]
    pub synced_project_names: Vec<String>,
    /// What the last run changed on the site.
    #[serde(default)]
    pub changes: Vec<String>,
    /// Sections the site has no place for, or that were left alone, and why.
    #[serde(default)]
    pub skipped: Vec<String>,
    /// Plain-language reason for the current status.
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub pending_questions: Vec<PendingQuestion>,
    #[serde(default)]
    pub run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl PlatformProfile {
    pub fn new(user_id: &str, platform: &str) -> Self {
        let ts = now();
        Self {
            id: slugify(platform),
            user_id: user_id.into(),
            platform: platform.into(),
            status: PlatformSyncStatus::NeverSynced,
            auto_sync: true,
            profile_url: String::new(),
            last_synced_at: None,
            last_attempt_at: None,
            synced_hash: None,
            synced_project_names: vec![],
            changes: vec![],
            skipped: vec![],
            message: String::new(),
            pending_questions: vec![],
            run_id: None,
            created_at: ts,
            updated_at: ts,
        }
    }
}

impl Entity for PlatformProfile {
    const COLLECTION: &'static str = "platform_profiles";
    const SYNCED: bool = false;
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Claude's view on one project: feature it on job sites or not, and why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPick {
    pub project_id: String,
    pub selected: bool,
    #[serde(default)]
    pub reason: String,
}

/// What the candidate chose to publish on job sites. One document, id
/// [`PublishingPlan::ID`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishingPlan {
    pub id: String,
    pub user_id: String,
    /// In the order they should appear.
    #[serde(default)]
    pub featured_project_ids: Vec<String>,
    /// Claude's suggestion the user started from, kept to show the reasons.
    #[serde(default)]
    pub picks: Vec<ProjectPick>,
    /// Set when the user clicked Proceed. No site is touched before that.
    #[serde(default)]
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PublishingPlan {
    pub const ID: &'static str = "default";

    pub fn new(user_id: &str) -> Self {
        let ts = now();
        Self {
            id: Self::ID.into(),
            user_id: user_id.into(),
            featured_project_ids: vec![],
            picks: vec![],
            confirmed_at: None,
            created_at: ts,
            updated_at: ts,
        }
    }
}

impl Entity for PublishingPlan {
    const COLLECTION: &'static str = "publishing_plans";
    const SYNCED: bool = false;
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_names_are_matched_loosely() {
        assert_eq!(canonical_platform(" linkedin "), Some("LinkedIn"));
        assert_eq!(canonical_platform("NAUKRI"), Some("Naukri"));
        assert_eq!(canonical_platform("Monster"), None);
        assert_eq!(PlatformProfile::new("u", "LinkedIn").id, "linkedin");
    }
}
