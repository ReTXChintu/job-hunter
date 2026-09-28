//! Individual pipeline steps. Each is a plain async function over the app
//! context so it can be run by the orchestrator or triggered individually
//! from the UI (analyze one job, regenerate one resume, ...).

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::claude::{ClaudeEvent, ClaudeRequest, ClaudeResponse};
use crate::context::AppContext;
use crate::dedup;
use crate::documents::render::{
    document_file_stem, render_cover_letter_bundle, render_resume_bundle, RenderOptions,
};
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::prompts;
use crate::util::{looks_like_email, now, slugify, truncate};

pub struct StepCtx {
    pub app: Arc<AppContext>,
    pub run_id: String,
    pub cancel: CancellationToken,
    /// Session id of the latest Claude run, known as soon as it starts, so a
    /// run that fails or times out can still be resumed.
    last_session: Arc<std::sync::Mutex<Option<String>>>,
}

impl StepCtx {
    pub fn new(app: Arc<AppContext>, run_id: &str, cancel: CancellationToken) -> Self {
        Self {
            app,
            run_id: run_id.into(),
            cancel,
            last_session: Default::default(),
        }
    }

    /// The session of the most recent `run_claude`, even when it failed.
    pub fn last_session(&self) -> Option<String> {
        self.last_session.lock().unwrap().clone()
    }

    pub fn run_dir(&self) -> PathBuf {
        self.app.paths.run_dir(&self.run_id)
    }

    pub fn event(&self, level: EventLevel, kind: &str, message: impl Into<String>) {
        self.app
            .emit_event(AgentEvent::new(&self.run_id, level, kind, message));
    }

    pub fn event_with(
        &self,
        level: EventLevel,
        kind: &str,
        message: impl Into<String>,
        data: Value,
    ) {
        self.app
            .emit_event(AgentEvent::new(&self.run_id, level, kind, message).with_data(data));
    }

    pub async fn activity(&self, text: &str) {
        let text = text.to_string();
        self.app
            .agent
            .update(|s| s.current_activity = Some(text))
            .await;
    }

    pub async fn progress(&self, percent: u8) {
        self.app
            .agent
            .update(|s| s.progress = Some(percent.min(100)))
            .await;
    }

    /// Build a request pre-configured from settings.
    pub async fn request(&self, label: &str, prompt: String) -> CoreResult<ClaudeRequest> {
        let settings = self.app.settings().await;
        let run_dir = self.run_dir();
        std::fs::create_dir_all(&run_dir)?;
        let mut req = ClaudeRequest::new(label, prompt, run_dir.clone());
        req.system_prompt_file = Some(prompts::write_system_prompt(&run_dir)?);
        req.model = settings.claude.model.clone();
        req.max_budget_usd = settings.claude.max_budget_usd_per_call;
        req.timeout = std::time::Duration::from_secs(settings.claude.timeout_seconds.max(60));
        Ok(req)
    }

    /// Run Claude, streaming activity into the event feed.
    pub async fn run_claude(&self, req: ClaudeRequest) -> CoreResult<ClaudeResponse> {
        let runner = self.app.runner().await?;
        let app = self.app.clone();
        let run_id = self.run_id.clone();
        let label = req.label.clone();
        *self.last_session.lock().unwrap() = None;
        let last_session = self.last_session.clone();
        let sink: crate::claude::EventSink = Arc::new(move |ev: ClaudeEvent| {
            if let ClaudeEvent::Init { session_id, .. } = &ev {
                *last_session.lock().unwrap() = Some(session_id.clone());
            }
            let (level, message) = match &ev {
                ClaudeEvent::Init { .. } => (
                    EventLevel::Info,
                    format!("Claude session started ({label})"),
                ),
                ClaudeEvent::Text { text } => {
                    (EventLevel::Info, format!("Claude: {}", truncate(text, 240)))
                }
                ClaudeEvent::ToolUse { summary, .. } => (EventLevel::Info, summary.clone()),
                ClaudeEvent::ToolResult { ok, summary } => {
                    if *ok {
                        return;
                    }
                    (
                        EventLevel::Warn,
                        format!("Tool error: {}", truncate(summary, 200)),
                    )
                }
                ClaudeEvent::Result { subtype, is_error } => {
                    if *is_error {
                        (EventLevel::Warn, format!("Claude ended with {subtype}"))
                    } else {
                        return;
                    }
                }
                ClaudeEvent::Stderr { text } => {
                    let lower = text.to_lowercase();
                    if lower.contains("error") || lower.contains("warn") {
                        (EventLevel::Warn, truncate(text, 240))
                    } else {
                        return;
                    }
                }
            };
            let event = AgentEvent::new(&run_id, level, "CLAUDE_ACTIVITY", message);
            app.agent.bus.emit(event);
        });
        let response = runner.run(req, sink, self.cancel.clone()).await?;
        if response.cost_usd > 0.0 {
            let cost = response.cost_usd;
            self.app
                .agent
                .update(|s| s.stats.claude_cost_usd += cost)
                .await;
        }
        Ok(response)
    }
}

pub(crate) fn structured<T: serde::de::DeserializeOwned>(
    resp: &ClaudeResponse,
    what: &str,
) -> CoreResult<T> {
    let value = resp
        .structured
        .clone()
        .ok_or_else(|| CoreError::ClaudeRunFailed {
            message: format!("Claude returned no structured {what}"),
            details: truncate(&resp.text, 600),
        })?;
    serde_json::from_value(value).map_err(|e| CoreError::ClaudeRunFailed {
        message: format!("could not decode {what}"),
        details: e.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Preflight
// ---------------------------------------------------------------------------

pub struct Preflight {
    pub truth: CandidateTruth,
    pub sources: Vec<String>,
    pub mock: bool,
}

pub async fn preflight(step: &StepCtx, needs_browser: bool) -> CoreResult<Preflight> {
    step.activity("Checking Claude CLI").await;
    let runner = step.app.runner().await?;
    let mock = runner.is_mock();
    if !mock {
        let cli = step.app.claude_cli().await?;
        let status = cli.status().await;
        if !status.installed {
            return Err(CoreError::ClaudeCliUnavailable {
                details: status.error.unwrap_or_default(),
            });
        }
        if !status.authenticated {
            return Err(CoreError::ClaudeNotAuthenticated {
                details: status
                    .error
                    .unwrap_or_else(|| "claude auth status reports not logged in".into()),
            });
        }
        step.event(
            EventLevel::Success,
            "STEP_DONE",
            format!(
                "Claude CLI {} ready ({})",
                status.version.unwrap_or_default(),
                status.account.unwrap_or_else(|| "signed in".into())
            ),
        );
        if needs_browser {
            step.activity("Checking Chrome").await;
            let chrome = step.app.chrome_status().await;
            if !chrome.installed {
                return Err(CoreError::ChromeUnavailable {
                    details: chrome.error.unwrap_or_default(),
                });
            }
            if !chrome.extension_installed {
                return Err(CoreError::ClaudeInChromeUnavailable {
                    details: format!(
                        "extension not found in profiles {:?}",
                        chrome.profiles_checked
                    ),
                });
            }
            step.event(
                EventLevel::Success,
                "STEP_DONE",
                format!(
                    "Chrome {} with Claude in Chrome {} found",
                    chrome.version.unwrap_or_default(),
                    chrome.extension_version.unwrap_or_default()
                ),
            );
        }
    } else {
        step.event(EventLevel::Warn, "MESSAGE", "Mock mode: Claude and Chrome are simulated; nothing real will be searched or submitted");
    }
    step.activity("Loading candidate profile").await;
    let truth = step.app.candidate_truth().await?;
    let completeness = truth.profile.completeness();
    if !completeness.ready {
        return Err(CoreError::Validation(format!(
            "The candidate profile is incomplete. Missing: {}",
            completeness.missing.join(", ")
        )));
    }
    step.event(
        EventLevel::Success,
        "STEP_DONE",
        format!(
            "Loaded candidate profile for {}",
            truth.profile.personal.name
        ),
    );
    let sources = step.app.settings().await.enabled_sources();
    if sources.is_empty() {
        return Err(CoreError::Validation(
            "No job sources are enabled. Enable at least one in Settings.".into(),
        ));
    }
    Ok(Preflight {
        truth,
        sources,
        mock,
    })
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Upper bound on search phrases per source, to keep one discovery run within
/// its turn and budget limits. Target roles beyond it are dropped, not merged.
pub const MAX_QUERIES: usize = 10;

/// One search phrase per target role (all of them, up to [`MAX_QUERIES`]),
/// falling back to the current title when no target roles are set.
pub fn build_queries(profile: &CandidateProfile) -> Vec<String> {
    let mut q: Vec<String> = Vec::new();
    for role in &profile.preferences.target_roles {
        let role = role.trim();
        if !role.is_empty() && !q.iter().any(|r| r.eq_ignore_ascii_case(role)) {
            q.push(role.to_string());
        }
    }
    if q.is_empty() && !profile.personal.current_title.trim().is_empty() {
        q.push(profile.personal.current_title.trim().to_string());
    }
    q.truncate(MAX_QUERIES);
    q
}

pub async fn discover_source(
    step: &StepCtx,
    truth: &CandidateTruth,
    source: &str,
) -> CoreResult<Vec<Job>> {
    let settings = step.app.settings().await;
    let profile = &truth.profile;
    let queries = build_queries(profile);
    let seen: Vec<String> = step
        .app
        .store
        .list::<Job>()?
        .into_iter()
        .map(|j| j.url)
        .collect();
    let remote = format!("{:?}", profile.preferences.remote_preference).to_uppercase();
    let params = prompts::DiscoveryParams {
        source,
        queries: &queries,
        locations: &profile.preferences.preferred_locations,
        remote_preference: &remote,
        recency_days: settings.recency_days,
        max_jobs: settings.max_jobs_per_source,
        seen_urls: &seen,
        careers_url: None,
    };
    let mut req = step
        .request(
            &format!("discover:{}", slugify(source)),
            prompts::discovery_prompt(&params),
        )
        .await?;
    req.chrome = true;
    req.allowed_tools = prompts::chrome_tools(false);
    req.json_schema = Some(prompts::schemas::discovery());
    req.max_turns = settings.claude.max_turns_browser;
    req.mock_context =
        json!({ "source": source, "maxJobs": settings.max_jobs_per_source, "seenUrls": seen });
    let resp = step.run_claude(req).await?;
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DiscoveryOut {
        #[serde(default)]
        jobs: Vec<DiscoveredJob>,
        #[serde(default)]
        notes: String,
        #[serde(default)]
        blocked: bool,
        #[serde(default)]
        blocked_reason: String,
    }
    let out: DiscoveryOut = structured(&resp, "discovery result")?;
    if out.blocked {
        step.event(
            EventLevel::Warn,
            "MESSAGE",
            format!(
                "{source}: stopped early - {}",
                if out.blocked_reason.is_empty() {
                    "site blocked the search".into()
                } else {
                    out.blocked_reason.clone()
                }
            ),
        );
    }
    if !out.notes.is_empty() {
        tracing::debug!(source, notes = %out.notes, "discovery notes");
    }
    let user_id = profile.user_id.clone();
    let mut jobs = Vec::new();
    for d in out.jobs {
        if d.url.trim().is_empty() || d.title.trim().is_empty() || d.company.trim().is_empty() {
            continue;
        }
        let mut job = Job::new(
            &user_id,
            source,
            d.url.trim(),
            d.company.trim(),
            d.title.trim(),
        );
        job.location = d.location;
        job.posted_at = d.posted_at.filter(|s| !s.is_empty());
        job.source_job_id = d.source_job_id.filter(|s| !s.is_empty());
        job.sources[0].source_job_id = job.source_job_id.clone();
        job.employment_type = d.employment_type.filter(|s| !s.is_empty());
        job.remote = d.remote.filter(|s| !s.is_empty());
        job.salary = d.salary.filter(|s| !s.is_empty());
        job.seniority = d.seniority.filter(|s| !s.is_empty());
        job.description = d.description;
        job.requirements = d.requirements;
        job.responsibilities = d.responsibilities;
        job.skills = d.skills;
        job.details_complete = d.details_complete && !job.description.trim().is_empty();
        job.apply_email = d
            .apply_email
            .map(|e| e.trim().to_string())
            .filter(|e| looks_like_email(e));
        job.contact_name = d.contact_name.filter(|n| !n.trim().is_empty());
        job.run_id = Some(step.run_id.clone());
        dedup::prepare(&mut job);
        jobs.push(job);
    }
    Ok(jobs)
}

pub async fn extract_details(step: &StepCtx, job: &mut Job) -> CoreResult<bool> {
    let settings = step.app.settings().await;
    let mut req = step
        .request(
            &format!("extract:{}", slugify(&job.company)),
            prompts::extraction_prompt(job),
        )
        .await?;
    req.chrome = true;
    req.allowed_tools = prompts::chrome_tools(false);
    req.json_schema = Some(prompts::schemas::extraction());
    req.max_turns = 25;
    req.mock_context = json!({ "job": { "title": job.title, "company": job.company, "url": job.url, "description": job.description } });
    let resp = step.run_claude(req).await?;
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExtractOut {
        found: bool,
        #[serde(default)]
        blocked: bool,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        job: Option<DiscoveredJob>,
    }
    let out: ExtractOut = structured(&resp, "extraction result")?;
    if !out.found || out.blocked {
        step.event(
            EventLevel::Warn,
            "MESSAGE",
            format!(
                "Could not extract {} at {}: {}",
                job.title,
                job.company,
                if out.reason.is_empty() {
                    "page unavailable".into()
                } else {
                    out.reason
                }
            ),
        );
        return Ok(false);
    }
    if let Some(d) = out.job {
        if !d.description.trim().is_empty() {
            job.description = d.description;
        }
        if !d.requirements.is_empty() {
            job.requirements = d.requirements;
        }
        if !d.responsibilities.is_empty() {
            job.responsibilities = d.responsibilities;
        }
        if !d.skills.is_empty() {
            job.skills = d.skills;
        }
        if job.location.is_empty() {
            job.location = d.location;
        }
        job.salary = job.salary.take().or(d.salary.filter(|s| !s.is_empty()));
        job.employment_type = job
            .employment_type
            .take()
            .or(d.employment_type.filter(|s| !s.is_empty()));
        job.remote = job.remote.take().or(d.remote.filter(|s| !s.is_empty()));
        job.seniority = job
            .seniority
            .take()
            .or(d.seniority.filter(|s| !s.is_empty()));
        job.posted_at = job
            .posted_at
            .take()
            .or(d.posted_at.filter(|s| !s.is_empty()));
        job.details_complete = d.details_complete || !job.description.trim().is_empty();
        job.touch();
        let _ = settings;
        return Ok(true);
    }
    Ok(false)
}

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

pub const ANALYSIS_BATCH: usize = 4;

pub async fn analyze_jobs(
    step: &StepCtx,
    truth: &CandidateTruth,
    jobs: &[Job],
) -> CoreResult<Vec<JobAnalysis>> {
    if jobs.is_empty() {
        return Ok(vec![]);
    }
    let mut req = step
        .request("analyze", prompts::analysis_prompt(truth, jobs))
        .await?;
    req.json_schema = Some(prompts::schemas::analysis());
    req.max_turns = 6;
    req.mock_context = json!({
        "candidate": {
            "skills": truth.profile.skills.all().into_iter().chain(truth.experiences.iter().flat_map(|e| e.technologies.clone())).collect::<Vec<_>>(),
            "totalExperienceYears": truth.total_experience_years(),
            "targetRoles": truth.profile.preferences.target_roles,
        },
        "jobs": jobs.iter().map(|j| json!({ "jobId": j.id, "title": j.title, "skills": j.skills, "requirements": j.requirements, "salary": j.salary })).collect::<Vec<_>>(),
    });
    let resp = step.run_claude(req).await?;
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Item {
        job_id: String,
        #[serde(flatten)]
        result: AnalysisResult,
    }
    #[derive(serde::Deserialize)]
    struct Out {
        analyses: Vec<Item>,
    }
    let out: Out = structured(&resp, "analysis")?;
    let mut analyses = Vec::new();
    for item in out.analyses {
        let Some(job) = jobs.iter().find(|j| j.id == item.job_id) else {
            tracing::warn!(job_id = %item.job_id, "analysis for unknown job id ignored");
            continue;
        };
        analyses.push(JobAnalysis::from_result(
            job,
            item.result,
            Some(step.run_id.clone()),
        ));
    }
    Ok(analyses)
}

/// Persist an analysis and update the job's status accordingly.
pub async fn record_analysis(
    step: &StepCtx,
    job: &mut Job,
    analysis: &JobAnalysis,
) -> CoreResult<()> {
    let settings = step.app.settings().await;
    step.app.store.put(analysis)?;
    job.analysis_id = Some(analysis.id.clone());
    if matches!(
        job.status,
        JobStatus::Discovered
            | JobStatus::Analyzed
            | JobStatus::NotRelevant
            | JobStatus::Shortlisted
    ) {
        job.status = if analysis.relevant && analysis.match_score >= settings.minimum_match_score {
            JobStatus::Shortlisted
        } else if analysis.relevant {
            JobStatus::Analyzed
        } else {
            JobStatus::NotRelevant
        };
    }
    job.touch();
    step.app.store.put(job)?;
    step.event_with(
        if analysis.relevant { EventLevel::Success } else { EventLevel::Info },
        "JOB_ANALYZED",
        format!("{} at {}: {}% match{}", job.title, job.company, analysis.match_score, if analysis.relevant { "" } else { " (not relevant)" }),
        json!({ "jobId": job.id, "relevant": analysis.relevant, "matchScore": analysis.match_score }),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Resume generation + validation
// ---------------------------------------------------------------------------

pub struct Generated {
    pub resume: Resume,
    pub cover_letter: Option<CoverLetter>,
    pub notes: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResumeOut {
    resume: ResumeDocument,
    #[serde(default)]
    cover_letter: String,
    #[serde(default)]
    application_notes: Vec<String>,
}

fn candidate_skill_list(truth: &CandidateTruth) -> Vec<String> {
    let mut v = truth.profile.skills.all();
    for e in &truth.experiences {
        v.extend(e.technologies.clone());
    }
    for p in &truth.projects {
        v.extend(p.technologies.clone());
    }
    v
}

pub async fn generate_resume(
    step: &StepCtx,
    truth: &CandidateTruth,
    job: &Job,
    analysis: Option<&JobAnalysis>,
) -> CoreResult<Generated> {
    let settings = step.app.settings().await;
    let include_cl = settings.resume.generate_cover_letter;
    let max_iter = settings.resume.ats_max_iterations.clamp(1, 3);
    let min_cov = settings.resume.minimum_keyword_coverage;
    let candidate_ctx = prompts::candidate_for_prompt(truth, false);
    let keywords: Vec<String> = analysis
        .map(|a| a.important_keywords.clone())
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| job.skills.clone());

    let mut previous: Option<AtsValidation> = None;
    let mut previous_doc: Option<ResumeDocument> = None;
    let mut best: Option<(ResumeOut, AtsValidation)> = None;

    for iteration in 1..=max_iter {
        step.activity(&format!(
            "Generating resume for {} at {} (attempt {iteration})",
            job.title, job.company
        ))
        .await;
        let mut req = step
            .request(
                &format!("generate_resume:{}", slugify(&job.company)),
                prompts::resume_prompt(
                    truth,
                    job,
                    analysis,
                    include_cl,
                    previous.as_ref(),
                    previous_doc.as_ref(),
                ),
            )
            .await?;
        req.json_schema = Some(prompts::schemas::resume());
        req.max_turns = 6;
        req.mock_context = json!({ "candidate": candidate_ctx, "job": { "title": job.title, "company": job.company, "skills": job.skills }, "includeCoverLetter": include_cl });
        let resp = step.run_claude(req).await?;
        let out: ResumeOut = structured(&resp, "resume")?;

        step.activity(&format!(
            "Validating resume for {} (attempt {iteration})",
            job.company
        ))
        .await;
        let mut vreq = step
            .request(
                &format!("validate_resume:{}", slugify(&job.company)),
                prompts::validation_prompt(truth, job, analysis, &out.resume, iteration, min_cov),
            )
            .await?;
        vreq.json_schema = Some(prompts::schemas::validation());
        vreq.max_turns = 4;
        vreq.mock_context = json!({ "resume": out.resume, "candidateSkills": candidate_skill_list(truth), "keywords": keywords });
        let vresp = step.run_claude(vreq).await?;
        let mut validation: AtsValidation = structured(&vresp, "validation")?;
        validation.iteration = iteration;
        step.event_with(
            match validation.status {
                AtsStatus::Pass => EventLevel::Success,
                AtsStatus::Revise => EventLevel::Warn,
                AtsStatus::Fail => EventLevel::Error,
            },
            "MESSAGE",
            format!(
                "ATS validation {:?} ({}% keyword coverage, {} unsupported claims)",
                validation.status,
                validation.keyword_coverage,
                validation.unsupported_claims.len()
            ),
            json!({ "jobId": job.id, "iteration": iteration }),
        );
        let pass = validation.status == AtsStatus::Pass && validation.unsupported_claims.is_empty();
        let better = best
            .as_ref()
            .map(|(_, v)| {
                validation.unsupported_claims.len() < v.unsupported_claims.len()
                    || (validation.unsupported_claims.len() == v.unsupported_claims.len()
                        && validation.keyword_coverage >= v.keyword_coverage)
            })
            .unwrap_or(true);
        if better {
            best = Some((out, validation.clone()));
        }
        if pass || validation.status == AtsStatus::Fail {
            break;
        }
        previous = Some(validation);
        previous_doc = best.as_ref().map(|(o, _)| o.resume.clone());
    }

    let (out, validation) =
        best.ok_or_else(|| CoreError::DocumentGeneration("no resume was produced".into()))?;
    if !validation.unsupported_claims.is_empty() {
        step.event(EventLevel::Warn, "MESSAGE", format!("Resume for {} still has unsupported claims after {} attempts; review before applying: {}", job.company, max_iter, validation.unsupported_claims.join("; ")));
    }

    // Persist + render
    let existing = step
        .app
        .store
        .find::<Resume>(|r| r.job_id.as_deref() == Some(&job.id))?;
    let version = existing.iter().map(|r| r.version).max().unwrap_or(0) + 1;
    let dir = document_dir(&step.app, &job.company, &job.title, &job.id, version);
    let candidate_name = &truth.profile.personal.name;
    let resume_stem = document_file_stem(candidate_name, "Resume");
    let cover_letter_stem = document_file_stem(candidate_name, "Cover_Letter");
    let chrome = step.app.chrome_path().await;
    let temp = step.app.paths.temp_dir();
    let opts = RenderOptions {
        dir: &dir,
        base_name: &resume_stem,
        chrome: chrome.as_deref(),
        temp_dir: &temp,
        want_pdf: settings.resume.generate_pdf,
        want_docx: settings.resume.generate_docx,
    };
    step.activity(&format!("Rendering resume documents for {}", job.company))
        .await;
    let files = render_resume_bundle(&out.resume, &opts).await?;
    for w in &files.warnings {
        step.event(EventLevel::Warn, "MESSAGE", w.clone());
    }
    let mut resume =
        Resume::new_generated(&job.user_id, &job.id, &job.company, &job.title, version);
    resume.docx_path = files.docx;
    resume.pdf_path = files.pdf;
    resume.html_path = files.html;
    resume.json_path = files.json;
    resume.content = Some(out.resume);
    resume.validation = Some(validation);
    resume.run_id = Some(step.run_id.clone());
    step.app.store.put(&resume)?;

    let cover_letter = if include_cl && !out.cover_letter.trim().is_empty() {
        let cl_opts = RenderOptions {
            base_name: &cover_letter_stem,
            ..opts
        };
        let cl_files = render_cover_letter_bundle(
            out.cover_letter.trim(),
            &format!("Cover letter - {}", job.company),
            &cl_opts,
        )
        .await?;
        let ts = now();
        let cl = CoverLetter {
            id: crate::util::new_id(),
            user_id: job.user_id.clone(),
            job_id: job.id.clone(),
            resume_id: Some(resume.id.clone()),
            version,
            text: out.cover_letter.trim().to_string(),
            docx_path: cl_files.docx,
            pdf_path: cl_files.pdf,
            txt_path: cl_files.txt,
            user_edited: false,
            files_deleted_at: None,
            created_at: ts,
            updated_at: ts,
        };
        step.app.store.put(&cl)?;
        Some(cl)
    } else {
        None
    };
    step.event_with(
        EventLevel::Success,
        "RESUME_GENERATED",
        format!(
            "Resume v{version} ready for {} at {}",
            job.title, job.company
        ),
        json!({ "jobId": job.id, "resumeId": resume.id }),
    );
    Ok(Generated {
        resume,
        cover_letter,
        notes: out.application_notes,
    })
}

/// Folder for one version of one job's documents:
/// `generated/<company>/<title>-<job id>/v<N>`. Per job, since two openings
/// at one company would otherwise share (and overwrite) "v1"; per version,
/// so every file keeps the plain name a recruiter sees ("Biswajit_Resume.pdf").
pub fn document_dir(
    app: &AppContext,
    company: &str,
    title: &str,
    job_id: &str,
    version: u32,
) -> PathBuf {
    let short: String = job_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(6)
        .collect();
    app.paths
        .company_resume_dir(&slugify(company))
        .join(format!("{}-{short}", slugify(title)))
        .join(format!("v{version}"))
}

/// Delete the generated files of an application that has been sent: every
/// resume and cover-letter version for its job. The content stays on the
/// records, so [`restore_application_files`] can re-create them. Returns
/// how many files were removed.
pub fn delete_application_files(app: &AppContext, application: &Application) -> CoreResult<usize> {
    let root = app.paths.generated_dir();
    let mut removed = 0;
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut remove = |path: &mut Option<String>| {
        if let Some(p) = path.take() {
            let p = PathBuf::from(p);
            // Only ever inside the generated-documents folder.
            if crate::util::path_is_inside(&root, &p)
                && p.is_file()
                && std::fs::remove_file(&p).is_ok()
            {
                removed += 1;
            }
            if let Some(parent) = p.parent() {
                if !dirs.iter().any(|d| d == parent) {
                    dirs.push(parent.to_path_buf());
                }
            }
        }
    };
    let ts = now();
    for mut r in app
        .store
        .find::<Resume>(|r| r.job_id.as_deref() == Some(application.job_id.as_str()))?
    {
        if r.pdf_path.is_none()
            && r.docx_path.is_none()
            && r.html_path.is_none()
            && r.json_path.is_none()
        {
            continue;
        }
        remove(&mut r.pdf_path);
        remove(&mut r.docx_path);
        remove(&mut r.html_path);
        remove(&mut r.json_path);
        r.files_deleted_at = Some(ts);
        r.updated_at = ts;
        app.store.put(&r)?;
    }
    for mut c in app
        .store
        .find::<CoverLetter>(|c| c.job_id == application.job_id)?
    {
        if c.pdf_path.is_none() && c.docx_path.is_none() && c.txt_path.is_none() {
            continue;
        }
        remove(&mut c.pdf_path);
        remove(&mut c.docx_path);
        remove(&mut c.txt_path);
        c.files_deleted_at = Some(ts);
        c.updated_at = ts;
        app.store.put(&c)?;
    }
    // The job's own folder (every version, plus the text/HTML copies the
    // records don't track). Only in the per-job layout; files from before it
    // sat in folders shared across jobs and were removed one by one above.
    if let Ok(job) = app.store.require::<Job>(&application.job_id) {
        if let Some(job_dir) = document_dir(app, &job.company, &job.title, &job.id, 1).parent() {
            if job_dir.is_dir() && crate::util::path_is_inside(&root, job_dir) {
                match std::fs::remove_dir_all(job_dir) {
                    Ok(()) => removed += 1,
                    Err(e) => {
                        tracing::warn!(error = %e, dir = %job_dir.display(), "could not remove a job's documents folder")
                    }
                }
                if let Some(company_dir) = job_dir.parent() {
                    dirs.push(company_dir.to_path_buf());
                }
            }
        }
    }
    // Tidy folders left empty (version, job, company), never the root itself.
    let root_canonical = std::fs::canonicalize(&root).ok();
    for dir in dirs {
        let mut current = Some(dir.as_path());
        while let Some(d) = current {
            let is_root = std::fs::canonicalize(d).ok() == root_canonical;
            // remove_dir only removes empty folders.
            if is_root || !crate::util::path_is_inside(&root, d) || std::fs::remove_dir(d).is_err()
            {
                break;
            }
            current = d.parent();
        }
    }
    if removed > 0 {
        tracing::info!(application = %application.id, removed, "deleted generated files after applying");
    }
    Ok(removed)
}

/// Delete an applied application's files if the user wants that (on by default).
pub async fn cleanup_after_applied(app: &AppContext, application: &Application) {
    if application.status != ApplicationStatus::Applied
        || !app.settings().await.resume.delete_files_after_applied
    {
        return;
    }
    if let Err(e) = delete_application_files(app, application) {
        tracing::warn!(error = %e, "could not delete generated files after applying");
    }
}

/// Re-create the files of an application's resume and cover letter from
/// their saved content (after they were deleted on applying).
pub async fn restore_application_files(app: &AppContext, application_id: &str) -> CoreResult<()> {
    let application: Application = app.store.require(application_id)?;
    if let Some(id) = &application.resume_id {
        let mut resume: Resume = app.store.require(id)?;
        let edited = resume.user_edited;
        rerender_resume(app, &mut resume).await?;
        resume.user_edited = edited;
        resume.files_deleted_at = None;
        app.store.put(&resume)?;
    }
    if let Some(id) = &application.cover_letter_id {
        let mut cl: CoverLetter = app.store.require(id)?;
        let job: Job = app.store.require(&cl.job_id)?;
        let edited = cl.user_edited;
        rerender_cover_letter(app, &mut cl, &job.company).await?;
        cl.user_edited = edited;
        cl.files_deleted_at = None;
        app.store.put(&cl)?;
    }
    Ok(())
}

/// Re-render an edited resume document (user edits) into new files.
pub async fn rerender_resume(app: &AppContext, resume: &mut Resume) -> CoreResult<()> {
    let Some(doc) = resume.content.clone() else {
        return Err(CoreError::Validation("resume has no content".into()));
    };
    let settings = app.settings().await;
    let dir = document_dir(
        app,
        &resume.company,
        &resume.job_title,
        resume.job_id.as_deref().unwrap_or(&resume.id),
        resume.version,
    );
    let stem = document_file_stem(&app.profile()?.personal.name, "Resume");
    let chrome = app.chrome_path().await;
    let temp = app.paths.temp_dir();
    let opts = RenderOptions {
        dir: &dir,
        base_name: &stem,
        chrome: chrome.as_deref(),
        temp_dir: &temp,
        want_pdf: settings.resume.generate_pdf,
        want_docx: settings.resume.generate_docx,
    };
    let files = render_resume_bundle(&doc, &opts).await?;
    resume.docx_path = files.docx;
    resume.pdf_path = files.pdf;
    resume.html_path = files.html;
    resume.json_path = files.json;
    resume.user_edited = true;
    resume.updated_at = now();
    app.store.put(resume)?;
    Ok(())
}

pub async fn rerender_cover_letter(
    app: &AppContext,
    cl: &mut CoverLetter,
    company: &str,
) -> CoreResult<()> {
    let settings = app.settings().await;
    let title = app
        .store
        .get::<Job>(&cl.job_id)?
        .map(|j| j.title)
        .unwrap_or_default();
    let dir = document_dir(app, company, &title, &cl.job_id, cl.version);
    let stem = document_file_stem(&app.profile()?.personal.name, "Cover_Letter");
    let chrome = app.chrome_path().await;
    let temp = app.paths.temp_dir();
    let opts = RenderOptions {
        dir: &dir,
        base_name: &stem,
        chrome: chrome.as_deref(),
        temp_dir: &temp,
        want_pdf: settings.resume.generate_pdf,
        want_docx: settings.resume.generate_docx,
    };
    let files =
        render_cover_letter_bundle(&cl.text, &format!("Cover letter - {company}"), &opts).await?;
    cl.docx_path = files.docx;
    cl.pdf_path = files.pdf;
    cl.txt_path = files.txt;
    cl.user_edited = true;
    cl.updated_at = now();
    app.store.put(cl)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Application preparation
// ---------------------------------------------------------------------------

pub fn profile_answers(profile: &CandidateProfile) -> Vec<ApplicationAnswer> {
    let mut v = Vec::new();
    let mut push = |q: &str, a: &str| {
        if !a.trim().is_empty() {
            v.push(ApplicationAnswer {
                question: q.into(),
                answer: a.trim().into(),
                source: AnswerSource::Profile,
            });
        }
    };
    push("Full name", &profile.personal.name);
    push("Email", &profile.personal.email);
    push("Phone", &profile.personal.phone);
    push("Location", &profile.personal.location);
    push("LinkedIn", &profile.personal.linkedin);
    push("GitHub", &profile.personal.github);
    push("Portfolio", &profile.personal.portfolio);
    push("Current title", &profile.personal.current_title);
    push("Notice period", &profile.preferences.notice_period);
    v
}

pub async fn prepare_application(
    step: &StepCtx,
    truth: &CandidateTruth,
    job: &mut Job,
    analysis: Option<&JobAnalysis>,
    generated: &Generated,
) -> CoreResult<Application> {
    let existing = step
        .app
        .store
        .find::<Application>(|a| a.job_id == job.id)?
        .into_iter()
        .next();
    let mut app = match existing {
        Some(a)
            if !matches!(
                a.status,
                ApplicationStatus::Applied
                    | ApplicationStatus::Rejected
                    | ApplicationStatus::Withdrawn
                    | ApplicationStatus::Interview
                    | ApplicationStatus::Offer
            ) =>
        {
            a
        }
        Some(a) => {
            return Err(CoreError::InvalidTransition(format!(
                "application for this job is already {}",
                a.status.as_str()
            )));
        }
        None => Application::new(&job.user_id, &job.id, &job.url, &job.source),
    };
    if app.status == ApplicationStatus::Discovered {
        app.transition(ApplicationStatus::Analyzed, "analysis complete")?;
    }
    app.resume_id = Some(generated.resume.id.clone());
    app.cover_letter_id = generated.cover_letter.as_ref().map(|c| c.id.clone());
    app.run_id = Some(step.run_id.clone());
    let mut answers = profile_answers(&truth.profile);
    let known = step.app.store.list::<AnswerRecord>()?;
    for k in known {
        answers.push(ApplicationAnswer {
            question: k.question,
            answer: k.answer,
            source: AnswerSource::Known,
        });
    }
    app.answers = answers;
    let mut issues: Vec<String> = Vec::new();
    if let Some(a) = analysis {
        issues.extend(a.concerns.clone());
        if !a.missing_skills.is_empty() {
            issues.push(format!(
                "Skills requested but not in your profile: {}",
                a.missing_skills.join(", ")
            ));
        }
        if !a.required_experience_met {
            issues.push("The posting asks for more experience than your records show.".into());
        }
    }
    if let Some(v) = &generated.resume.validation {
        if !v.unsupported_claims.is_empty() {
            issues.push(format!(
                "Resume validator flagged unsupported claims: {}",
                v.unsupported_claims.join("; ")
            ));
        }
        if !v.missing_keywords.is_empty() {
            issues.push(format!(
                "Keywords not covered by the resume: {}",
                v.missing_keywords.join(", ")
            ));
        }
    }
    issues.extend(generated.notes.clone());
    app.potential_issues = issues;
    if app.status != ApplicationStatus::ReadyForReview {
        app.transition(ApplicationStatus::ReadyForReview, "materials generated")?;
    }
    app.updated_at = now();
    step.app.store.put(&app)?;
    job.application_id = Some(app.id.clone());
    job.status = JobStatus::ReadyForReview;
    job.touch();
    step.app.store.put(job)?;
    step.event_with(
        EventLevel::Success,
        "APPLICATION_READY",
        format!("{} at {} is ready for your review", job.title, job.company),
        json!({ "applicationId": app.id, "jobId": job.id }),
    );
    Ok(app)
}

// ---------------------------------------------------------------------------
// Apply (Claude in Chrome)
// ---------------------------------------------------------------------------

pub async fn apply(
    step: &StepCtx,
    truth: &CandidateTruth,
    application: &mut Application,
    job: &mut Job,
    resuming: bool,
    simulate: Option<&str>,
) -> CoreResult<ApplyResult> {
    if !application.is_approved() {
        return Err(CoreError::InvalidTransition(
            "application has not been approved by the user".into(),
        ));
    }
    let settings = step.app.settings().await;
    let resume = match &application.resume_id {
        Some(id) => step.app.store.get::<Resume>(id)?,
        None => None,
    };
    let cover = match &application.cover_letter_id {
        Some(id) => step.app.store.get::<CoverLetter>(id)?,
        None => None,
    };
    let answers = step.app.store.list::<AnswerRecord>()?;
    let previously: Vec<ApplicationAnswer> = application
        .answers
        .iter()
        .filter(|a| a.source == AnswerSource::User)
        .cloned()
        .collect();
    // The browser can only upload files from folders this Claude session may
    // read; without this every resume upload was refused.
    let mut upload_dirs: Vec<PathBuf> = Vec::new();
    for file in [
        resume.as_ref().and_then(|r| r.pdf_path.as_deref()),
        resume.as_ref().and_then(|r| r.docx_path.as_deref()),
        cover.as_ref().and_then(|c| c.pdf_path.as_deref()),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(dir) = std::path::Path::new(file).parent() {
            if !upload_dirs.iter().any(|d| d == dir) {
                upload_dirs.push(dir.to_path_buf());
            }
        }
    }
    let params = prompts::ApplyParams {
        application,
        job,
        truth,
        resume_pdf: resume.as_ref().and_then(|r| r.pdf_path.as_deref()),
        resume_docx: resume.as_ref().and_then(|r| r.docx_path.as_deref()),
        cover_letter_pdf: cover.as_ref().and_then(|c| c.pdf_path.as_deref()),
        cover_letter_text: cover.as_ref().map(|c| c.text.as_str()),
        answers: &answers,
        previously_answered: &previously,
        resuming,
    };
    let mut req = step
        .request(
            &format!("apply:{}", slugify(&job.company)),
            prompts::apply_prompt(&params),
        )
        .await?;
    req.chrome = true;
    req.allowed_tools = prompts::chrome_tools(true);
    req.json_schema = Some(prompts::schemas::apply());
    req.max_turns = settings.claude.max_turns_browser.max(40);
    req.add_dirs = upload_dirs.clone();
    req.resume_session = if resuming {
        application.claude_session_id.clone()
    } else {
        None
    };
    req.mock_context = json!({ "application": { "url": job.url }, "simulate": simulate });
    let resp = match step.run_claude(req).await {
        Ok(r) => r,
        Err(CoreError::ClaudeRunFailed { message, details }) if resuming => {
            // Session may have expired; retry from scratch once.
            tracing::warn!(%message, "resume of application session failed; retrying fresh");
            let params = prompts::ApplyParams {
                resuming: false,
                ..params
            };
            let mut req = step
                .request(
                    &format!("apply:{}", slugify(&job.company)),
                    prompts::apply_prompt(&params),
                )
                .await?;
            req.chrome = true;
            req.allowed_tools = prompts::chrome_tools(true);
            req.json_schema = Some(prompts::schemas::apply());
            req.max_turns = settings.claude.max_turns_browser.max(40);
            req.add_dirs = upload_dirs.clone();
            req.mock_context = json!({ "application": { "url": job.url }, "simulate": simulate });
            let _ = details;
            step.run_claude(req).await?
        }
        Err(e) => return Err(e),
    };
    application.claude_session_id = resp.session_id.clone();
    let result: ApplyResult = structured(&resp, "application result")?;
    Ok(result)
}

/// Apply the outcome of a browser application to the records.
pub fn record_apply_result(
    step: &StepCtx,
    application: &mut Application,
    job: &mut Job,
    result: &ApplyResult,
) -> CoreResult<()> {
    for a in &result.answers_used {
        if !application.answers.iter().any(|x| x.question == a.question) {
            application.answers.push(a.clone());
        }
    }
    if !result.application_url.trim().is_empty() {
        application.application_url = result.application_url.trim().to_string();
    }
    let mut notes: Vec<String> = Vec::new();
    if !result.steps_completed.is_empty() {
        notes.push(format!(
            "Steps completed automatically: {}",
            result.steps_completed.join("; ")
        ));
    }
    match result.outcome {
        ApplyOutcome::Submitted => {
            if result.evidence.trim().is_empty() {
                // No evidence: never record as applied.
                application.failure_reason = Some(
                    "Claude reported a submission without evidence; please verify in the browser."
                        .into(),
                );
                application.transition(
                    ApplicationStatus::ManualActionRequired,
                    "no submission evidence",
                )?;
                job.status = JobStatus::ManualActionRequired;
                step.event_with(
                    EventLevel::Warn,
                    "APPLICATION_UPDATED",
                    format!(
                        "{}: submission could not be verified; please check manually",
                        job.company
                    ),
                    json!({ "applicationId": application.id }),
                );
            } else {
                application.evidence = Some(result.evidence.clone());
                application.failure_reason = None;
                application.pending_questions.clear();
                application.transition(ApplicationStatus::Applied, "submitted with evidence")?;
                job.status = JobStatus::Applied;
                step.event_with(
                    EventLevel::Success,
                    "APPLICATION_UPDATED",
                    format!("Applied to {} at {}", job.title, job.company),
                    json!({ "applicationId": application.id, "status": "APPLIED" }),
                );
            }
        }
        ApplyOutcome::HumanInputRequired => {
            application.pending_questions = result.unknown_questions.clone();
            application.failure_reason = Some(if result.reason.is_empty() {
                "The application form asks questions that need your answers.".into()
            } else {
                result.reason.clone()
            });
            application.transition(ApplicationStatus::WaitingForUser, "questions need answers")?;
            job.status = JobStatus::WaitingForUser;
            step.event_with(
                EventLevel::Warn,
                "APPLICATION_UPDATED",
                format!(
                    "{} needs your input: {} question(s)",
                    job.company,
                    result.unknown_questions.len()
                ),
                json!({ "applicationId": application.id, "status": "WAITING_FOR_USER" }),
            );
        }
        ApplyOutcome::ManualActionRequired | ApplyOutcome::Failed => {
            application.failure_reason = Some(if result.reason.is_empty() {
                "The application could not be completed automatically.".into()
            } else {
                result.reason.clone()
            });
            application.transition(
                ApplicationStatus::ManualActionRequired,
                "automatic application stopped",
            )?;
            job.status = JobStatus::ManualActionRequired;
            step.event_with(
                EventLevel::Warn,
                "APPLICATION_UPDATED",
                format!(
                    "{}: manual action required - {}",
                    job.company,
                    application.failure_reason.clone().unwrap_or_default()
                ),
                json!({ "applicationId": application.id, "status": "MANUAL_ACTION_REQUIRED" }),
            );
        }
    }
    if !notes.is_empty() {
        if !application.notes.is_empty() {
            application.notes.push('\n');
        }
        application.notes.push_str(&notes.join("\n"));
    }
    application.updated_at = now();
    job.touch();
    step.app.store.put(application)?;
    step.app.store.put(job)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_target_role_becomes_a_query() {
        let mut p = CandidateProfile::new("u");
        p.preferences.target_roles = [
            "Full Stack Developer",
            "Technical Team Lead",
            "MERN Stack Developer",
            "Node Js. Developer",
            "Backend Developer",
            " backend developer ",
            "Backend Engineer",
            "",
            "SDE",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            build_queries(&p),
            [
                "Full Stack Developer",
                "Technical Team Lead",
                "MERN Stack Developer",
                "Node Js. Developer",
                "Backend Developer",
                "Backend Engineer",
                "SDE"
            ]
        );
    }

    #[test]
    fn falls_back_to_the_current_title() {
        let mut p = CandidateProfile::new("u");
        p.personal.current_title = "Technical Team Lead".into();
        assert_eq!(build_queries(&p), ["Technical Team Lead"]);
    }
}
