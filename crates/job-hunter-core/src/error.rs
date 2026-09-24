use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Every failure the core can surface. Each variant carries a message that is
/// safe to show to the user; developer details go in `details` and are only
/// shown when the user opens the "developer details" disclosure in the UI.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Claude CLI is not installed or could not be found")]
    ClaudeCliUnavailable { details: String },
    #[error("Claude CLI is installed but not authenticated")]
    ClaudeNotAuthenticated { details: String },
    #[error("Claude CLI failed: {message}")]
    ClaudeRunFailed { message: String, details: String },
    #[error("Claude in Chrome is not available")]
    ClaudeInChromeUnavailable { details: String },
    #[error("Google Chrome could not be found")]
    ChromeUnavailable { details: String },
    #[error("Backend is unavailable: {message}")]
    BackendUnavailable { message: String },
    #[error("Backend account is not configured")]
    BackendNotConfigured,
    #[error("Network failure: {0}")]
    Network(String),
    #[error("Document generation failed: {0}")]
    DocumentGeneration(String),
    #[error("Document import failed: {0}")]
    DocumentImport(String),
    #[error("Invalid input: {0}")]
    Validation(String),
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),
    #[error("Agent is busy: {0}")]
    AgentBusy(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Secret store error: {0}")]
    Secrets(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

/// Serializable form of an error, sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFacingError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub recoverable: bool,
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::ClaudeCliUnavailable { .. } => "CLAUDE_CLI_UNAVAILABLE",
            CoreError::ClaudeNotAuthenticated { .. } => "CLAUDE_NOT_AUTHENTICATED",
            CoreError::ClaudeRunFailed { .. } => "CLAUDE_RUN_FAILED",
            CoreError::ClaudeInChromeUnavailable { .. } => "CLAUDE_IN_CHROME_UNAVAILABLE",
            CoreError::ChromeUnavailable { .. } => "CHROME_UNAVAILABLE",
            CoreError::BackendUnavailable { .. } => "BACKEND_UNAVAILABLE",
            CoreError::BackendNotConfigured => "BACKEND_NOT_CONFIGURED",
            CoreError::Network(_) => "NETWORK_FAILURE",
            CoreError::DocumentGeneration(_) => "DOCUMENT_GENERATION_FAILED",
            CoreError::DocumentImport(_) => "DOCUMENT_IMPORT_FAILED",
            CoreError::Validation(_) => "VALIDATION",
            CoreError::NotFound { .. } => "NOT_FOUND",
            CoreError::InvalidTransition(_) => "INVALID_TRANSITION",
            CoreError::AgentBusy(_) => "AGENT_BUSY",
            CoreError::Cancelled => "CANCELLED",
            CoreError::Secrets(_) => "SECRET_STORE",
            CoreError::Io(_) => "IO",
            CoreError::Serde(_) => "SERIALIZATION",
            CoreError::Other(_) => "OTHER",
        }
    }

    pub fn details(&self) -> Option<String> {
        match self {
            CoreError::ClaudeCliUnavailable { details }
            | CoreError::ClaudeNotAuthenticated { details }
            | CoreError::ClaudeRunFailed { details, .. }
            | CoreError::ClaudeInChromeUnavailable { details }
            | CoreError::ChromeUnavailable { details } => Some(details.clone()),
            CoreError::Io(e) => Some(e.to_string()),
            CoreError::Serde(e) => Some(e.to_string()),
            _ => None,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            CoreError::ClaudeCliUnavailable { .. } => {
                "Claude CLI is not installed or could not be found. Install Claude Code and make sure the claude command is on your PATH, or set its path in Settings.".into()
            }
            CoreError::ClaudeNotAuthenticated { .. } => {
                "Claude CLI is installed but not signed in. Run claude in a terminal and complete the login, then try again.".into()
            }
            CoreError::ClaudeInChromeUnavailable { .. } => {
                "Claude in Chrome is not available. Install the Claude in Chrome extension, open Chrome, and make sure the extension is enabled.".into()
            }
            CoreError::ChromeUnavailable { .. } => {
                "Google Chrome could not be found. Install Chrome or set its path in Settings.".into()
            }
            CoreError::BackendNotConfigured => {
                "Not signed in to a Job Hunter backend account. Sign in from Settings. Data is kept locally until then.".into()
            }
            CoreError::BackendUnavailable { message } => {
                format!("The backend is unreachable ({message}). Your data is safe locally and will sync when the connection returns.")
            }
            other => other.to_string(),
        }
    }

    pub fn to_user_facing(&self) -> UserFacingError {
        UserFacingError {
            code: self.code().to_string(),
            message: self.user_message(),
            details: self.details(),
            recoverable: !matches!(self, CoreError::Io(_) | CoreError::Serde(_)),
        }
    }

    pub fn other(msg: impl Into<String>) -> Self {
        CoreError::Other(msg.into())
    }
}

impl From<anyhow::Error> for CoreError {
    fn from(e: anyhow::Error) -> Self {
        CoreError::Other(format!("{e:#}"))
    }
}

impl From<CoreError> for UserFacingError {
    fn from(e: CoreError) -> Self {
        e.to_user_facing()
    }
}
