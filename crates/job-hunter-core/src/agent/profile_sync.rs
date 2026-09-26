//! Keeps the candidate's own profiles on job sites (LinkedIn, Naukri, ...) in
//! line with the desktop profile, so applications don't stop to ask for
//! details the site already wants.
//!
//! Nothing is published before the user confirmed which projects to feature
//! (the publishing plan) and ran one update on a site themselves. After that,
//! [`auto_sync_tick`] re-runs a site's update whenever the content it would
//! publish changes, once the agent is idle.

use std::sync::Arc;

use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::steps::{self, StepCtx};
use crate::claude::{protocol::noop_sink, ClaudeRequest};
use crate::context::AppContext;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::prompts;
use crate::util::{new_id, now, slugify};

/// How long the profile must stay unedited before an automatic update runs,
/// so a burst of edits leads to one update rather than several.
pub const AUTO_SYNC_QUIET: Duration = Duration::minutes(3);

/// A platform's stored state plus whether its content is behind the desktop.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformProfileView {
    pub profile: PlatformProfile,
    /// Synced before, and the desktop profile has changed since.
    pub out_of_date: bool,
}

// ---------------------------------------------------------------------------
// What gets published
// ---------------------------------------------------------------------------

/// The featured projects, in the chosen order.
pub fn featured_projects<'a>(truth: &'a CandidateTruth, plan: &PublishingPlan) -> Vec<&'a Project> {
    plan.featured_project_ids
        .iter()
        .filter_map(|id| truth.projects.iter().find(|p| &p.id == id))
        .collect()
}

fn dedup_ci(items: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let item = item.trim().to_string();
        if !item.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&item)) {
            out.push(item);
        }
    }
    out
}

/// Everything a job-site profile is filled from. Only stable facts go in (no
/// ids, timestamps or "years of experience" that grow by themselves), so its
/// [`content_hash`] changes exactly when there is something new to publish.
pub fn platform_content(truth: &CandidateTruth, plan: &PublishingPlan) -> Value {
    let p = &truth.profile;
    let employer = |id: &Option<String>| {
        id.as_ref()
            .and_then(|id| truth.experiences.iter().find(|e| &e.id == id))
            .map(|e| e.company.clone())
    };
    json!({
        "personal": {
            "name": p.personal.name,
            "email": p.personal.email,
            "phone": p.personal.phone,
            "location": p.personal.location,
            "linkedin": p.personal.linkedin,
            "github": p.personal.github,
            "portfolio": p.personal.portfolio,
        },
        "headline": p.personal.current_title,
        "summary": p.summary,
        "skills": dedup_ci(p.skills.all()),
        "experiences": truth.experiences.iter().map(|e| json!({
            "company": e.company,
            "role": e.role,
            "location": e.location,
            "startDate": e.start_date,
            "endDate": e.end_date,
            "description": e.description,
            "technologies": e.technologies,
            "achievements": e.achievements,
        })).collect::<Vec<_>>(),
        "projects": featured_projects(truth, plan).into_iter().map(|pr| json!({
            "name": pr.name,
            "description": pr.description,
            "role": pr.role,
            "employer": employer(&pr.experience_id),
            "technologies": pr.technologies,
            "responsibilities": pr.responsibilities,
            "achievements": pr.achievements,
            "url": pr.url,
        })).collect::<Vec<_>>(),
        "education": p.education.iter().map(|e| json!({
            "institution": e.institution,
            "degree": e.degree,
            "field": e.field,
            "startDate": e.start_date,
            "endDate": e.end_date,
            "grade": e.grade,
            "description": e.description,
        })).collect::<Vec<_>>(),
        "certifications": p.certifications.iter().map(|c| json!({
            "name": c.name, "issuer": c.issuer, "date": c.date, "url": c.url,
        })).collect::<Vec<_>>(),
        "languages": p.languages,
        "careerPreferences": {
            "targetRoles": p.preferences.target_roles,
            "preferredLocations": p.preferences.preferred_locations,
            "remotePreference": p.preferences.remote_preference,
            "relocation": p.preferences.relocation,
            "employmentTypes": p.preferences.employment_types,
            "noticePeriod": p.preferences.notice_period,
            "expectedSalary": p.preferences.salary,
        },
    })
}

/// Stable fingerprint (FNV-1a, 64 bit) of the content, comparable across
/// app versions (unlike `std`'s hasher).
pub fn content_hash(content: &Value) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in content.to_string().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

// ---------------------------------------------------------------------------
// Publishing plan (which projects to feature)
// ---------------------------------------------------------------------------

/// The saved plan, or an empty unconfirmed one.
pub fn publishing_plan(app: &AppContext) -> CoreResult<PublishingPlan> {
    Ok(app
        .store
        .get::<PublishingPlan>(PublishingPlan::ID)?
        .unwrap_or_else(|| PublishingPlan::new(&app.user_id())))
}

/// The user clicked Proceed: store their choice of projects.
pub fn save_publishing_plan(
    app: &AppContext,
    featured_project_ids: Vec<String>,
    picks: Vec<ProjectPick>,
) -> CoreResult<PublishingPlan> {
    let projects = app.projects()?;
    let mut ids: Vec<String> = Vec::new();
    for id in featured_project_ids {
        if !projects.iter().any(|p| p.id == id) {
            return Err(CoreError::Validation(format!("unknown project id {id}")));
        }
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let mut plan = publishing_plan(app)?;
    plan.featured_project_ids = ids;
    plan.picks = normalize_picks(picks, &projects);
    plan.confirmed_at = Some(now());
    plan.updated_at = now();
    app.store.put(&plan)?;
    Ok(plan)
}

/// One pick per existing project: unknown ids and repeats are dropped, and
/// projects Claude skipped are added as not selected.
pub fn normalize_picks(picks: Vec<ProjectPick>, projects: &[Project]) -> Vec<ProjectPick> {
    let mut out: Vec<ProjectPick> = Vec::new();
    for pick in picks {
        if projects.iter().any(|p| p.id == pick.project_id)
            && !out.iter().any(|o| o.project_id == pick.project_id)
        {
            out.push(pick);
        }
    }
    for p in projects {
        if !out.iter().any(|o| o.project_id == p.id) {
            out.push(ProjectPick {
                project_id: p.id.clone(),
                selected: false,
                reason: String::new(),
            });
        }
    }
    out
}

/// Ask Claude which projects to feature. Nothing is saved: the user reviews
/// the suggestion and saves it with [`save_publishing_plan`].
pub async fn suggest_project_picks(app: &Arc<AppContext>) -> CoreResult<Vec<ProjectPick>> {
    let truth = app.candidate_truth().await?;
    if truth.projects.is_empty() {
        return Ok(vec![]);
    }
    let runner = app.runner().await?;
    let run_dir = app.paths.run_dir("project-selection");
    std::fs::create_dir_all(&run_dir)?;
    let settings = app.settings().await;
    let mut req = ClaudeRequest::new(
        "select_projects",
        prompts::project_selection_prompt(&truth),
        run_dir.clone(),
    );
    req.system_prompt_file = Some(prompts::write_system_prompt(&run_dir)?);
    req.json_schema = Some(prompts::schemas::project_selection());
    req.model = settings.claude.model.clone();
    req.max_budget_usd = settings.claude.max_budget_usd_per_call;
    req.max_turns = 4;
    req.mock_context =
        json!({ "projectIds": truth.projects.iter().map(|p| p.id.clone()).collect::<Vec<_>>() });
    let resp = runner
        .run(req, noop_sink(), tokio_util::sync::CancellationToken::new())
        .await?;
    #[derive(Deserialize)]
    struct Out {
        picks: Vec<ProjectPick>,
    }
    let out: Out = steps::structured(&resp, "project selection")?;
    Ok(normalize_picks(out.picks, &truth.projects))
}

// ---------------------------------------------------------------------------
// Platform state
// ---------------------------------------------------------------------------

fn require_platform(name: &str) -> CoreResult<&'static str> {
    canonical_platform(name).ok_or_else(|| {
        CoreError::Validation(format!(
            "Job Hunter can't update profiles on {name}. Supported: {}",
            PROFILE_PLATFORMS.join(", ")
        ))
    })
}

fn platform_record(app: &AppContext, platform: &str) -> CoreResult<PlatformProfile> {
    Ok(app
        .store
        .get::<PlatformProfile>(&slugify(platform))?
        .unwrap_or_else(|| PlatformProfile::new(&app.user_id(), platform)))
}

/// Hash of what would be published right now, if the plan was confirmed.
async fn current_hash(app: &AppContext) -> CoreResult<Option<String>> {
    let plan = publishing_plan(app)?;
    if plan.confirmed_at.is_none() {
        return Ok(None);
    }
    let truth = app.candidate_truth().await?;
    Ok(Some(content_hash(&platform_content(&truth, &plan))))
}

/// Every supported platform, synced or not.
pub async fn platform_profiles(app: &AppContext) -> CoreResult<Vec<PlatformProfileView>> {
    let hash = current_hash(app).await?;
    PROFILE_PLATFORMS
        .iter()
        .map(|platform| {
            let profile = platform_record(app, platform)?;
            let out_of_date =
                profile.synced_hash.is_some() && hash.is_some() && profile.synced_hash != hash;
            Ok(PlatformProfileView {
                profile,
                out_of_date,
            })
        })
        .collect()
}

pub fn set_auto_sync(
    app: &AppContext,
    platform: &str,
    enabled: bool,
) -> CoreResult<PlatformProfile> {
    let platform = require_platform(platform)?;
    let mut record = platform_record(app, platform)?;
    record.auto_sync = enabled;
    record.updated_at = now();
    app.store.put(&record)?;
    Ok(record)
}

// ---------------------------------------------------------------------------
// The update run
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncOut {
    outcome: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    profile_url: String,
    #[serde(default)]
    changes: Vec<String>,
    #[serde(default)]
    skipped: Vec<String>,
    #[serde(default)]
    projects_on_site: Vec<String>,
    #[serde(default)]
    unknown_questions: Vec<PendingQuestion>,
}

/// Update the candidate's profile on one site, in the background. Needs a
/// confirmed publishing plan: the user decides what gets published first.
pub async fn start_profile_sync(
    app: Arc<AppContext>,
    platform: &str,
    previously_answered: Vec<ApplicationAnswer>,
) -> CoreResult<AgentRun> {
    let platform = require_platform(platform)?;
    if publishing_plan(&app)?.confirmed_at.is_none() {
        return Err(CoreError::Validation(
            "Choose which projects to show on your profiles first.".into(),
        ));
    }
    let mut run = AgentRun::new(&app.user_id(), RunKind::ProfileSync, app.is_mock().await);
    run.sources = vec![platform.to_string()];
    let cancel = app.agent.begin(&run).await?;
    run.updated_at = now();
    app.store.put(&run)?;
    let run_id = run.id.clone();
    let app2 = app.clone();
    let task = tokio::spawn(async move {
        let step = StepCtx {
            app: app2.clone(),
            run_id: run_id.clone(),
            cancel,
        };
        let result = sync_platform(&step, platform, &previously_answered).await;
        if let Err(e) = &result {
            // Whatever stopped the run, the card must not stay on "Syncing".
            if let Ok(mut record) = platform_record(&app2, platform) {
                record.status = PlatformSyncStatus::Failed;
                record.message = match e {
                    CoreError::Cancelled => "Stopped before it finished.".into(),
                    other => other.user_message(),
                };
                record.updated_at = now();
                let _ = app2.store.put(&record);
            }
        }
        let mut run: AgentRun = app2
            .store
            .get(&run_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| AgentRun::new(&app2.user_id(), RunKind::ProfileSync, false));
        run.finished_at = Some(now());
        run.stats = app2.agent.status().await.stats;
        let (state, error) = match result {
            Ok(PlatformSyncStatus::Failed) => {
                let message = platform_record(&app2, platform)
                    .map(|r| r.message)
                    .unwrap_or_default();
                (AgentState::Failed, Some(message))
            }
            Ok(_) | Err(CoreError::Cancelled) => (AgentState::Completed, None),
            Err(e) => (AgentState::Failed, Some(e.user_message())),
        };
        run.state = state;
        run.error = error.clone();
        app2.agent.finish(state, error, &run_id).await;
        run.updated_at = now();
        let _ = app2.store.put(&run);
    });
    app.agent.set_task(task).await;
    Ok(run)
}

async fn sync_platform(
    step: &StepCtx,
    platform: &'static str,
    previously_answered: &[ApplicationAnswer],
) -> CoreResult<PlatformSyncStatus> {
    let app = &step.app;
    let pre = steps::preflight(step, true).await?;
    app.agent
        .transition(AgentState::UpdatingProfile, &step.run_id)
        .await?;
    let plan = publishing_plan(app)?;
    let content = platform_content(&pre.truth, &plan);
    let hash = content_hash(&content);
    let project_names: Vec<String> = featured_projects(&pre.truth, &plan)
        .iter()
        .map(|p| p.name.clone())
        .collect();

    let mut record = platform_record(app, platform)?;
    let remove_projects: Vec<String> = record
        .synced_project_names
        .iter()
        .filter(|n| !project_names.iter().any(|p| p.eq_ignore_ascii_case(n)))
        .cloned()
        .collect();
    record.status = PlatformSyncStatus::Syncing;
    record.last_attempt_at = Some(now());
    record.run_id = Some(step.run_id.clone());
    record.message.clear();
    record.pending_questions.clear();
    record.updated_at = now();
    app.store.put(&record)?;

    step.activity(&format!("Updating your {platform} profile"))
        .await;
    step.event_with(
        EventLevel::Info,
        "STEP_STARTED",
        format!("Updating your {platform} profile"),
        json!({ "platform": platform }),
    );

    let answers = app.list_answers()?;
    let resume_path = pre
        .truth
        .profile
        .master_resume
        .as_ref()
        .map(|m| m.stored_path.clone());
    let settings = app.settings().await;
    let mut req = step
        .request(
            &format!("profile_sync:{}", slugify(platform)),
            prompts::profile_sync_prompt(&prompts::ProfileSyncParams {
                platform,
                profile: &content,
                remove_projects: &remove_projects,
                known_answers: &answers,
                previously_answered,
                resume_path: resume_path.as_deref(),
            }),
        )
        .await?;
    req.chrome = true;
    req.allowed_tools = prompts::chrome_tools(true);
    req.json_schema = Some(prompts::schemas::profile_sync());
    // A full profile touches many sections, each with its own editor.
    req.max_turns = settings.claude.max_turns_browser.saturating_mul(2);
    if req.max_budget_usd > 0.0 {
        req.max_budget_usd *= 2.0;
    }
    req.mock_context = json!({
        "platform": platform,
        "projects": project_names,
        "knownQuestions": answers.iter().map(|a| a.question.clone())
            .chain(previously_answered.iter().map(|a| a.question.clone()))
            .collect::<Vec<_>>(),
    });
    let resp = step.run_claude(req).await?;
    let out: SyncOut = steps::structured(&resp, "profile update result")?;

    record.changes = out.changes;
    record.skipped = out.skipped;
    record.message = out.reason;
    if !out.profile_url.trim().is_empty() {
        record.profile_url = out.profile_url.trim().to_string();
    }
    record.status = match out.outcome.as_str() {
        "UPDATED" => {
            record.synced_hash = Some(hash);
            record.last_synced_at = Some(now());
            record.synced_project_names = if out.projects_on_site.is_empty() {
                project_names
            } else {
                out.projects_on_site
            };
            PlatformSyncStatus::Synced
        }
        "HUMAN_INPUT_REQUIRED" => {
            record.pending_questions = out
                .unknown_questions
                .into_iter()
                .map(|mut q| {
                    if q.id.is_empty() {
                        q.id = new_id();
                    }
                    q
                })
                .collect();
            PlatformSyncStatus::NeedsInput
        }
        "MANUAL_ACTION_REQUIRED" => PlatformSyncStatus::ManualActionRequired,
        _ => PlatformSyncStatus::Failed,
    };
    record.updated_at = now();
    app.store.put(&record)?;

    let (level, message) = match record.status {
        PlatformSyncStatus::Synced => (
            EventLevel::Success,
            format!(
                "{platform} profile updated ({} change{})",
                record.changes.len(),
                if record.changes.len() == 1 { "" } else { "s" }
            ),
        ),
        PlatformSyncStatus::NeedsInput => (
            EventLevel::Warn,
            format!(
                "{platform} needs {} answer{} from you to finish your profile",
                record.pending_questions.len(),
                if record.pending_questions.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ),
        ),
        _ => (
            EventLevel::Warn,
            format!("{platform} profile not updated: {}", record.message),
        ),
    };
    step.event_with(
        level,
        "PROFILE_UPDATED",
        message,
        json!({ "platform": platform, "status": record.status }),
    );
    Ok(record.status)
}

/// The user answered the questions a site asked: remember the answers (they
/// also help job applications) and run the update again.
pub async fn answer_platform_questions(
    app: Arc<AppContext>,
    platform: &str,
    answers: Vec<ApplicationAnswer>,
) -> CoreResult<AgentRun> {
    let platform = require_platform(platform)?;
    let mut given = Vec::new();
    for mut a in answers {
        if a.answer.trim().is_empty() {
            continue;
        }
        a.source = AnswerSource::User;
        let existing = app.store.list::<AnswerRecord>()?;
        match find_answer(&existing, &a.question).cloned() {
            Some(mut rec) => {
                rec.answer = a.answer.clone();
                rec.updated_at = now();
                app.store.put(&rec)?;
            }
            None => {
                app.store.put(&AnswerRecord::new(
                    &app.user_id(),
                    &a.question,
                    &a.answer,
                    "profile",
                ))?;
            }
        }
        given.push(a);
    }
    if given.is_empty() {
        return Err(CoreError::Validation("Answer at least one question".into()));
    }
    start_profile_sync(app, platform, given).await
}

// ---------------------------------------------------------------------------
// Automatic updates
// ---------------------------------------------------------------------------

/// Start an update for the first site that has auto-sync on, was updated
/// successfully before, and is now behind the desktop profile. Waits until
/// the agent is idle and the profile has been left alone for `quiet`.
pub async fn auto_sync_tick(
    app: &Arc<AppContext>,
    quiet: Duration,
) -> CoreResult<Option<AgentRun>> {
    if app.agent.is_busy().await {
        return Ok(None);
    }
    let plan = publishing_plan(app)?;
    if plan.confirmed_at.is_none() {
        return Ok(None);
    }
    let truth = app.candidate_truth().await?;
    if !truth.profile.completeness().ready {
        return Ok(None);
    }
    let last_edit = truth
        .experiences
        .iter()
        .map(|e| e.updated_at)
        .chain(truth.projects.iter().map(|p| p.updated_at))
        .chain([truth.profile.updated_at, plan.updated_at])
        .max()
        .unwrap_or_else(now);
    if now() - last_edit < quiet {
        return Ok(None);
    }
    let hash = content_hash(&platform_content(&truth, &plan));
    for platform in PROFILE_PLATFORMS {
        let Some(record) = app.store.get::<PlatformProfile>(&slugify(platform))? else {
            continue;
        };
        if record.auto_sync
            && record.status == PlatformSyncStatus::Synced
            && record.synced_hash.as_deref() != Some(hash.as_str())
        {
            tracing::info!(platform, "profile changed; updating the job-site profile");
            return start_profile_sync(app.clone(), platform, vec![])
                .await
                .map(Some);
        }
    }
    Ok(None)
}

/// Check for sites to update automatically, once a minute, for the life of
/// the app. The host spawns this on its runtime.
pub async fn run_auto_sync(app: Arc<AppContext>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Err(e) = auto_sync_tick(&app, AUTO_SYNC_QUIET).await {
            tracing::warn!(error = %e, "automatic profile update check failed");
        }
    }
}
