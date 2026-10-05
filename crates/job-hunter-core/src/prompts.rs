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
    pub const PROJECT_DRAFTING: &str =
        include_str!("../../../agent/skills/project-drafting/SKILL.md");
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
    pub const PROJECT_SELECTION: &str =
        include_str!("../../../agent/skills/project-selection/SKILL.md");
    pub const PROFILE_SYNC: &str = include_str!("../../../agent/skills/profile-sync/SKILL.md");
    pub const EMAIL_APPLICATION: &str =
        include_str!("../../../agent/skills/email-application/SKILL.md");
    pub const SHARED_JOB_READING: &str =
        include_str!("../../../agent/skills/shared-job-reading/SKILL.md");
    pub const EMAIL_DRAFT: &str = include_str!("../../../agent/skills/email-draft/SKILL.md");
    pub const INBOX_TRACKING: &str = include_str!("../../../agent/skills/inbox-tracking/SKILL.md");
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
    static PROJECT_DRAFT: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/project-draft.json"))
            .expect("project-draft schema")
    });
    pub fn project_draft() -> Value {
        PROJECT_DRAFT.clone()
    }
    static PROJECT_SELECTION: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!(
            "../../../agent/schemas/project-selection.json"
        ))
        .expect("project-selection schema")
    });
    pub fn project_selection() -> Value {
        PROJECT_SELECTION.clone()
    }
    static PROFILE_SYNC: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/profile-sync.json"))
            .expect("profile-sync schema")
    });
    pub fn profile_sync() -> Value {
        PROFILE_SYNC.clone()
    }
    static INBOX_CHECK: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/inbox-check.json"))
            .expect("inbox-check schema")
    });
    pub fn inbox_check() -> Value {
        INBOX_CHECK.clone()
    }
    static SHARED_JOB: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/shared-job.json"))
            .expect("shared-job schema")
    });
    pub fn shared_job() -> Value {
        SHARED_JOB.clone()
    }
    static EMAIL_DRAFT: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../agent/schemas/email-draft.json"))
            .expect("email-draft schema")
    });
    pub fn email_draft() -> Value {
        EMAIL_DRAFT.clone()
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
    // Rewrite when stale: fixed task dirs (profile-parse, ...) outlive app
    // upgrades that change the system prompt.
    if std::fs::read_to_string(&path).ok().as_deref() != Some(SYSTEM) {
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
    /// Telegram source only: `[{channel, sinceMessageId, sinceTime}]`.
    pub telegram_channels: Option<&'a Value>,
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
        "telegramChannels": p.telegram_channels,
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

/// The email an email application sends: to the posting's address, a plain
/// subject, and the approved cover letter as the body (the user reviews and
/// can edit it before approving).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub fn email_message(
    job: &Job,
    truth: &CandidateTruth,
    cover_letter: Option<&str>,
) -> Option<EmailMessage> {
    let to = job.apply_email.as_deref()?.trim();
    if to.is_empty() {
        return None;
    }
    Some(compose_email(to, job, truth, cover_letter))
}

/// The email for a Gmail draft: like [`email_message`], but `to` may be
/// empty (the posting gave no address; the candidate fills it in).
pub fn draft_email_message(
    job: &Job,
    truth: &CandidateTruth,
    cover_letter: Option<&str>,
) -> EmailMessage {
    let to = job.apply_email.as_deref().unwrap_or("").trim();
    compose_email(to, job, truth, cover_letter)
}

fn compose_email(
    to: &str,
    job: &Job,
    truth: &CandidateTruth,
    cover_letter: Option<&str>,
) -> EmailMessage {
    let personal = &truth.profile.personal;
    let body = match cover_letter.map(str::trim).filter(|c| !c.is_empty()) {
        Some(letter) => letter.to_string(),
        None => {
            let greeting = job
                .contact_name
                .as_deref()
                .and_then(|n| n.split_whitespace().next())
                .map(|first| format!("Hi {first},"))
                .unwrap_or_else(|| "Hello,".to_string());
            let contact: Vec<&str> = [
                personal.phone.as_str(),
                personal.email.as_str(),
                personal.linkedin.as_str(),
            ]
            .into_iter()
            .filter(|c| !c.trim().is_empty())
            .collect();
            format!(
                "{greeting}

I'd like to apply for the {} role at {}. My resume is attached.

Regards,
{}
{}",
                job.title,
                job.company,
                personal.name,
                contact.join(" | ")
            )
        }
    };
    EmailMessage {
        to: to.to_string(),
        subject: format!("Application for {} - {}", job.title, personal.name),
        body,
    }
}

/// Prompt for reading a job the candidate shared (text, screenshots, PDFs).
pub fn shared_job_prompt(text: &str, files: &[String]) -> String {
    let mut s = String::from(
        "# Task: read a job posting the candidate shared

Follow the shared-job-reading skill exactly. Read every file with the Read tool.",
    );
    s.push_str(&section("Skill", skills::SHARED_JOB_READING));
    s.push_str(&section(
        "Inputs",
        &json_block(&json!({ "text": truncate(text, 20_000), "files": files })),
    ));
    s.push_str(
        "

Return only the structured output.",
    );
    s
}

/// Prompt for saving (never sending) an application email as a Gmail draft.
pub fn email_draft_prompt(email: &EmailMessage, resume_pdf: Option<&str>) -> String {
    let mut s = String::from(
        "# Task: save a job-application email as a Gmail draft

Follow the email-draft skill exactly. NEVER click Send: the candidate reviews and sends it.",
    );
    s.push_str(&section("Skill: email-draft", skills::EMAIL_DRAFT));
    s.push_str(&section(
        "Inputs",
        &json_block(&json!({ "email": email, "documents": { "resumePdf": resume_pdf } })),
    ));
    s.push_str(
        "

Return only the structured output.",
    );
    s
}

pub fn apply_prompt(p: &ApplyParams<'_>) -> String {
    let approved = p.application.is_approved();
    if let Some(email) = email_message(p.job, p.truth, p.cover_letter_text) {
        return email_apply_prompt(p, approved, &email);
    }
    let mut s = format!(
        "# Task: complete an approved job application\n\nAPPROVED = {}\nAPPLICATION_ID = {}\n\nFollow the chrome-application, application-questions, application-tracking and manual-fallback skills exactly.",
        approved, p.application.id
    );
    if p.resuming {
        s.push_str("\n\nYou are RESUMING an application that stopped part-way: for the user's answers (listed under `previouslyAnswered`, if any), for a manual step the user may since have done (sign-in, CAPTCHA), or because the previous run hit a time or turn limit. The tab is probably still open: call `tabs_context_mcp`, continue in it from where the form is, and don't redo completed steps. If the tab is gone, open the job URL again and re-fill.");
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

fn email_apply_prompt(p: &ApplyParams<'_>, approved: bool, email: &EmailMessage) -> String {
    let mut s = format!(
        "# Task: send an approved job application by email

APPROVED = {}
APPLICATION_ID = {}

The posting asks candidates to email their resume. Follow the email-application and manual-fallback skills exactly.",
        approved, p.application.id
    );
    if p.resuming {
        s.push_str("

You are RESUMING: the previous run stopped part-way. Check Gmail's Drafts and Sent first: if this message was already sent, report SUBMITTED with that evidence; if a draft exists, finish and send it instead of composing a new one.");
    }
    s.push_str(&section(
        "Skill: email-application",
        skills::EMAIL_APPLICATION,
    ));
    s.push_str(&section("Skill: manual-fallback", skills::MANUAL_FALLBACK));
    let input = json!({
        "application": {
            "id": p.application.id,
            "approved": approved,
            "url": p.job.url,
            "company": p.job.company,
            "title": p.job.title,
            "source": p.job.source,
        },
        "email": email,
        "documents": { "resumePdf": p.resume_pdf },
    });
    s.push_str(&section("Inputs", &json_block(&input)));
    s.push_str(
        "

Return only the structured output. SUBMITTED requires Gmail's \"Message sent\" confirmation.",
    );
    s
}

/// Prompt for finding employers' replies in Gmail (inbox and spam).
pub fn inbox_check_prompt(applications: &Value, since_days: u32) -> String {
    let mut s = String::from("# Task: find employers' replies to the candidate's applications in Gmail

Follow the inbox-tracking skill exactly. Read-only: never reply, send, delete, archive, label or move anything.");
    s.push_str(&section("Skill: inbox-tracking", skills::INBOX_TRACKING));
    s.push_str(&section(
        "Inputs",
        &json_block(&json!({ "sinceDays": since_days, "applications": applications })),
    ));
    s.push_str(
        "

Return only the structured output.",
    );
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

/// Prompt for turning the candidate's own description of a project into a
/// structured `Project` (see the project-drafting skill).
pub fn project_draft_prompt(
    description: &str,
    experiences: &[Experience],
    chosen_employer: Option<&Experience>,
) -> String {
    let mut s = String::from(
        "# Task: turn the candidate's description of a project into a structured project entry

Follow the project-drafting skill exactly. Use only what the description states.",
    );
    s.push_str(&section("Skill", skills::PROJECT_DRAFTING));
    let employers: Vec<Value> = experiences
        .iter()
        .map(|e| json!({"company": e.company, "role": e.role, "startDate": e.start_date, "endDate": e.end_date}))
        .collect();
    s.push_str(&section("Employers", &json_block(&json!(employers))));
    if let Some(e) = chosen_employer {
        s.push_str(&section(
            "Employer the candidate chose",
            &json_block(&json!({"company": e.company, "role": e.role})),
        ));
    }
    s.push_str(&section(
        "Candidate's description",
        &format!(
            "```
{}
```",
            truncate(description, 8000)
        ),
    ));
    s.push_str(
        "

Return only the structured output.",
    );
    s
}

/// Prompt for choosing which projects to feature on job-site profiles.
pub fn project_selection_prompt(truth: &CandidateTruth) -> String {
    let mut s = String::from(
        "# Task: choose the projects to feature on the candidate's job-site profiles

Follow the project-selection skill exactly. Judge only from the data below.",
    );
    s.push_str(&section("Skill", skills::PROJECT_SELECTION));
    let p = &truth.profile;
    let input = json!({
        "targetRoles": p.preferences.target_roles,
        "skills": p.skills.all(),
        "experiences": truth.experiences.iter().map(|e| json!({
            "id": e.id, "company": e.company, "role": e.role, "startDate": e.start_date, "endDate": e.end_date,
        })).collect::<Vec<_>>(),
        "projects": truth.projects.iter().map(|pr| json!({
            "id": pr.id, "name": pr.name, "description": pr.description, "role": pr.role,
            "employer": pr.experience_id.as_ref().and_then(|id| truth.experiences.iter().find(|e| &e.id == id)).map(|e| e.company.clone()),
            "technologies": pr.technologies, "responsibilities": pr.responsibilities, "achievements": pr.achievements, "url": pr.url,
        })).collect::<Vec<_>>(),
    });
    s.push_str(&section("Inputs", &json_block(&input)));
    s.push_str(
        "

Return one pick per project using the structured output schema.",
    );
    s
}

pub struct ProfileSyncParams<'a> {
    pub platform: &'a str,
    /// The content to publish (see `agent::profile_sync::platform_content`).
    pub profile: &'a Value,
    pub remove_projects: &'a [String],
    pub known_answers: &'a [AnswerRecord],
    pub previously_answered: &'a [ApplicationAnswer],
    pub resume_path: Option<&'a str>,
    /// Continuing a session that stopped part-way.
    pub resuming: bool,
}

/// Prompt for filling in the candidate's own profile on one job site.
pub fn profile_sync_prompt(p: &ProfileSyncParams<'_>) -> String {
    let mut s = format!(
        "# Task: profile update on {}

CONFIRMED = true (the candidate asked for this update and approved the content)

Follow the profile-sync skill exactly.",
        p.platform
    );
    if p.resuming {
        s.push_str("

You are RESUMING this update: your previous turn in this conversation stopped part-way (a time, turn or budget limit, or the candidate answered the questions you reported). The browser tab you were working in is probably still open: call `tabs_context_mcp` and continue in that tab from where it is. Sections you already saved are done; check them quickly and move on. Don't start over.");
    }
    s.push_str(&section("Skill: profile-sync", skills::PROFILE_SYNC));
    let input = json!({
        "platform": p.platform,
        "profile": p.profile,
        "removeProjects": p.remove_projects,
        "knownAnswers": p.known_answers.iter().map(|a| json!({"question": a.question, "answer": a.answer})).collect::<Vec<_>>(),
        "previouslyAnswered": p.previously_answered,
        "resumePath": p.resume_path,
    });
    s.push_str(&section("Inputs", &json_block(&input)));
    s.push_str(
        "

Return only the structured output. Report only changes you saw saved.",
    );
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

    #[test]
    fn project_draft_prompt_carries_the_rules_the_description_and_the_employers() {
        let schema = schemas::project_draft();
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "responsibilities"));
        let prompt = project_draft_prompt("Built a school management system", &[], None);
        assert!(prompt.contains("Never invent features, metrics"));
        assert!(prompt.contains("Built a school management system"));
        assert!(!prompt.contains("Employer the candidate chose"));
    }
}
