//! Work the user asked for while the agent was busy. It runs one task at a
//! time, so later requests wait here and start by themselves, in order, as
//! soon as the agent is free (idle, or stopped for the user's input).
//!
//! Local-only (`Entity::SYNCED = false`): the queue belongs to this
//! computer's agent; phones and the web see it through the desktop.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::util::{new_id, now};

/// One unit of agent work, exactly as the start commands take it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentTask {
    #[serde(rename_all = "camelCase")]
    JobHunt {
        #[serde(default)]
        discover_only: bool,
        #[serde(default)]
        prepare_only: bool,
        #[serde(default)]
        sources: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Application {
        application_id: String,
        /// Continue the application's Claude session (after the user's
        /// answers) instead of starting it over.
        #[serde(default)]
        resume: bool,
        /// Honoured by the mock runner only.
        #[serde(default)]
        simulate: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    AnalyzeJob {
        job_id: String,
    },
    #[serde(rename_all = "camelCase")]
    GenerateResume {
        job_id: String,
    },
    #[serde(rename_all = "camelCase")]
    ProfileSync {
        platform: String,
        #[serde(default)]
        resume: bool,
        /// Answers the user just gave to the site's questions.
        #[serde(default)]
        answers: Vec<super::ApplicationAnswer>,
    },
    #[serde(rename_all = "camelCase")]
    JobLink {
        url: String,
        #[serde(default)]
        apply_now: bool,
    },
    #[serde(rename_all = "camelCase")]
    SharedJob {
        text: String,
        #[serde(default)]
        files: Vec<String>,
        #[serde(default)]
        draft_email: bool,
    },
    InboxCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueuedTask {
    pub id: String,
    pub user_id: String,
    pub task: AgentTask,
    /// What the Agent page shows for it, e.g. "Apply: Developer at Acme".
    pub title: String,
    pub requested_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl QueuedTask {
    pub fn new(user_id: &str, task: AgentTask, title: impl Into<String>) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.into(),
            task,
            title: title.into(),
            requested_at: ts,
            updated_at: ts,
        }
    }
}

impl Entity for QueuedTask {
    const COLLECTION: &'static str = "agent_queue";
    const SYNCED: bool = false;
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// How a start request ended: running now, or waiting its turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    #[serde(default)]
    pub run: Option<super::AgentRun>,
    #[serde(default)]
    pub queued: Option<QueuedTask>,
    /// 1-based place in the queue when queued.
    #[serde(default)]
    pub position: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_serialise_with_a_kind_tag_and_camel_case_fields() {
        let task = AgentTask::Application {
            application_id: "a1".into(),
            resume: false,
            simulate: None,
        };
        let v = serde_json::to_value(&task).unwrap();
        assert_eq!(v["kind"], "APPLICATION");
        assert_eq!(v["applicationId"], "a1");
        let back: AgentTask = serde_json::from_value(v).unwrap();
        assert_eq!(back, task);
        let hunt: AgentTask =
            serde_json::from_value(serde_json::json!({ "kind": "JOB_HUNT" })).unwrap();
        assert_eq!(
            hunt,
            AgentTask::JobHunt {
                discover_only: false,
                prepare_only: false,
                sources: vec![]
            }
        );
    }
}
