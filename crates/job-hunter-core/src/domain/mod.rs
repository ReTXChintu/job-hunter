//! Domain model. Every struct here is serialised with camelCase field names so
//! the same JSON shape is used on disk, in MongoDB and in the React frontend
//! (mirrored by `packages/types`).

pub mod agent;
pub mod answers;
pub mod application;
pub mod candidate;
pub mod job;
pub mod notification;
pub mod platform;
pub mod resume;

pub use agent::*;
pub use answers::*;
pub use application::*;
pub use candidate::*;
pub use job::*;
pub use notification::*;
pub use platform::*;
pub use resume::*;

/// Documents that can live in a store collection.
pub trait Entity:
    serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static
{
    const COLLECTION: &'static str;
    /// Whether writes are queued for the backend. Local-only collections are
    /// listed in [`LOCAL_COLLECTIONS`] instead of [`COLLECTIONS`].
    const SYNCED: bool = true;
    fn id(&self) -> &str;
    fn updated_at(&self) -> chrono::DateTime<chrono::Utc>;
}

/// Collections kept on this computer only (never sent to the backend).
pub const LOCAL_COLLECTIONS: &[&str] = &["platform_profiles", "publishing_plans", "scan_cursors"];

/// Names of every persisted collection (local store and MongoDB share them).
pub const COLLECTIONS: &[&str] = &[
    "users",
    "candidate_profiles",
    "experiences",
    "projects",
    "jobs",
    "job_analyses",
    "resumes",
    "cover_letters",
    "applications",
    "application_answers",
    "agent_runs",
    "agent_events",
    "settings",
    "notifications",
];

/// The single local user. Schemas carry a `userId` so the data model can grow
/// to multiple users without migration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Entity for User {
    const COLLECTION: &'static str = "users";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }
}

pub const LOCAL_USER_ID: &str = "local-user";
