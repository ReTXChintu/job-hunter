use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::util::{new_id, now};

/// Lifecycle of a job as seen by the user. Mirrors application tracking
/// statuses so the Jobs page can filter on one field.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Discovered,
    Analyzed,
    NotRelevant,
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

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Discovered => "DISCOVERED",
            JobStatus::Analyzed => "ANALYZED",
            JobStatus::NotRelevant => "NOT_RELEVANT",
            JobStatus::Shortlisted => "SHORTLISTED",
            JobStatus::ReadyForReview => "READY_FOR_REVIEW",
            JobStatus::Approved => "APPROVED",
            JobStatus::Applying => "APPLYING",
            JobStatus::Applied => "APPLIED",
            JobStatus::ManualActionRequired => "MANUAL_ACTION_REQUIRED",
            JobStatus::WaitingForUser => "WAITING_FOR_USER",
            JobStatus::Rejected => "REJECTED",
            JobStatus::Interview => "INTERVIEW",
            JobStatus::Offer => "OFFER",
            JobStatus::Withdrawn => "WITHDRAWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobSource {
    pub platform: String,
    pub url: String,
    #[serde(default)]
    pub source_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub user_id: String,
    pub discovered_at: DateTime<Utc>,
    /// Best-effort posting date as shown on the source (ISO date or relative
    /// text such as "2 days ago" normalised by the extractor).
    #[serde(default)]
    pub posted_at: Option<String>,
    pub source: String,
    #[serde(default)]
    pub source_job_id: Option<String>,
    pub url: String,
    #[serde(default)]
    pub canonical_url: Option<String>,
    pub company: String,
    pub title: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub employment_type: Option<String>,
    #[serde(default)]
    pub remote: Option<String>,
    #[serde(default)]
    pub salary: Option<String>,
    #[serde(default)]
    pub seniority: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub sources: Vec<JobSource>,
    pub status: JobStatus,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub analysis_id: Option<String>,
    #[serde(default)]
    pub application_id: Option<String>,
    #[serde(default)]
    pub saved: bool,
    /// True when the description was fully extracted from the job page.
    #[serde(default)]
    pub details_complete: bool,
    #[serde(default)]
    pub dedup_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Job {
    #[allow(clippy::too_many_arguments)]
    pub fn new(user_id: &str, source: &str, url: &str, company: &str, title: &str) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.to_string(),
            discovered_at: ts,
            posted_at: None,
            source: source.to_string(),
            source_job_id: None,
            url: url.to_string(),
            canonical_url: None,
            company: company.to_string(),
            title: title.to_string(),
            location: String::new(),
            employment_type: None,
            remote: None,
            salary: None,
            seniority: None,
            description: String::new(),
            requirements: vec![],
            responsibilities: vec![],
            skills: vec![],
            sources: vec![JobSource {
                platform: source.to_string(),
                url: url.to_string(),
                source_job_id: None,
            }],
            status: JobStatus::Discovered,
            run_id: None,
            analysis_id: None,
            application_id: None,
            saved: false,
            details_complete: false,
            dedup_key: String::new(),
            created_at: ts,
            updated_at: ts,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now();
    }
}

impl Entity for Job {
    const COLLECTION: &'static str = "jobs";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Raw job as extracted by Claude from a job site, before it becomes a `Job`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredJob {
    pub title: String,
    pub company: String,
    pub url: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub posted_at: Option<String>,
    #[serde(default)]
    pub source_job_id: Option<String>,
    #[serde(default)]
    pub employment_type: Option<String>,
    #[serde(default)]
    pub remote: Option<String>,
    #[serde(default)]
    pub salary: Option<String>,
    #[serde(default)]
    pub seniority: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub details_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobAnalysis {
    pub id: String,
    pub job_id: String,
    pub user_id: String,
    pub relevant: bool,
    /// 0-100, for sorting and explanation only. Never used to auto-apply.
    pub match_score: u8,
    #[serde(default)]
    pub matched_skills: Vec<String>,
    #[serde(default)]
    pub missing_skills: Vec<String>,
    #[serde(default)]
    pub required_experience_met: bool,
    #[serde(default)]
    pub seniority_match: bool,
    #[serde(default)]
    pub location_match: bool,
    #[serde(default)]
    pub employment_type_match: bool,
    #[serde(default)]
    pub salary_assessment: String,
    #[serde(default)]
    pub required_qualifications: Vec<String>,
    #[serde(default)]
    pub nice_to_have: Vec<String>,
    #[serde(default)]
    pub concerns: Vec<String>,
    #[serde(default)]
    pub important_keywords: Vec<String>,
    /// Why this job matches (or does not), in plain language.
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for JobAnalysis {
    const COLLECTION: &'static str = "job_analyses";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Shape Claude returns for a single job analysis (without ids/timestamps).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub relevant: bool,
    pub match_score: u8,
    #[serde(default)]
    pub matched_skills: Vec<String>,
    #[serde(default)]
    pub missing_skills: Vec<String>,
    #[serde(default)]
    pub required_experience_met: bool,
    #[serde(default)]
    pub seniority_match: bool,
    #[serde(default)]
    pub location_match: bool,
    #[serde(default)]
    pub employment_type_match: bool,
    #[serde(default)]
    pub salary_assessment: String,
    #[serde(default)]
    pub required_qualifications: Vec<String>,
    #[serde(default)]
    pub nice_to_have: Vec<String>,
    #[serde(default)]
    pub concerns: Vec<String>,
    #[serde(default)]
    pub important_keywords: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

impl JobAnalysis {
    pub fn from_result(job: &Job, r: AnalysisResult, run_id: Option<String>) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            job_id: job.id.clone(),
            user_id: job.user_id.clone(),
            relevant: r.relevant,
            match_score: r.match_score.min(100),
            matched_skills: r.matched_skills,
            missing_skills: r.missing_skills,
            required_experience_met: r.required_experience_met,
            seniority_match: r.seniority_match,
            location_match: r.location_match,
            employment_type_match: r.employment_type_match,
            salary_assessment: r.salary_assessment,
            required_qualifications: r.required_qualifications,
            nice_to_have: r.nice_to_have,
            concerns: r.concerns,
            important_keywords: r.important_keywords,
            summary: r.summary,
            run_id,
            created_at: ts,
            updated_at: ts,
        }
    }
}
