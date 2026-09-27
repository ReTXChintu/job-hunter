//! Things the user should hear about without watching the app: a job hunt
//! finished, an application or a job-site update needs them, something
//! failed. Synced to the backend so the web app and the phone show them too.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Entity, EventLevel};
use crate::util::{new_id, now};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub level: EventLevel,
    /// Machine-readable: JOB_HUNT_FINISHED, JOB_HUNT_FAILED, APPLICATION_NEEDS_INPUT,
    /// APPLICATION_MANUAL_ACTION, APPLICATION_SUBMITTED, PROFILE_UPDATED,
    /// PROFILE_NEEDS_INPUT, PROFILE_MANUAL_ACTION, PROFILE_UPDATE_FAILED.
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// Where it leads: "applications", "application" (with `link_id`),
    /// "job-sites", "agent".
    #[serde(default)]
    pub link_page: String,
    #[serde(default)]
    pub link_id: Option<String>,
    /// Read on the desktop. The web app and the phone track what they've
    /// shown themselves.
    #[serde(default)]
    pub read: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(
        user_id: &str,
        level: EventLevel,
        kind: &str,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.into(),
            level,
            kind: kind.into(),
            title: title.into(),
            body: body.into(),
            link_page: String::new(),
            link_id: None,
            read: false,
            created_at: ts,
            updated_at: ts,
        }
    }

    pub fn link(mut self, page: &str, id: Option<&str>) -> Self {
        self.link_page = page.into();
        self.link_id = id.map(String::from);
        self
    }
}

impl Entity for Notification {
    const COLLECTION: &'static str = "notifications";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
