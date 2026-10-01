//! A job the user found themselves and shared: pasted text, screenshots or
//! PDFs. Claude reads it, the usual pipeline analyses it and writes the
//! resume and cover letter, and (optionally) the application email is saved
//! as a Gmail draft with the resume attached, for the user to review and
//! send. Nothing is ever sent from here.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use super::steps::{self, Generated, StepCtx};
use crate::context::AppContext;
use crate::dedup;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::prompts;
use crate::util::{looks_like_email, now, slugify};

/// What can be shared, and how much.
pub const SHARED_FILE_TYPES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "pdf"];
pub const MAX_SHARED_FILES: usize = 10;
pub const MAX_SHARED_FILE_BYTES: u64 = 20 * 1024 * 1024;

pub struct SharedJobInput {
    pub text: String,
    /// Screenshots and PDFs (absolute paths); copied into the run folder.
    pub files: Vec<PathBuf>,
    /// Save the application email as a Gmail draft once the resume is ready.
    pub draft_email: bool,
}

fn validate(input: &SharedJobInput) -> CoreResult<()> {
    if input.text.trim().len() < 30 && input.files.is_empty() {
        return Err(CoreError::Validation(
            "Paste the job description or add a screenshot or PDF of it.".into(),
        ));
    }
    if input.files.len() > MAX_SHARED_FILES {
        return Err(CoreError::Validation(format!(
            "Share at most {MAX_SHARED_FILES} files."
        )));
    }
    for f in &input.files {
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !SHARED_FILE_TYPES.contains(&ext.as_str()) {
            return Err(CoreError::Validation(format!(
                "{} isn't an image or PDF.",
                f.display()
            )));
        }
        let meta = std::fs::metadata(f)
            .map_err(|_| CoreError::Validation(format!("{} can't be read.", f.display())))?;
        if meta.len() > MAX_SHARED_FILE_BYTES {
            return Err(CoreError::Validation(format!(
                "{} is larger than 20 MB.",
                f.display()
            )));
        }
    }
    Ok(())
}

/// Copy the shared files into `dir`, with safe distinct names.
fn copy_files(files: &[PathBuf], dir: &Path) -> CoreResult<Vec<String>> {
    std::fs::create_dir_all(dir)?;
    let mut out = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let stem = f
            .file_stem()
            .map(|s| slugify(&s.to_string_lossy()))
            .unwrap_or_else(|| "file".into());
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let target = dir.join(format!("{:02}-{stem}.{ext}", i + 1));
        std::fs::copy(f, &target)?;
        out.push(target.to_string_lossy().to_string());
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedOut {
    found: bool,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    job: Option<SharedJob>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SharedJob {
    title: String,
    company: String,
    url: String,
    location: String,
    posted_at: String,
    employment_type: String,
    remote: String,
    salary: String,
    seniority: String,
    description: String,
    requirements: Vec<String>,
    responsibilities: Vec<String>,
    skills: Vec<String>,
    apply_email: String,
    contact_name: String,
}

/// Read, analyse and prepare a shared job in the background.
pub async fn start_shared_job(app: Arc<AppContext>, input: SharedJobInput) -> CoreResult<AgentRun> {
    validate(&input)?;
    let mut run = AgentRun::new(&app.user_id(), RunKind::SharedJob, app.is_mock().await);
    run.sources = vec!["Shared".into()];
    let cancel = app.agent.begin(&run).await?;
    let files = copy_files(&input.files, &app.paths.run_dir(&run.id).join("shared"))?;
    run.updated_at = now();
    app.store.put(&run)?;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = StepCtx::new(app2.clone(), &run_id, cancel);
        let result = process(&step, &input.text, &files, input.draft_email).await;
        let mut run: AgentRun = app2
            .store
            .get(&run_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| AgentRun::new(&app2.user_id(), RunKind::SharedJob, false));
        run.finished_at = Some(now());
        run.stats = app2.agent.status().await.stats;
        let (state, error) = match &result {
            Ok(_) | Err(CoreError::Cancelled) => (AgentState::Completed, None),
            Err(e) => {
                app2.notify(
                    Notification::new(
                        &app2.user_id(),
                        EventLevel::Error,
                        "SHARED_JOB_FAILED",
                        "Couldn't prepare the job you shared",
                        e.user_message(),
                    )
                    .link("jobs", None),
                );
                (AgentState::Failed, Some(e.user_message()))
            }
        };
        if let Ok(application) = &result {
            run.application_id = Some(application.id.clone());
        }
        run.state = state;
        run.error = error.clone();
        app2.agent.finish(state, error, &run_id).await;
        run.updated_at = now();
        let _ = app2.store.put(&run);
    });
    app.agent.set_task(task).await;
    Ok(run)
}

async fn process(
    step: &StepCtx,
    text: &str,
    files: &[String],
    draft_email: bool,
) -> CoreResult<Application> {
    let app = &step.app;
    let pre = steps::preflight(step, draft_email).await?;
    app.agent
        .transition(AgentState::Analyzing, &step.run_id)
        .await?;
    step.activity("Reading the job you shared").await;
    let mut req = step
        .request("read_shared_job", prompts::shared_job_prompt(text, files))
        .await?;
    req.json_schema = Some(prompts::schemas::shared_job());
    req.max_turns = 10;
    // Lets Claude's Read tool open the screenshots and PDFs (see runner).
    if !files.is_empty() {
        req.add_dirs = vec![step.run_dir().join("shared")];
    }
    req.mock_context = json!({ "text": text, "files": files });
    let resp = step.run_claude(req).await?;
    let out: SharedOut = steps::structured(&resp, "shared job")?;
    let shared = match out.job {
        Some(j) if out.found && !j.title.trim().is_empty() => j,
        _ => {
            return Err(CoreError::Validation(if out.reason.is_empty() {
                "That doesn't look like a job posting.".into()
            } else {
                out.reason
            }))
        }
    };

    let company = if shared.company.trim().is_empty() {
        "Unknown company".to_string()
    } else {
        shared.company.trim().to_string()
    };
    let url = if shared.url.trim().is_empty() {
        format!("shared://{}", step.run_id)
    } else {
        shared.url.trim().to_string()
    };
    let opt = |s: String| Some(s.trim().to_string()).filter(|s| !s.is_empty());
    let mut job = Job::new(
        &pre.truth.profile.user_id,
        "Shared",
        &url,
        &company,
        shared.title.trim(),
    );
    job.location = shared.location.trim().to_string();
    job.posted_at = opt(shared.posted_at);
    job.employment_type = opt(shared.employment_type);
    job.remote = opt(shared.remote);
    job.salary = opt(shared.salary);
    job.seniority = opt(shared.seniority);
    job.description = shared.description;
    job.requirements = shared.requirements;
    job.responsibilities = shared.responsibilities;
    job.skills = shared.skills;
    job.apply_email = opt(shared.apply_email).filter(|e| looks_like_email(e));
    job.contact_name = opt(shared.contact_name);
    job.details_complete = !job.description.trim().is_empty();
    job.run_id = Some(step.run_id.clone());
    dedup::prepare(&mut job);
    app.store.put(&job)?;
    step.event_with(
        EventLevel::Info,
        "JOB_DISCOVERED",
        format!("{} at {} (shared by you)", job.title, job.company),
        json!({ "jobId": job.id }),
    );

    let mut analyses = steps::analyze_jobs(step, &pre.truth, std::slice::from_ref(&job)).await?;
    let analysis = analyses.pop();
    if let Some(a) = &analysis {
        steps::record_analysis(step, &mut job, a).await?;
    }
    app.agent
        .transition(AgentState::PreparingApplications, &step.run_id)
        .await?;
    let generated = steps::generate_resume(step, &pre.truth, &job, analysis.as_ref()).await?;
    let mut application =
        steps::prepare_application(step, &pre.truth, &mut job, analysis.as_ref(), &generated)
            .await?;

    let drafted = if draft_email {
        save_draft(step, &pre.truth, &job, &mut application, &generated).await?
    } else {
        false
    };
    let at = format!("{} at {}", job.title, job.company);
    app.notify(
        if drafted {
            Notification::new(
                &application.user_id,
                EventLevel::Success,
                "EMAIL_DRAFTED",
                format!("Email draft ready: {at}"),
                match &job.apply_email {
                    Some(to) => format!("In your Gmail Drafts, to {to}, with the resume attached. Review it and send it."),
                    None => "In your Gmail Drafts with the resume attached. The posting had no email address: add the recipient, then send.".into(),
                },
            )
        } else {
            Notification::new(
                &application.user_id,
                EventLevel::Success,
                "SHARED_JOB_READY",
                format!("Resume ready: {at}"),
                "Review the application in Job Hunter.",
            )
        }
        .link("application", Some(&application.id)),
    );
    Ok(application)
}

/// Save the application email as a Gmail draft (never sent). Returns
/// whether Gmail confirmed the draft.
async fn save_draft(
    step: &StepCtx,
    truth: &CandidateTruth,
    job: &Job,
    application: &mut Application,
    generated: &Generated,
) -> CoreResult<bool> {
    step.activity(&format!(
        "Saving the email to {} as a Gmail draft",
        job.company
    ))
    .await;
    let email = prompts::draft_email_message(
        job,
        truth,
        generated.cover_letter.as_ref().map(|c| c.text.as_str()),
    );
    let resume_pdf = generated.resume.pdf_path.clone();
    let settings = step.app.settings().await;
    let mut req = step
        .request(
            &format!("draft_email:{}", slugify(&job.company)),
            prompts::email_draft_prompt(&email, resume_pdf.as_deref()),
        )
        .await?;
    req.chrome = true;
    req.allowed_tools = prompts::chrome_tools(true);
    req.json_schema = Some(prompts::schemas::email_draft());
    req.max_turns = settings.claude.max_turns_browser.max(40);
    req.add_dirs = resume_pdf
        .as_deref()
        .and_then(|p| Path::new(p).parent())
        .map(|d| vec![d.to_path_buf()])
        .unwrap_or_default();
    req.mock_context = json!({ "to": email.to });
    let resp = step.run_claude(req).await?;
    #[derive(Deserialize)]
    struct Out {
        outcome: String,
        #[serde(default)]
        reason: String,
    }
    let out: Out = steps::structured(&resp, "email draft")?;
    let drafted = out.outcome == "DRAFTED";
    if drafted {
        application.email_drafted_at = Some(now());
    }
    if !application.notes.is_empty() {
        application.notes.push('\n');
    }
    application.notes.push_str(&if drafted {
        "Email saved as a Gmail draft; review and send it from Gmail, then mark this as applied."
            .to_string()
    } else {
        format!("Gmail draft not saved: {}", out.reason)
    });
    application.updated_at = now();
    step.app.store.put(application)?;
    step.event(
        if drafted {
            EventLevel::Success
        } else {
            EventLevel::Warn
        },
        "MESSAGE",
        if drafted {
            format!("Email draft for {} saved in Gmail", job.company)
        } else {
            format!("Couldn't save the Gmail draft: {}", out.reason)
        },
    );
    Ok(drafted)
}
