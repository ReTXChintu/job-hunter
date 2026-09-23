use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::util::{new_id, now};

/// Structured, ATS-friendly resume content. Claude produces this; Rust renders
/// it to DOCX / HTML / PDF so formatting is deterministic and clean.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeDocument {
    #[serde(default)]
    pub contact: ResumeContact,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub skills: Vec<ResumeSkillSection>,
    #[serde(default)]
    pub experience: Vec<ResumeExperience>,
    #[serde(default)]
    pub projects: Vec<ResumeProject>,
    #[serde(default)]
    pub education: Vec<ResumeEducation>,
    #[serde(default)]
    pub certifications: Vec<String>,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeContact {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub linkedin: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub portfolio: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeSkillSection {
    pub name: String,
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeExperience {
    pub company: String,
    pub role: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeProject {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub technologies: Vec<String>,
    #[serde(default)]
    pub bullets: Vec<String>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeEducation {
    pub institution: String,
    #[serde(default)]
    pub degree: String,
    #[serde(default)]
    pub field: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub grade: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AtsStatus {
    Pass,
    Revise,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AtsValidation {
    pub status: AtsStatus,
    pub keyword_coverage: u8,
    #[serde(default)]
    pub missing_keywords: Vec<String>,
    #[serde(default)]
    pub unsupported_claims: Vec<String>,
    #[serde(default)]
    pub missing_requirements: Vec<String>,
    #[serde(default)]
    pub formatting_issues: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub iteration: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResumeKind {
    Master,
    Generated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resume {
    pub id: String,
    pub user_id: String,
    pub kind: ResumeKind,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub job_title: String,
    pub version: u32,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub docx_path: Option<String>,
    #[serde(default)]
    pub pdf_path: Option<String>,
    #[serde(default)]
    pub html_path: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
    #[serde(default)]
    pub content: Option<ResumeDocument>,
    #[serde(default)]
    pub validation: Option<AtsValidation>,
    #[serde(default)]
    pub run_id: Option<String>,
    /// Set when the user edited the generated content by hand.
    #[serde(default)]
    pub user_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Resume {
    pub fn new_generated(
        user_id: &str,
        job_id: &str,
        company: &str,
        job_title: &str,
        version: u32,
    ) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.into(),
            kind: ResumeKind::Generated,
            job_id: Some(job_id.into()),
            company: company.into(),
            job_title: job_title.into(),
            version,
            label: format!("{company} - {job_title} v{version}"),
            docx_path: None,
            pdf_path: None,
            html_path: None,
            json_path: None,
            content: None,
            validation: None,
            run_id: None,
            user_edited: false,
            created_at: ts,
            updated_at: ts,
        }
    }
}

impl Entity for Resume {
    const COLLECTION: &'static str = "resumes";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoverLetter {
    pub id: String,
    pub user_id: String,
    pub job_id: String,
    #[serde(default)]
    pub resume_id: Option<String>,
    pub version: u32,
    pub text: String,
    #[serde(default)]
    pub docx_path: Option<String>,
    #[serde(default)]
    pub pdf_path: Option<String>,
    #[serde(default)]
    pub txt_path: Option<String>,
    #[serde(default)]
    pub user_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for CoverLetter {
    const COLLECTION: &'static str = "cover_letters";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
