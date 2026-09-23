use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::util::{new_id, now};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PersonalInfo {
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
    #[serde(default)]
    pub current_title: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemotePreference {
    #[default]
    Any,
    Remote,
    Hybrid,
    Onsite,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelocationPreference {
    #[default]
    NotSpecified,
    Yes,
    No,
    Maybe,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EmploymentType {
    FullTime,
    PartTime,
    Contract,
    Freelance,
    Internship,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SalaryPreference {
    #[serde(default)]
    pub minimum: Option<f64>,
    #[serde(default)]
    pub maximum: Option<f64>,
    #[serde(default = "default_currency")]
    pub currency: String,
    /// "YEAR" or "MONTH"
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_currency() -> String {
    "INR".into()
}
fn default_period() -> String {
    "YEAR".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CareerPreferences {
    #[serde(default)]
    pub target_roles: Vec<String>,
    #[serde(default)]
    pub preferred_locations: Vec<String>,
    #[serde(default)]
    pub remote_preference: RemotePreference,
    #[serde(default)]
    pub hybrid_acceptable: bool,
    #[serde(default)]
    pub relocation: RelocationPreference,
    /// Years of experience the candidate wants roles to require at minimum.
    #[serde(default)]
    pub minimum_experience_years: Option<f32>,
    #[serde(default)]
    pub salary: SalaryPreference,
    #[serde(default)]
    pub employment_types: Vec<EmploymentType>,
    #[serde(default)]
    pub notice_period: String,
    /// Free-form: industries to avoid, companies to skip, etc.
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillGroups {
    #[serde(default)]
    pub frontend: Vec<String>,
    #[serde(default)]
    pub backend: Vec<String>,
    #[serde(default)]
    pub database: Vec<String>,
    #[serde(default)]
    pub devops: Vec<String>,
    #[serde(default)]
    pub cloud: Vec<String>,
    #[serde(default)]
    pub testing: Vec<String>,
    #[serde(default)]
    pub other: Vec<String>,
}

impl SkillGroups {
    pub fn all(&self) -> Vec<String> {
        let mut v = Vec::new();
        for group in [
            &self.frontend,
            &self.backend,
            &self.database,
            &self.devops,
            &self.cloud,
            &self.testing,
            &self.other,
        ] {
            v.extend(group.iter().cloned());
        }
        v
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Education {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
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
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Certification {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResumeFormat {
    Pdf,
    Docx,
}

/// Reference to the imported master resume. The original file is copied into
/// the app data directory and never modified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MasterResumeRef {
    pub id: String,
    pub original_file_name: String,
    pub stored_path: String,
    pub text_path: String,
    pub format: ResumeFormat,
    pub imported_at: DateTime<Utc>,
    pub text_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateProfile {
    pub id: String,
    pub user_id: String,
    #[serde(default)]
    pub personal: PersonalInfo,
    #[serde(default)]
    pub preferences: CareerPreferences,
    #[serde(default)]
    pub skills: SkillGroups,
    #[serde(default)]
    pub education: Vec<Education>,
    #[serde(default)]
    pub certifications: Vec<Certification>,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub master_resume: Option<MasterResumeRef>,
    /// Free text summary written by the user (optional).
    #[serde(default)]
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CandidateProfile {
    pub fn new(user_id: &str) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.to_string(),
            personal: PersonalInfo::default(),
            preferences: CareerPreferences::default(),
            skills: SkillGroups::default(),
            education: vec![],
            certifications: vec![],
            achievements: vec![],
            languages: vec![],
            master_resume: None,
            summary: String::new(),
            created_at: ts,
            updated_at: ts,
        }
    }

    /// The profile is "usable" for a job hunt when we at least know who the
    /// candidate is and what they are looking for.
    pub fn completeness(&self) -> ProfileCompleteness {
        let mut missing = Vec::new();
        if self.personal.name.trim().is_empty() {
            missing.push("name");
        }
        if self.personal.email.trim().is_empty() {
            missing.push("email");
        }
        if self.preferences.target_roles.is_empty() {
            missing.push("targetRoles");
        }
        if self.skills.all().is_empty() {
            missing.push("skills");
        }
        ProfileCompleteness {
            ready: missing.is_empty(),
            missing: missing.into_iter().map(String::from).collect(),
            has_master_resume: self.master_resume.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCompleteness {
    pub ready: bool,
    pub missing: Vec<String>,
    pub has_master_resume: bool,
}

impl Entity for CandidateProfile {
    const COLLECTION: &'static str = "candidate_profiles";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Experience {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub location: String,
    /// "YYYY-MM"
    #[serde(default)]
    pub start_date: String,
    /// `None` means current position.
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub technologies: Vec<String>,
    #[serde(default)]
    pub achievements: Vec<String>,
    /// Ids of `Project` records done at this employer.
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub updated_at: DateTime<Utc>,
}

impl Entity for Experience {
    const COLLECTION: &'static str = "experiences";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub experience_id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub technologies: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub updated_at: DateTime<Utc>,
}

impl Entity for Project {
    const COLLECTION: &'static str = "projects";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Everything Claude is allowed to know about the candidate. This is the
/// "truth source" given to every prompt: anything not in here must not appear
/// in a generated document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateTruth {
    pub profile: CandidateProfile,
    pub experiences: Vec<Experience>,
    pub projects: Vec<Project>,
    pub master_resume_text: Option<String>,
}

impl CandidateTruth {
    pub fn total_experience_years(&self) -> f32 {
        let mut months = 0i32;
        for e in &self.experiences {
            if let Some(start) = parse_ym(&e.start_date) {
                let end = e.end_date.as_deref().and_then(parse_ym).unwrap_or_else(|| {
                    let n = now();
                    (
                        n.format("%Y").to_string().parse().unwrap_or(2000),
                        n.format("%m").to_string().parse().unwrap_or(1),
                    )
                });
                let span = (end.0 - start.0) * 12 + (end.1 - start.1);
                if span > 0 {
                    months += span;
                }
            }
        }
        months as f32 / 12.0
    }
}

fn parse_ym(s: &str) -> Option<(i32, i32)> {
    let mut parts = s.trim().split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: i32 = parts.next().unwrap_or("1").parse().ok()?;
    Some((y, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completeness_reports_missing_fields() {
        let mut p = CandidateProfile::new("u");
        let c = p.completeness();
        assert!(!c.ready);
        assert!(c.missing.contains(&"name".to_string()));
        p.personal.name = "Ada".into();
        p.personal.email = "ada@example.com".into();
        p.preferences.target_roles = vec!["Engineer".into()];
        p.skills.backend = vec!["Rust".into()];
        assert!(p.completeness().ready);
    }

    #[test]
    fn experience_years_are_summed() {
        let truth = CandidateTruth {
            profile: CandidateProfile::new("u"),
            experiences: vec![
                Experience {
                    start_date: "2020-01".into(),
                    end_date: Some("2022-01".into()),
                    ..Experience::default_for_test()
                },
                Experience {
                    start_date: "2022-01".into(),
                    end_date: Some("2023-07".into()),
                    ..Experience::default_for_test()
                },
            ],
            projects: vec![],
            master_resume_text: None,
        };
        assert!((truth.total_experience_years() - 3.5).abs() < 0.01);
    }

    impl Experience {
        pub(crate) fn default_for_test() -> Self {
            Self {
                id: new_id(),
                user_id: "u".into(),
                company: "Co".into(),
                role: "Dev".into(),
                location: String::new(),
                start_date: "2020-01".into(),
                end_date: None,
                description: String::new(),
                technologies: vec![],
                achievements: vec![],
                projects: vec![],
                created_at: now(),
                updated_at: now(),
            }
        }
    }
}
