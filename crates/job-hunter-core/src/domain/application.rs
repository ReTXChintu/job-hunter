use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::error::{CoreError, CoreResult};
use crate::util::{new_id, now};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplicationStatus {
    Discovered,
    Analyzed,
    Shortlisted,
    ReadyForReview,
    Approved,
    Applying,
    Applied,
    ManualActionRequired,
    WaitingForUser,
    Rejected,
    Interview,
    Offer,
    Withdrawn,
}

impl ApplicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationStatus::Discovered => "DISCOVERED",
            ApplicationStatus::Analyzed => "ANALYZED",
            ApplicationStatus::Shortlisted => "SHORTLISTED",
            ApplicationStatus::ReadyForReview => "READY_FOR_REVIEW",
            ApplicationStatus::Approved => "APPROVED",
            ApplicationStatus::Applying => "APPLYING",
            ApplicationStatus::Applied => "APPLIED",
            ApplicationStatus::ManualActionRequired => "MANUAL_ACTION_REQUIRED",
            ApplicationStatus::WaitingForUser => "WAITING_FOR_USER",
            ApplicationStatus::Rejected => "REJECTED",
            ApplicationStatus::Interview => "INTERVIEW",
            ApplicationStatus::Offer => "OFFER",
            ApplicationStatus::Withdrawn => "WITHDRAWN",
        }
    }

    /// Allowed transitions. Approval is the only way into `Approved`, and only
    /// `Approved` (or a user-resumed `WaitingForUser`) may enter `Applying`.
    pub fn can_transition_to(self, next: ApplicationStatus) -> bool {
        use ApplicationStatus as S;
        if self == next {
            return true;
        }
        match self {
            S::Discovered => matches!(next, S::Analyzed | S::Rejected),
            S::Analyzed => matches!(next, S::Shortlisted | S::Rejected | S::ReadyForReview),
            S::Shortlisted => matches!(next, S::ReadyForReview | S::Rejected),
            S::ReadyForReview => {
                matches!(next, S::Approved | S::Rejected | S::ManualActionRequired)
            }
            S::Approved => matches!(
                next,
                S::Applying | S::Rejected | S::ManualActionRequired | S::Applied | S::Withdrawn
            ),
            S::Applying => matches!(
                next,
                S::Applied
                    | S::ManualActionRequired
                    | S::WaitingForUser
                    | S::Rejected
                    | S::Withdrawn
            ),
            S::WaitingForUser => matches!(
                next,
                S::Applying | S::ManualActionRequired | S::Rejected | S::Applied | S::Withdrawn
            ),
            S::ManualActionRequired => {
                matches!(next, S::Applied | S::Rejected | S::Withdrawn | S::Applying)
            }
            S::Applied => matches!(next, S::Interview | S::Rejected | S::Withdrawn | S::Offer),
            S::Interview => matches!(next, S::Offer | S::Rejected | S::Withdrawn),
            S::Offer => matches!(next, S::Withdrawn | S::Rejected),
            S::Rejected => matches!(next, S::ReadyForReview),
            S::Withdrawn => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnswerSource {
    /// From the reusable answer database.
    Known,
    /// Typed by the user during this application.
    User,
    /// Derived from the candidate profile (name, email, etc.).
    Profile,
    /// Left blank on purpose.
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationAnswer {
    pub question: String,
    pub answer: String,
    pub source: AnswerSource,
}

/// A question the browser workflow could not answer from known data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingQuestion {
    #[serde(default = "new_id")]
    pub id: String,
    pub question: String,
    /// "text" | "textarea" | "select" | "radio" | "checkbox" | "number" | "date" | "file" | "unknown"
    #[serde(default)]
    pub field_type: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusChange {
    pub status: ApplicationStatus,
    pub at: DateTime<Utc>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: String,
    pub user_id: String,
    pub job_id: String,
    pub status: ApplicationStatus,
    #[serde(default)]
    pub resume_id: Option<String>,
    #[serde(default)]
    pub cover_letter_id: Option<String>,
    #[serde(default)]
    pub application_url: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub answers: Vec<ApplicationAnswer>,
    #[serde(default)]
    pub pending_questions: Vec<PendingQuestion>,
    #[serde(default)]
    pub status_history: Vec<StatusChange>,
    #[serde(default)]
    pub potential_issues: Vec<String>,
    #[serde(default)]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub applied_at: Option<DateTime<Utc>>,
    /// Plain-language reason when the automatic application could not finish.
    #[serde(default)]
    pub failure_reason: Option<String>,
    /// What Claude saw that proves (or disproves) submission.
    #[serde(default)]
    pub evidence: Option<String>,
    #[serde(default)]
    pub claude_session_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub manual_completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Application {
    pub fn new(user_id: &str, job_id: &str, url: &str, source: &str) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.into(),
            job_id: job_id.into(),
            status: ApplicationStatus::Discovered,
            resume_id: None,
            cover_letter_id: None,
            application_url: url.into(),
            source: source.into(),
            notes: String::new(),
            answers: vec![],
            pending_questions: vec![],
            status_history: vec![StatusChange {
                status: ApplicationStatus::Discovered,
                at: ts,
                reason: "created".into(),
            }],
            potential_issues: vec![],
            approved_at: None,
            applied_at: None,
            failure_reason: None,
            evidence: None,
            claude_session_id: None,
            run_id: None,
            manual_completed: false,
            created_at: ts,
            updated_at: ts,
        }
    }

    pub fn transition(
        &mut self,
        next: ApplicationStatus,
        reason: impl Into<String>,
    ) -> CoreResult<()> {
        if !self.status.can_transition_to(next) {
            return Err(CoreError::InvalidTransition(format!(
                "application {} cannot go from {} to {}",
                self.id,
                self.status.as_str(),
                next.as_str()
            )));
        }
        if self.status != next {
            let ts = now();
            self.status = next;
            self.status_history.push(StatusChange {
                status: next,
                at: ts,
                reason: reason.into(),
            });
            if next == ApplicationStatus::Approved {
                self.approved_at = Some(ts);
            }
            if next == ApplicationStatus::Applied {
                self.applied_at = Some(ts);
            }
            self.updated_at = ts;
        }
        Ok(())
    }

    pub fn is_approved(&self) -> bool {
        self.approved_at.is_some()
            && matches!(
                self.status,
                ApplicationStatus::Approved
                    | ApplicationStatus::Applying
                    | ApplicationStatus::WaitingForUser
                    | ApplicationStatus::Applied
                    | ApplicationStatus::ManualActionRequired
            )
    }
}

impl Entity for Application {
    const COLLECTION: &'static str = "applications";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Outcome reported by the browser application workflow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplyOutcome {
    Submitted,
    HumanInputRequired,
    ManualActionRequired,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub outcome: ApplyOutcome,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub application_url: String,
    #[serde(default)]
    pub answers_used: Vec<ApplicationAnswer>,
    #[serde(default)]
    pub unknown_questions: Vec<PendingQuestion>,
    #[serde(default)]
    pub steps_completed: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_is_required_before_applying() {
        let mut app = Application::new("u", "j", "https://example.com", "LinkedIn");
        app.transition(ApplicationStatus::Analyzed, "").unwrap();
        app.transition(ApplicationStatus::ReadyForReview, "")
            .unwrap();
        assert!(app.transition(ApplicationStatus::Applying, "").is_err());
        assert!(!app.is_approved());
        app.transition(ApplicationStatus::Approved, "user").unwrap();
        assert!(app.is_approved());
        app.transition(ApplicationStatus::Applying, "").unwrap();
        app.transition(ApplicationStatus::WaitingForUser, "question")
            .unwrap();
        app.transition(ApplicationStatus::Applying, "answered")
            .unwrap();
        app.transition(ApplicationStatus::Applied, "submitted")
            .unwrap();
        assert!(app.applied_at.is_some());
        assert_eq!(app.status_history.len(), 8);
    }

    #[test]
    fn rejected_can_be_reopened_only_to_review() {
        let mut app = Application::new("u", "j", "u", "s");
        app.transition(ApplicationStatus::Rejected, "no").unwrap();
        assert!(app.transition(ApplicationStatus::Applied, "").is_err());
        app.transition(ApplicationStatus::ReadyForReview, "reopen")
            .unwrap();
    }

    #[test]
    fn manual_action_can_be_marked_applied() {
        let mut app = Application::new("u", "j", "u", "s");
        app.transition(ApplicationStatus::Analyzed, "").unwrap();
        app.transition(ApplicationStatus::ReadyForReview, "")
            .unwrap();
        app.transition(ApplicationStatus::Approved, "").unwrap();
        app.transition(ApplicationStatus::Applying, "").unwrap();
        app.transition(ApplicationStatus::ManualActionRequired, "captcha")
            .unwrap();
        app.transition(ApplicationStatus::Applied, "user marked")
            .unwrap();
    }
}
