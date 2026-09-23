//! Agent instructions, skills and schemas are authored as files under
//! `agent/` (the source of truth) and embedded at compile time so the desktop
//! binary is self-contained. This module also builds the per-task prompts.

use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::domain::*;
use crate::error::CoreResult;
use crate::util::truncate;

pub const SYSTEM: &str = include_str!("../../../agent/system/job-hunter.md");

pub mod skills {
    pub const CANDIDATE_PROFILE: &str =
        include_str!("../../../agent/skills/candidate-profile/SKILL.md");
    pub const JOB_DISCOVERY: &str = include_str!("../../../agent/skills/job-discovery/SKILL.md");
    pub const JOB_EXTRACTION: &str = include_str!("../../../agent/skills/job-extraction/SKILL.md");
    pub const JOB_ANALYSIS: &str = include_str!("../../../agent/skills/job-analysis/SKILL.md");
    pub const JOB_DEDUPLICATION: &str =
        include_str!("../../../agent/skills/job-deduplication/SKILL.md");
    pub const RESUME_GENERATION: &str =
        include_str!("../../../agent/skills/resume-generation/SKILL.md");
    pub const RESUME_VALIDATION: &str =
        include_str!("../../../agent/skills/resume-validation/SKILL.md");
    pub const COVER_LETTER: &str = include_str!("../../../agent/skills/cover-letter/SKILL.md");
    pub const APPLICATION_PREPARATION: &str =
        include_str!("../../../agent/skills/application-preparation/SKILL.md");
    pub const CHROME_APPLICATION: &str =
        include_str!("../../../agent/skills/chrome-application/SKILL.md");
    pub const APPLICATION_QUESTIONS: &str =
        include_str!("../../../agent/skills/application-questions/SKILL.md");
    pub const APPLICATION_TRACKING: &str =
        include_str!("../../../agent/skills/application-tracking/SKILL.md");
    pub const MANUAL_FALLBACK: &str =
        include_str!("../../../agent/skills/manual-fallback/SKILL.md");
}

pub mod schemas {
    use super::*;
    static DISCOVERY: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/discovery.json"))
            .expect("discovery schema")
    });
    static EXTRACTION: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/extraction.json"))
            .expect("extraction schema")
    });
    static ANALYSIS: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/analysis.json"))
            .expect("analysis schema")
    });
    static RESUME: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/resume.json"))
            .expect("resume schema")
    });
    static VALIDATION: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/validation.json"))
            .expect("validation schema")
    });
    static APPLY: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/apply.json"))
            .expect("apply schema")
    });
    static PROFILE_PARSE: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/profile-parse.json"))
            .expect("profile-parse schema")
    });

    pub fn discovery() -> Value {
        DISCOVERY.clone()
    }
    pub fn extraction() -> Value {
        EXTRACTION.clone()
    }
    pub fn analysis() -> Value {
        ANALYSIS.clone()
    }
    pub fn resume() -> Value {
        RESUME.clone()
    }
    pub fn validation() -> Value {
        VALIDATION.clone()
    }
    pub fn apply() -> Value {
        APPLY.clone()
    }
    pub fn profile_parse() -> Value {
        PROFILE_PARSE.clone()
    }
}

/// Claude in Chrome tools allowed during read-only discovery/extraction.
pub const CHROME_TOOLS_READONLY: &[&str] = &[
    "mcp__claude-in-chrome__list_connected_browsers",
    "mcp__claude-in-chrome__tabs_context_mcp",
    "mcp__claude-in-chrome__tabs_create_mcp",
    "mcp__claude-in-chrome__tabs_close_mcp",
    "mcp__claude-in-chrome__navigate",
    "mcp__claude-in-chrome__get_page_text",
    "mcp__claude-in-chrome__read_page",
    "mcp__claude-in-chrome__find",
    "mcp__claude-in-chrome__computer",
    "mcp__claude-in-chrome__browser_batch",
    "mcp__claude-in-chrome__resize_window",
];

/// Additional tools needed to fill and submit an approved application.
pub const CHROME_TOOLS_APPLY: &[&str] = &[
    "mcp__claude-in-chrome__form_input",
    "mcp__claude-in-chrome__file_upload",
];

pub fn chrome_tools(apply: bool) -> Vec<String> {
    let mut v: Vec<String> = CHROME_TOOLS_READONLY
        .iter()
        .map(|s| s.to_string())
        .collect();
    if apply {
        v.extend(CHROME_TOOLS_APPLY.iter().map(|s| s.to_string()));
    }
    v
}

/// Write the system prompt to the run directory (for `--append-system-prompt-file`).
pub fn write_system_prompt(run_dir: &Path) -> CoreResult<PathBuf> {
    std::fs::create_dir_all(run_dir)?;
    let path = run_dir.join("system-prompt.md");
    if !path.exists() {
        std::fs::write(&path, SYSTEM)?;
    }
    Ok(path)
}

/// Compact projection of the candidate for prompts.
pub fn candidate_for_prompt(truth: &CandidateTruth, include_master_text: bool) -> Value {
    let p = &truth.profile;
    let mut v = json!({
        "personal": p.personal,
        "preferences": p.preferences,
        "skills": p.skills,
        "education": p.education,
        "certifications": p.certifications,
        "achievements": p.achievements,
        "languages": p.languages,
        "summary": p.summary,
        "totalExperienceYears": (truth.total_experience_years() * 10.0).round() / 10.0,
        "experiences": truth.experiences.iter().map(|e| json!({
            "id": e.id, "company": e.company, "role": e.role, "location": e.location,
            "startDate": e.start_date, "endDate": e.end_date, "description": e.description,
            "technologies": e.technologies, "achievements": e.achievements,
        })).collect::<Vec<_>>(),
        "projects": truth.projects.iter().map(|pr| json!({
            "id": pr.id, "experienceId": pr.experience_id, "name": pr.name, "description": pr.description,
            "role": pr.role, "technologies": pr.technologies, "responsibilities": pr.responsibilities,
            "achievements": pr.achievements, "url": pr.url,
        })).collect::<Vec<_>>(),
    });
    if include_master_text {
        if let Some(text) = &truth.master_resume_text {
            v["masterResumeText"] = Value::String(truncate(text, 14000));
        }
    }
    v
}

fn job_for_prompt(job: &Job) -> Value {
    json!({
        "jobId": job.id,
        "title": job.title,
        "company": job.company,
        "location": job.location,
        "employmentType": job.employment_type,
        "remote": job.remote,
        "salary": job.salary,
        "seniority": job.seniority,
        "postedAt": job.posted_at,
        "url": job.url,
        "description": truncate(&job.description, 9000),
        "requirements": job.requirements,
        "responsibilities": job.responsibilities,
        "skills": job.skills,
    })
}

fn section(title: &str, body: &str) -> String {
    format!("\n\n## {title}\n\n{body}")
}

fn json_block(v: &Value) -> String {
    format!(
        "```json\n{}\n```",
        serde_json::to_string_pretty(v).unwrap_or_default()
    )
}

pub struct DiscoveryParams<'a> {
    pub source: &'a str,
    pub queries: &'a [String],
    pub locations: &'a [String],
    pub remote_preference: &'a str,
    pub recency_days: u32,
    pub max_jobs: u32,
    pub seen_urls: &'a [String],
    pub careers_url: Option<&'a str>,
}

pub fn discovery_prompt(p: &DiscoveryParams<'_>) -> String {
    let mut s = String::from("# Task: discover jobs\n\nFollow the job-discovery skill below exactly. This is a read-only task: do not apply, save, follow or message.");
    s.push_str(&section("Skill", skills::JOB_DISCOVERY));
    s.push_str(&section("Deduplication hints", skills::JOB_DEDUPLICATION));
    let input = json!({
        "source": p.source,
        "careersUrl": p.careers_url,
        "queries": p.queries,
        "locations": p.locations,
        "remotePreference": p.remote_preference,
        "recencyDays": p.recency_days,
        "maxJobs": p.max_jobs,
        "seenUrls": p.seen_urls.iter().take(200).collect::<Vec<_>>(),
    });
    s.push_str(&section("Inputs", &json_block(&input)));
    s.push_str("\n\nReturn the result using the structured output schema. Include full descriptions whenever you opened the posting.");
    s
}

pub fn extraction_prompt(job: &Job) -> String {
    let mut s = String::from("# Task: extract complete job details\n\nFollow the job-extraction skill exactly. Read-only: do not click Apply/Save/Follow.");
    s.push_str(&section("Skill", skills::JOB_EXTRACTION));
    s.push_str(&section(
        "Inputs",
        &json_block(&json!({
            "url": job.url,
            "known": { "title": job.title, "company": job.company, "location": job.location },
        })),
    ));
    s.push_str("\n\nReturn the result using the structured output schema.");
    s
}

pub fn analysis_prompt(truth: &CandidateTruth, jobs: &[Job]) -> String {
    let mut s = String::from("# Task: analyse job relevance\n\nFollow the job-analysis skill exactly. Use only the candidate data below; do not assume anything else about the candidate.");
    s.push_str(&section("Skill", skills::JOB_ANALYSIS));
    s.push_str(&section(
        "Candidate",
        &json_block(&candidate_for_prompt(truth, false)),
    ));
    let jobs_v: Vec<Value> = jobs.iter().map(job_for_prompt).collect();
    s.push_str(&section("Jobs", &json_block(&Value::Array(jobs_v))));
    s.push_str("\n\nReturn one analysis per job (same order, same jobId values) using the structured output schema.");
    s
}

pub fn resume_prompt(
    truth: &CandidateTruth,
    job: &Job,
    analysis: Option<&JobAnalysis>,
    include_cover_letter: bool,
    previous: Option<&AtsValidation>,
    previous_resume: Option<&ResumeDocument>,
) -> String {
    let mut s = String::from("# Task: generate a tailored ATS-friendly resume\n\nFollow the resume-generation skill exactly. Every statement must be supported by the candidate data below.");
    s.push_str(&section("Skill", skills::RESUME_GENERATION));
    if include_cover_letter {
        s.push_str(&section("Cover letter skill", skills::COVER_LETTER));
    }
    s.push_str(&section(
        "Candidate (source of truth)",
        &json_block(&candidate_for_prompt(truth, true)),
    ));
    s.push_str(&section("Job", &json_block(&job_for_prompt(job))));
    if let Some(a) = analysis {
        s.push_str(&section("Analysis", &json_block(&json!({
            "matchedSkills": a.matched_skills, "missingSkills": a.missing_skills,
            "importantKeywords": a.important_keywords, "requiredQualifications": a.required_qualifications,
            "concerns": a.concerns, "summary": a.summary,
        }))));
    }
    if let Some(prev) = previous {
        s.push_str(&section(
            "Previous validation (fix every point)",
            &json_block(&serde_json::to_value(prev).unwrap_or(Value::Null)),
        ));
        if let Some(doc) = previous_resume {
            s.push_str(&section(
                "Previous resume draft",
                &json_block(&serde_json::to_value(doc).unwrap_or(Value::Null)),
            ));
        }
    }
    s.push_str(&format!(
        "\n\nGenerate the resume{}. Return only the structured output.",
        if include_cover_letter {
            " and a cover letter"
        } else {
            " (leave coverLetter empty)"
        }
    ));
    s
}

pub fn validation_prompt(
    truth: &CandidateTruth,
    job: &Job,
    analysis: Option<&JobAnalysis>,
    resume: &ResumeDocument,
    iteration: u8,
    min_coverage: u8,
) -> String {
    let mut s = format!("# Task: validate a generated resume (iteration {iteration})\n\nFollow the resume-validation skill exactly. Target keyword coverage: {min_coverage}%.");
    s.push_str(&section("Skill", skills::RESUME_VALIDATION));
    s.push_str(&section(
        "Candidate (source of truth)",
        &json_block(&candidate_for_prompt(truth, true)),
    ));
    s.push_str(&section("Job", &json_block(&job_for_prompt(job))));
    if let Some(a) = analysis {
        s.push_str(&section("Important keywords", &json_block(&json!({ "importantKeywords": a.important_keywords, "matchedSkills": a.matched_skills, "missingSkills": a.missing_skills }))));
    }
    s.push_str(&section(
        "Resume to validate",
        &json_block(&serde_json::to_value(resume).unwrap_or(Value::Null)),
    ));
    s.push_str("\n\nReturn only the structured output.");
    s
}

pub struct ApplyParams<'a> {
    pub application: &'a Application,
    pub job: &'a Job,
    pub truth: &'a CandidateTruth,
    pub resume_pdf: Option<&'a str>,
    pub resume_docx: Option<&'a str>,
    pub cover_letter_pdf: Option<&'a str>,
    pub cover_letter_text: Option<&'a str>,
    pub answers: &'a [AnswerRecord],
    pub previously_answered: &'a [ApplicationAnswer],
    pub resuming: bool,
}

pub fn apply_prompt(p: &ApplyParams<'_>) -> String {
    let approved = p.application.is_approved();
    let mut s = format!(
        "# Task: complete an approved job application\n\nAPPROVED = {}\nAPPLICATION_ID = {}\n\nFollow the chrome-application, application-questions, application-tracking and manual-fallback skills exactly.",
        approved, p.application.id
    );
    if p.resuming {
        s.push_str("\n\nYou are RESUMING an application that stopped for user input. The user has now answered the questions listed under `previouslyAnswered`. Continue from where you stopped (the tab may still be open; if not, open the job URL again and re-fill).");
    }
    s.push_str(&section(
        "Skill: chrome-application",
        skills::CHROME_APPLICATION,
    ));
    s.push_str(&section(
        "Skill: application-questions",
        skills::APPLICATION_QUESTIONS,
    ));
    s.push_str(&section(
        "Skill: application-tracking",
        skills::APPLICATION_TRACKING,
    ));
    s.push_str(&section("Skill: manual-fallback", skills::MANUAL_FALLBACK));
    let profile = &p.truth.profile;
    let input = json!({
        "application": {
            "id": p.application.id,
            "approved": approved,
            "url": if p.application.application_url.is_empty() { p.job.url.clone() } else { p.application.application_url.clone() },
            "company": p.job.company,
            "title": p.job.title,
            "source": p.job.source,
        },
        "candidate": {
            "personal": profile.personal,
            "currentTitle": profile.personal.current_title,
            "noticePeriod": profile.preferences.notice_period,
            "salaryPreference": profile.preferences.salary,
            "totalExperienceYears": (p.truth.total_experience_years() * 10.0).round() / 10.0,
            "education": profile.education,
            "experiences": p.truth.experiences.iter().map(|e| json!({"company": e.company, "role": e.role, "startDate": e.start_date, "endDate": e.end_date, "location": e.location})).collect::<Vec<_>>(),
        },
        "documents": {
            "resumePdf": p.resume_pdf,
            "resumeDocx": p.resume_docx,
            "coverLetterPdf": p.cover_letter_pdf,
            "coverLetterText": p.cover_letter_text,
        },
        "answers": p.answers.iter().map(|a| json!({"question": a.question, "answer": a.answer})).collect::<Vec<_>>(),
        "previouslyAnswered": p.previously_answered,
    });
    s.push_str(&section("Inputs", &json_block(&input)));
    s.push_str("\n\nReturn only the structured output. Remember: SUBMITTED requires visible evidence; otherwise report HUMAN_INPUT_REQUIRED, MANUAL_ACTION_REQUIRED or FAILED.");
    s
}

pub fn profile_parse_prompt(resume_text: &str, existing: Option<&CandidateProfile>) -> String {
    let mut s = String::from("# Task: parse a master resume into structured candidate data\n\nFollow the candidate-profile skill exactly. Extract only what the document states.");
    s.push_str(&section("Skill", skills::CANDIDATE_PROFILE));
    if let Some(e) = existing {
        s.push_str(&section(
            "Existing profile (do not overwrite non-empty values)",
            &json_block(&json!({"personal": e.personal, "skills": e.skills})),
        ));
    }
    s.push_str(&section(
        "Resume text",
        &format!("```\n{}\n```", truncate(resume_text, 20000)),
    ));
    s.push_str("\n\nReturn only the structured output.");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_parse_and_system_prompt_has_rules() {
        assert!(schemas::discovery()["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "jobs"));
        assert!(
            schemas::apply()["properties"]["outcome"]["enum"]
                .as_array()
                .unwrap()
                .len()
                == 4
        );
        assert!(SYSTEM.contains("never claim an application was submitted"));
        assert!(skills::CHROME_APPLICATION.contains("Do not bypass CAPTCHA"));
    }

    #[test]
    fn apply_prompt_states_approval() {
        let job = Job::new("u", "LinkedIn", "https://x/1", "Acme", "Dev");
        let mut app = Application::new("u", &job.id, &job.url, "LinkedIn");
        let truth = CandidateTruth {
            profile: CandidateProfile::new("u"),
            experiences: vec![],
            projects: vec![],
            master_resume_text: None,
        };
        fn make<'a>(
            app: &'a Application,
            job: &'a Job,
            truth: &'a CandidateTruth,
        ) -> ApplyParams<'a> {
            ApplyParams {
                application: app,
                job,
                truth,
                resume_pdf: None,
                resume_docx: None,
                cover_letter_pdf: None,
                cover_letter_text: None,
                answers: &[],
                previously_answered: &[],
                resuming: false,
            }
        }
        assert!(apply_prompt(&make(&app, &job, &truth)).contains("APPROVED = false"));
        app.transition(ApplicationStatus::Analyzed, "").unwrap();
        app.transition(ApplicationStatus::ReadyForReview, "")
            .unwrap();
        app.transition(ApplicationStatus::Approved, "").unwrap();
        assert!(apply_prompt(&make(&app, &job, &truth)).contains("APPROVED = true"));
    }
}
