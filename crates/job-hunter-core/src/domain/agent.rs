use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::util::{new_id, now};

/// Explicit agent state machine states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentState {
    Idle,
    Initializing,
    Discovering,
    Extracting,
    Deduplicating,
    Analyzing,
    PreparingApplications,
    WaitingForApproval,
    Applying,
    Completed,
    Failed,
    ManualActionRequired,
    WaitingForUser,
    Paused,
    Stopping,
}

impl AgentState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentState::Idle => "IDLE",
            AgentState::Initializing => "INITIALIZING",
            AgentState::Discovering => "DISCOVERING",
            AgentState::Extracting => "EXTRACTING",
            AgentState::Deduplicating => "DEDUPLICATING",
            AgentState::Analyzing => "ANALYZING",
            AgentState::PreparingApplications => "PREPARING_APPLICATIONS",
            AgentState::WaitingForApproval => "WAITING_FOR_APPROVAL",
            AgentState::Applying => "APPLYING",
            AgentState::Completed => "COMPLETED",
            AgentState::Failed => "FAILED",
            AgentState::ManualActionRequired => "MANUAL_ACTION_REQUIRED",
            AgentState::WaitingForUser => "WAITING_FOR_USER",
            AgentState::Paused => "PAUSED",
            AgentState::Stopping => "STOPPING",
        }
    }

    /// True when the agent is not doing anything and a new run may start.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentState::Idle
                | AgentState::Completed
                | AgentState::Failed
                | AgentState::WaitingForApproval
                | AgentState::ManualActionRequired
                | AgentState::WaitingForUser
        )
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            AgentState::Initializing
                | AgentState::Discovering
                | AgentState::Extracting
                | AgentState::Deduplicating
                | AgentState::Analyzing
                | AgentState::PreparingApplications
                | AgentState::Applying
                | AgentState::Stopping
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventLevel {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub id: String,
    pub run_id: String,
    pub at: DateTime<Utc>,
    pub level: EventLevel,
    /// Machine-readable kind: STATE_CHANGED, STEP_STARTED, STEP_DONE, STEP_FAILED,
    /// PROGRESS, MESSAGE, JOB_DISCOVERED, JOB_ANALYZED, APPLICATION_READY,
    /// APPLICATION_UPDATED, CLAUDE_ACTIVITY, RUN_FINISHED.
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl AgentEvent {
    pub fn new(run_id: &str, level: EventLevel, kind: &str, message: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            run_id: run_id.into(),
            at: now(),
            level,
            kind: kind.into(),
            message: message.into(),
            data: serde_json::Value::Null,
        }
    }
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

impl Entity for AgentEvent {
    const COLLECTION: &'static str = "agent_events";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.at
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub jobs_discovered: u32,
    pub jobs_new: u32,
    pub duplicates_removed: u32,
    pub jobs_analyzed: u32,
    pub relevant: u32,
    pub awaiting_approval: u32,
    pub approved: u32,
    pub applied: u32,
    pub manual_action: u32,
    pub errors: u32,
    pub claude_cost_usd: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunKind {
    JobHunt,
    Application,
    ResumeGeneration,
    Analysis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub user_id: String,
    pub kind: RunKind,
    pub state: AgentState,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub stats: RunStats,
    #[serde(default)]
    pub current_activity: Option<String>,
    #[serde(default)]
    pub progress: Option<u8>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub mock: bool,
    /// Sources this run searched.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Ids of jobs discovered in this run.
    #[serde(default)]
    pub job_ids: Vec<String>,
    /// For application runs.
    #[serde(default)]
    pub application_id: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl AgentRun {
    pub fn new(user_id: &str, kind: RunKind, mock: bool) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.into(),
            kind,
            state: AgentState::Initializing,
            started_at: ts,
            finished_at: None,
            stats: RunStats::default(),
            current_activity: None,
            progress: None,
            error: None,
            mock,
            sources: vec![],
            job_ids: vec![],
            application_id: None,
            updated_at: ts,
        }
    }
}

impl Entity for AgentRun {
    const COLLECTION: &'static str = "agent_runs";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Snapshot of the live agent, pushed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub state: AgentState,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub run_kind: Option<RunKind>,
    #[serde(default)]
    pub current_activity: Option<String>,
    #[serde(default)]
    pub progress: Option<u8>,
    #[serde(default)]
    pub stats: RunStats,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub error: Option<String>,
    pub mock: bool,
    pub paused: bool,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self {
            state: AgentState::Idle,
            run_id: None,
            run_kind: None,
            current_activity: None,
            progress: None,
            stats: RunStats::default(),
            started_at: None,
            error: None,
            mock: false,
            paused: false,
        }
    }
}
