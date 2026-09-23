//! Deterministic stand-in for Claude used when `MOCK_MODE=true` and in tests.
//! It never touches Claude, Chrome or the network and never reports a
//! submitted application.

use std::time::Duration;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::protocol::{ClaudeEvent, ClaudeRequest, ClaudeResponse, EventSink};
use super::runner::ClaudeRunner;
use crate::error::{CoreError, CoreResult};
use crate::util::normalize_text;

static FIXTURE_JOBS: Lazy<Value> = Lazy::new(|| {
    serde_json::from_str(include_str!(
        "../../../../agent/fixtures/discovered-jobs.json"
    ))
    .expect("fixture jobs")
});
static FIXTURE_CANDIDATE: Lazy<Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../../../agent/fixtures/candidate.json"))
        .expect("fixture candidate")
});

pub struct MockClaudeRunner {
    pub delay: Duration,
}

impl Default for MockClaudeRunner {
    fn default() -> Self {
        Self {
            delay: Duration::from_millis(250),
        }
    }
}

impl MockClaudeRunner {
    pub fn instant() -> Self {
        Self {
            delay: Duration::ZERO,
        }
    }

    pub fn fixture_candidate() -> Value {
        FIXTURE_CANDIDATE.clone()
    }

    async fn step(
        &self,
        sink: &EventSink,
        cancel: &CancellationToken,
        summary: &str,
    ) -> CoreResult<()> {
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        sink(ClaudeEvent::ToolUse {
            name: "mock".into(),
            summary: summary.into(),
        });
        if !self.delay.is_zero() {
            tokio::select! {
                _ = tokio::time::sleep(self.delay) => {}
                _ = cancel.cancelled() => return Err(CoreError::Cancelled),
            }
        }
        Ok(())
    }

    fn discover(ctx: &Value) -> Value {
        let source = ctx
            .get("source")
            .and_then(|s| s.as_str())
            .unwrap_or("LinkedIn");
        let max = ctx.get("maxJobs").and_then(|m| m.as_u64()).unwrap_or(10) as usize;
        let seen: Vec<String> = ctx
            .get("seenUrls")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let jobs: Vec<Value> = FIXTURE_JOBS
            .get(source)
            .and_then(|j| j.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|j| {
                !seen
                    .iter()
                    .any(|s| j.get("url").and_then(|u| u.as_str()) == Some(s.as_str()))
            })
            .take(max)
            .collect();
        json!({ "jobs": jobs, "notes": format!("mock discovery on {source}"), "blocked": false })
    }

    fn extract(ctx: &Value) -> Value {
        let mut job = ctx.get("job").cloned().unwrap_or(json!({}));
        if job
            .get("description")
            .and_then(|d| d.as_str())
            .map(|d| d.is_empty())
            .unwrap_or(true)
        {
            job["description"] = Value::String("Mock extracted description. The role involves building web applications with React and Node.js.".into());
        }
        job["detailsComplete"] = Value::Bool(true);
        json!({ "found": true, "blocked": false, "job": job })
    }

    fn analyze(ctx: &Value) -> Value {
        let candidate_skills: Vec<String> = ctx
            .pointer("/candidate/skills")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(normalize_text))
                    .collect()
            })
            .unwrap_or_default();
        let years = ctx
            .pointer("/candidate/totalExperienceYears")
            .and_then(|y| y.as_f64())
            .unwrap_or(0.0);
        let target_roles: Vec<String> = ctx
            .pointer("/candidate/targetRoles")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(normalize_text))
                    .collect()
            })
            .unwrap_or_default();
        let mut analyses = Vec::new();
        for job in ctx
            .get("jobs")
            .and_then(|j| j.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let job_id = job
                .get("jobId")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let title = normalize_text(job.get("title").and_then(|s| s.as_str()).unwrap_or(""));
            let skills: Vec<String> = job
                .get("skills")
                .and_then(|s| s.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let mut matched = Vec::new();
            let mut missing = Vec::new();
            for s in &skills {
                if candidate_skills.iter().any(|c| c == &normalize_text(s)) {
                    matched.push(s.clone());
                } else {
                    missing.push(s.clone());
                }
            }
            let skill_ratio = if skills.is_empty() {
                0.5
            } else {
                matched.len() as f64 / skills.len() as f64
            };
            let role_words: Vec<&str> = title.split(' ').collect();
            let role_match = target_roles.iter().any(|r| {
                r.split(' ')
                    .filter(|w| w.len() > 3)
                    .any(|w| role_words.contains(&w))
            });
            let required_years = job
                .get("requirements")
                .and_then(|r| r.as_array())
                .and_then(|a| {
                    a.iter().filter_map(|v| v.as_str()).find_map(|s| {
                        s.split('+')
                            .next()
                            .and_then(|n| n.trim().split(' ').next_back())
                            .and_then(|n| n.parse::<f64>().ok())
                            .filter(|_| s.to_lowercase().contains("year"))
                    })
                })
                .unwrap_or(0.0);
            let experience_met = required_years <= years.max(0.5) + 0.5;
            let seniority_match =
                !title.contains("staff") && !title.contains("director") && !title.contains("vp");
            let mut score = (skill_ratio * 70.0) as i64
                + if role_match { 20 } else { 0 }
                + if experience_met { 10 } else { -30 };
            score = score.clamp(0, 100);
            let relevant = role_match && experience_met && seniority_match && skill_ratio >= 0.4;
            let mut concerns = Vec::new();
            if !experience_met {
                concerns.push(format!("Requires about {required_years:.0} years of experience; candidate has {years:.1}."));
            }
            if !seniority_match {
                concerns.push("Seniority appears above the candidate's target level.".into());
            }
            analyses.push(json!({
                "jobId": job_id,
                "relevant": relevant,
                "matchScore": score,
                "matchedSkills": matched,
                "missingSkills": missing,
                "requiredExperienceMet": experience_met,
                "seniorityMatch": seniority_match,
                "locationMatch": true,
                "employmentTypeMatch": true,
                "salaryAssessment": job.get("salary").and_then(|s| s.as_str()).map(|s| format!("Posting states: {s}")).unwrap_or_else(|| "not stated".into()),
                "requiredQualifications": job.get("requirements").cloned().unwrap_or(json!([])),
                "niceToHave": [],
                "concerns": concerns,
                "importantKeywords": skills,
                "summary": if relevant { "Mock analysis: the role matches the candidate's target roles and most required skills." } else { "Mock analysis: the role does not fit the candidate's target roles, seniority or experience." },
            }));
        }
        json!({ "analyses": analyses })
    }

    fn generate_resume(ctx: &Value) -> Value {
        let c = ctx.get("candidate").cloned().unwrap_or(json!({}));
        let job = ctx.get("job").cloned().unwrap_or(json!({}));
        let personal = c.get("personal").cloned().unwrap_or(json!({}));
        let s = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let job_skills: Vec<String> = job
            .get("skills")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let mut skill_sections = Vec::new();
        if let Some(groups) = c.get("skills").and_then(|g| g.as_object()) {
            for (name, items) in groups {
                let mut list: Vec<String> = items
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if list.is_empty() {
                    continue;
                }
                // Put the posting's keywords first (only ones the candidate has).
                list.sort_by_key(|x| {
                    !job_skills
                        .iter()
                        .any(|j| normalize_text(j) == normalize_text(x))
                });
                let title = match name.as_str() {
                    "frontend" => "Frontend",
                    "backend" => "Backend",
                    "database" => "Databases",
                    "devops" => "DevOps",
                    "cloud" => "Cloud",
                    "testing" => "Testing",
                    _ => "Other",
                };
                skill_sections.push(json!({ "name": title, "items": list }));
            }
        }
        let experience: Vec<Value> = c
            .get("experiences")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|e| {
                let mut bullets: Vec<String> = e.get("achievements").and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
                let desc = s(e, "description");
                if !desc.is_empty() {
                    bullets.insert(0, desc);
                }
                json!({
                    "company": s(e, "company"), "role": s(e, "role"), "location": s(e, "location"),
                    "startDate": s(e, "startDate"), "endDate": e.get("endDate").and_then(|x| x.as_str()).unwrap_or("Present"),
                    "bullets": bullets,
                })
            })
            .collect();
        let projects: Vec<Value> = c
            .get("projects")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|p| json!({ "name": s(p, "name"), "description": s(p, "description"), "technologies": p.get("technologies").cloned().unwrap_or(json!([])), "bullets": p.get("achievements").cloned().unwrap_or(json!([])), "url": s(p, "url") }))
            .collect();
        let education: Vec<Value> = c
            .get("education")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|e| json!({ "institution": s(e, "institution"), "degree": s(e, "degree"), "field": s(e, "field"), "startDate": s(e, "startDate"), "endDate": s(e, "endDate"), "grade": s(e, "grade") }))
            .collect();
        let certifications: Vec<String> = c
            .get("certifications")
            .and_then(|e| e.as_array())
            .map(|a| {
                a.iter()
                    .map(|x| s(x, "name"))
                    .filter(|n| !n.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let name = s(&personal, "name");
        let company = s(&job, "company");
        let title = s(&job, "title");
        let include_cl = ctx
            .get("includeCoverLetter")
            .and_then(|b| b.as_bool())
            .unwrap_or(true);
        let cover_letter = if include_cl {
            format!(
                "Dear Hiring Team at {company},\n\nI am writing to apply for the {title} role. {}\n\nMy recent work maps closely to what the posting describes, and I would welcome the chance to bring that experience to your team.\n\nThank you for your consideration.\n\n{name}",
                s(&c, "summary")
            )
        } else {
            String::new()
        };
        json!({
            "resume": {
                "contact": personal,
                "headline": s(&personal, "currentTitle"),
                "summary": format!("{} Interested in the {title} role at {company}.", s(&c, "summary")),
                "skills": skill_sections,
                "experience": experience,
                "projects": projects,
                "education": education,
                "certifications": certifications,
                "achievements": c.get("achievements").cloned().unwrap_or(json!([])),
                "languages": c.get("languages").cloned().unwrap_or(json!([])),
            },
            "coverLetter": cover_letter,
            "applicationNotes": ["Generated in mock mode from the candidate profile."],
        })
    }

    fn validate_resume(ctx: &Value) -> Value {
        let resume = ctx.get("resume").cloned().unwrap_or(json!({}));
        let candidate_skills: Vec<String> = ctx
            .pointer("/candidateSkills")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(normalize_text))
                    .collect()
            })
            .unwrap_or_default();
        let keywords: Vec<String> = ctx
            .get("keywords")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let text = normalize_text(&resume.to_string());
        let mut missing = Vec::new();
        let mut hits = 0;
        for k in &keywords {
            if text.contains(&normalize_text(k)) {
                hits += 1;
            } else {
                missing.push(k.clone());
            }
        }
        let coverage = if keywords.is_empty() {
            100
        } else {
            (hits * 100 / keywords.len()) as u8
        };
        let mut unsupported = Vec::new();
        if let Some(sections) = resume.get("skills").and_then(|s| s.as_array()) {
            for sec in sections {
                for item in sec
                    .get("items")
                    .and_then(|i| i.as_array())
                    .cloned()
                    .unwrap_or_default()
                {
                    if let Some(i) = item.as_str() {
                        if !candidate_skills.is_empty()
                            && !candidate_skills.contains(&normalize_text(i))
                        {
                            unsupported.push(i.to_string());
                        }
                    }
                }
            }
        }
        let status = if !unsupported.is_empty() {
            "REVISE"
        } else {
            "PASS"
        };
        json!({
            "status": status,
            "keywordCoverage": coverage,
            "missingKeywords": missing,
            "unsupportedClaims": unsupported,
            "missingRequirements": [],
            "formattingIssues": [],
            "notes": if status == "PASS" { "Mock validation passed." } else { "Remove skills that are not in the candidate profile." },
        })
    }

    fn apply(ctx: &Value) -> Value {
        let simulate = ctx
            .get("simulate")
            .and_then(|s| s.as_str())
            .unwrap_or("MANUAL_ACTION_REQUIRED");
        match simulate {
            "HUMAN_INPUT_REQUIRED" => json!({
                "outcome": "HUMAN_INPUT_REQUIRED",
                "reason": "The form asks questions that are not in the answer database.",
                "evidence": "",
                "applicationUrl": ctx.pointer("/application/url").cloned().unwrap_or(json!("")),
                "answersUsed": [],
                "unknownQuestions": [
                    { "question": "Are you authorized to work in India?", "fieldType": "radio", "options": ["Yes", "No"], "required": true, "context": "" },
                    { "question": "What is your expected annual salary (INR)?", "fieldType": "number", "options": [], "required": true, "context": "" }
                ],
                "stepsCompleted": ["Opened the job page", "Started the application form", "Filled contact details"],
            }),
            "SUBMITTED" => json!({
                "outcome": "SUBMITTED",
                "reason": "Mock submission (no real website was touched).",
                "evidence": "MOCK: confirmation page 'Your application has been submitted'",
                "applicationUrl": ctx.pointer("/application/url").cloned().unwrap_or(json!("")),
                "answersUsed": [],
                "unknownQuestions": [],
                "stepsCompleted": ["Opened the job page", "Filled the form", "Uploaded resume", "Submitted"],
            }),
            _ => json!({
                "outcome": "MANUAL_ACTION_REQUIRED",
                "reason": "Mock mode never submits applications. Complete this one manually.",
                "evidence": "",
                "applicationUrl": ctx.pointer("/application/url").cloned().unwrap_or(json!("")),
                "answersUsed": [],
                "unknownQuestions": [],
                "stepsCompleted": ["Opened the job page"],
            }),
        }
    }

    fn parse_profile() -> Value {
        let c = FIXTURE_CANDIDATE.clone();
        let profile = c.get("profile").cloned().unwrap_or(json!({}));
        json!({
            "personal": profile.get("personal").cloned().unwrap_or(json!({})),
            "summary": profile.get("summary").cloned().unwrap_or(json!("")),
            "skills": profile.get("skills").cloned().unwrap_or(json!({})),
            "experiences": c.get("experiences").cloned().unwrap_or(json!([])),
            "projects": c.get("projects").and_then(|p| p.as_array()).map(|a| a.iter().map(|p| { let mut p = p.clone(); p["experienceCompany"] = json!("Meridian Retail"); p }).collect::<Vec<_>>()).unwrap_or_default(),
            "education": profile.get("education").cloned().unwrap_or(json!([])),
            "certifications": profile.get("certifications").cloned().unwrap_or(json!([])),
            "achievements": profile.get("achievements").cloned().unwrap_or(json!([])),
            "languages": profile.get("languages").cloned().unwrap_or(json!([])),
        })
    }
}

#[async_trait]
impl ClaudeRunner for MockClaudeRunner {
    async fn run(
        &self,
        req: ClaudeRequest,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> CoreResult<ClaudeResponse> {
        sink(ClaudeEvent::Init {
            session_id: format!("mock-{}", crate::util::new_id()),
            tools: 0,
        });
        let label = req.label.split(':').next().unwrap_or("").to_string();
        let ctx = &req.mock_context;
        let structured = match label.as_str() {
            "discover" => {
                self.step(&sink, &cancel, "Opening a new browser tab (mock)")
                    .await?;
                self.step(
                    &sink,
                    &cancel,
                    &format!(
                        "Searching {} (mock)",
                        ctx.get("source")
                            .and_then(|s| s.as_str())
                            .unwrap_or("source")
                    ),
                )
                .await?;
                self.step(&sink, &cancel, "Reading the page (mock)").await?;
                Self::discover(ctx)
            }
            "extract" => {
                self.step(&sink, &cancel, "Opening the job page (mock)")
                    .await?;
                Self::extract(ctx)
            }
            "analyze" => {
                self.step(
                    &sink,
                    &cancel,
                    "Comparing jobs with the candidate profile (mock)",
                )
                .await?;
                Self::analyze(ctx)
            }
            "generate_resume" => {
                self.step(&sink, &cancel, "Writing a tailored resume (mock)")
                    .await?;
                Self::generate_resume(ctx)
            }
            "validate_resume" => {
                self.step(&sink, &cancel, "Validating the resume (mock)")
                    .await?;
                Self::validate_resume(ctx)
            }
            "apply" => {
                self.step(&sink, &cancel, "Opening the job page (mock)")
                    .await?;
                self.step(&sink, &cancel, "Filling the application form (mock)")
                    .await?;
                Self::apply(ctx)
            }
            "parse_profile" => {
                self.step(&sink, &cancel, "Parsing the master resume (mock)")
                    .await?;
                Self::parse_profile()
            }
            other => {
                return Err(CoreError::other(format!(
                    "mock runner has no behaviour for task '{other}'"
                )))
            }
        };
        sink(ClaudeEvent::Result {
            subtype: "success".into(),
            is_error: false,
        });
        Ok(ClaudeResponse {
            session_id: Some("mock-session".into()),
            subtype: "success".into(),
            is_error: false,
            text: String::new(),
            structured: Some(structured),
            cost_usd: 0.0,
            num_turns: 1,
            permission_denials: vec![],
            raw_log_path: None,
            duration_ms: 0,
        })
    }

    fn is_mock(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::protocol::noop_sink;
    use std::path::PathBuf;

    #[tokio::test]
    async fn mock_discovery_returns_fixture_jobs_and_skips_seen() {
        let runner = MockClaudeRunner::instant();
        let mut req = ClaudeRequest::new("discover:LinkedIn", String::new(), PathBuf::from("."));
        req.mock_context = json!({ "source": "LinkedIn", "maxJobs": 10, "seenUrls": ["https://www.linkedin.com/jobs/view/4001001002"] });
        let resp = runner
            .run(req, noop_sink(), CancellationToken::new())
            .await
            .unwrap();
        let jobs = resp.structured.unwrap()["jobs"].as_array().unwrap().len();
        assert_eq!(jobs, 2);
    }

    #[tokio::test]
    async fn mock_analysis_flags_relevant_and_irrelevant() {
        let runner = MockClaudeRunner::instant();
        let mut req = ClaudeRequest::new("analyze", String::new(), PathBuf::from("."));
        req.mock_context = json!({
            "candidate": { "skills": ["React", "Node.js", "PostgreSQL", "TypeScript"], "totalExperienceYears": 5.0, "targetRoles": ["Full Stack Developer"] },
            "jobs": [
                { "jobId": "a", "title": "Senior Full Stack Developer", "skills": ["React", "Node.js", "PostgreSQL", "Docker"], "requirements": ["5+ years of experience"] },
                { "jobId": "b", "title": "Staff Machine Learning Engineer", "skills": ["PyTorch", "CUDA"], "requirements": ["10+ years of experience"] }
            ]
        });
        let resp = runner
            .run(req, noop_sink(), CancellationToken::new())
            .await
            .unwrap();
        let analyses = resp.structured.unwrap()["analyses"].clone();
        assert_eq!(analyses[0]["relevant"], json!(true));
        assert_eq!(analyses[0]["missingSkills"], json!(["Docker"]));
        assert_eq!(analyses[1]["relevant"], json!(false));
        assert!(!analyses[1]["concerns"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn mock_validation_detects_unsupported_claims() {
        let runner = MockClaudeRunner::instant();
        let mut req = ClaudeRequest::new("validate_resume", String::new(), PathBuf::from("."));
        req.mock_context = json!({
            "resume": { "skills": [{ "name": "Backend", "items": ["Node.js", "Kubernetes"] }], "summary": "Node.js developer" },
            "candidateSkills": ["Node.js", "React"],
            "keywords": ["Node.js", "Kubernetes"]
        });
        let resp = runner
            .run(req, noop_sink(), CancellationToken::new())
            .await
            .unwrap();
        let v = resp.structured.unwrap();
        assert_eq!(v["status"], json!("REVISE"));
        assert_eq!(v["unsupportedClaims"], json!(["Kubernetes"]));
        assert_eq!(v["keywordCoverage"], json!(100));
    }

    #[tokio::test]
    async fn mock_apply_never_submits_by_default() {
        let runner = MockClaudeRunner::instant();
        let req = ClaudeRequest::new("apply", String::new(), PathBuf::from("."));
        let resp = runner
            .run(req, noop_sink(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            resp.structured.unwrap()["outcome"],
            json!("MANUAL_ACTION_REQUIRED")
        );
    }
}
