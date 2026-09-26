//! `AppContext` wires configuration, persistence, Claude, Chrome and the agent
//! together and exposes the query/command surface used by the desktop app.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::agent::handle::AgentHandle;
use crate::chrome::{self, ChromeStatus};
use crate::claude::{ClaudeCli, ClaudeRunner, ClaudeStatus, CliClaudeRunner, MockClaudeRunner};
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::logging::LogBuffer;
use crate::paths::AppPaths;
use crate::secrets::{KeyringSecretStore, MemorySecretStore, SecretStore};
use crate::settings::AppSettings;
use crate::store::{LocalStore, SyncStatus, SyncWorker};
use crate::util::{new_id, now};

pub struct AppContext {
    pub paths: AppPaths,
    settings: RwLock<AppSettings>,
    pub store: Arc<LocalStore>,
    pub secrets: Arc<dyn SecretStore>,
    pub sync: Arc<SyncWorker>,
    pub logs: Arc<LogBuffer>,
    pub agent: AgentHandle,
    runner: RwLock<Option<Arc<dyn ClaudeRunner>>>,
    claude_cli: RwLock<Option<ClaudeCli>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub claude: ClaudeStatus,
    pub chrome: ChromeStatus,
    pub backend: SyncStatus,
    pub profile: ProfileCompleteness,
    pub mock_mode: bool,
    pub setup_completed: bool,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobListItem {
    pub job: Job,
    pub analysis: Option<JobAnalysis>,
    pub application_status: Option<ApplicationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    pub job: Job,
    pub analysis: Option<JobAnalysis>,
    pub application: Option<Application>,
    pub resume: Option<Resume>,
    pub cover_letter: Option<CoverLetter>,
    pub resumes: Vec<Resume>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationListItem {
    pub application: Application,
    pub job: Job,
    pub analysis: Option<JobAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationDetail {
    pub application: Application,
    pub job: Job,
    pub analysis: Option<JobAnalysis>,
    pub resume: Option<Resume>,
    pub cover_letter: Option<CoverLetter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCounts {
    pub jobs_discovered: usize,
    pub relevant: usize,
    pub awaiting_approval: usize,
    pub approved: usize,
    pub applied: usize,
    pub manual_action: usize,
    pub waiting_for_user: usize,
    pub interviews: usize,
    pub rejected: usize,
    pub offers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub today: DashboardCounts,
    pub total: DashboardCounts,
    pub latest_jobs: Vec<JobListItem>,
    pub recent_applications: Vec<ApplicationListItem>,
    pub pending_actions: Vec<ApplicationListItem>,
    pub last_run: Option<AgentRun>,
    pub agent: AgentStatus,
    pub sync: SyncStatus,
}

impl AppContext {
    /// Initialise everything. `logs` is created by the host before logging is
    /// installed so early messages are captured too.
    pub async fn init(paths: AppPaths, logs: Arc<LogBuffer>) -> CoreResult<Arc<Self>> {
        let _ = dotenvy::dotenv();
        let mut settings = AppSettings::load(&paths.settings_file())?;
        // The server address is fixed per build (see `backend_url`). If this
        // desktop signed in under an earlier build's address, follow the
        // current one: a new server IP must not strand a signed-in device.
        let server_url = crate::backend_url::configured();
        // Sync always talks to this build's server. Don't rely on a saved
        // copy of the address: earlier builds never saved it at sign-in,
        // so after a restart sync found no address and reported "not signed
        // in" while still holding a valid device token.
        settings.backend.backend_url = server_url.clone();
        if !settings.remote.relay_url.is_empty() {
            settings.remote.relay_url = server_url;
        }
        let store = Arc::new(LocalStore::open(&paths.database_dir())?);
        let secrets: Arc<dyn SecretStore> =
            if settings.mock_mode && std::env::var("JOB_HUNTER_USE_KEYRING").is_err() {
                Arc::new(MemorySecretStore::default())
            } else {
                Arc::new(KeyringSecretStore)
            };
        let sync = SyncWorker::new(
            store.clone(),
            secrets.clone(),
            settings.backend.backend_url.clone(),
            settings.backend.account_email.clone(),
        );
        tokio::spawn(sync.clone().run());
        let ctx = Arc::new(Self {
            paths,
            settings: RwLock::new(settings),
            store,
            secrets,
            sync,
            logs,
            agent: AgentHandle::new(),
            runner: RwLock::new(None),
            claude_cli: RwLock::new(None),
        });
        ctx.ensure_user_and_profile()?;
        tracing::info!(data_dir = %ctx.paths.root.display(), mock = ctx.settings().await.mock_mode, "Job Hunter core initialised");
        Ok(ctx)
    }

    /// Test/mock constructor with an isolated data directory.
    pub async fn init_mock(root: PathBuf) -> CoreResult<Arc<Self>> {
        std::env::set_var("MOCK_MODE", "true");
        let paths = AppPaths::at(root)?;
        let logs = LogBuffer::new(500, None);
        let ctx = Self::init(paths, logs).await?;
        ctx.set_runner(Arc::new(MockClaudeRunner::instant())).await;
        Ok(ctx)
    }

    pub fn user_id(&self) -> String {
        LOCAL_USER_ID.to_string()
    }

    fn ensure_user_and_profile(&self) -> CoreResult<()> {
        if self.store.get::<User>(LOCAL_USER_ID)?.is_none() {
            let ts = now();
            self.store.put(&User {
                id: LOCAL_USER_ID.into(),
                display_name: "Local user".into(),
                created_at: ts,
                updated_at: ts,
            })?;
        }
        if self
            .store
            .find::<CandidateProfile>(|p| p.user_id == LOCAL_USER_ID)?
            .is_empty()
        {
            self.store.put(&CandidateProfile::new(LOCAL_USER_ID))?;
        }
        Ok(())
    }

    // ---- settings ---------------------------------------------------------------

    pub async fn settings(&self) -> AppSettings {
        self.settings.read().await.clone()
    }

    pub async fn save_settings(&self, mut incoming: AppSettings) -> CoreResult<AppSettings> {
        // Approval mode is never user-configurable to anything but review.
        incoming.approval_mode = crate::settings::ApprovalMode::ReviewRequired;
        incoming.resume.ats_max_iterations = incoming.resume.ats_max_iterations.clamp(1, 3);
        let previous = self.settings().await;
        incoming.save(&self.paths.settings_file())?;
        if previous.backend.backend_url != incoming.backend.backend_url {
            self.sync
                .set_backend_url(incoming.backend.backend_url.clone())
                .await;
        }
        if previous.backend.account_email != incoming.backend.account_email {
            self.sync
                .set_account_email(incoming.backend.account_email.clone())
                .await;
        }
        if previous.mock_mode != incoming.mock_mode
            || previous.claude.cli_path != incoming.claude.cli_path
        {
            *self.runner.write().await = None;
            *self.claude_cli.write().await = None;
        }
        *self.settings.write().await = incoming.clone();
        tracing::info!("settings saved");
        Ok(incoming)
    }

    pub async fn is_mock(&self) -> bool {
        self.settings().await.mock_mode
    }

    // ---- claude / chrome ---------------------------------------------------------

    pub async fn set_runner(&self, runner: Arc<dyn ClaudeRunner>) {
        *self.runner.write().await = Some(runner);
    }

    pub async fn claude_cli(&self) -> CoreResult<ClaudeCli> {
        if let Some(c) = self.claude_cli.read().await.clone() {
            return Ok(c);
        }
        let settings = self.settings().await;
        let cli = ClaudeCli::locate(settings.claude.cli_path.as_deref())?;
        *self.claude_cli.write().await = Some(cli.clone());
        Ok(cli)
    }

    pub async fn runner(&self) -> CoreResult<Arc<dyn ClaudeRunner>> {
        if let Some(r) = self.runner.read().await.clone() {
            return Ok(r);
        }
        let settings = self.settings().await;
        let runner: Arc<dyn ClaudeRunner> = if settings.mock_mode {
            Arc::new(MockClaudeRunner::default())
        } else {
            Arc::new(CliClaudeRunner::new(self.claude_cli().await?))
        };
        *self.runner.write().await = Some(runner.clone());
        Ok(runner)
    }

    pub async fn claude_status(&self) -> ClaudeStatus {
        match self.claude_cli().await {
            Ok(cli) => cli.status().await,
            Err(e) => ClaudeStatus {
                installed: false,
                path: None,
                version: None,
                authenticated: false,
                auth_method: None,
                account: None,
                subscription: None,
                error: Some(e.details().unwrap_or_else(|| e.to_string())),
            },
        }
    }

    pub async fn chrome_path(&self) -> Option<PathBuf> {
        let settings = self.settings().await;
        chrome::find_chrome(settings.browser.chrome_path.as_deref())
    }

    pub async fn chrome_status(&self) -> ChromeStatus {
        let settings = self.settings().await;
        chrome::status(settings.browser.chrome_path.as_deref()).await
    }

    pub async fn setup_status(&self) -> SetupStatus {
        let settings = self.settings().await;
        let profile = self
            .profile()
            .unwrap_or_else(|_| CandidateProfile::new(LOCAL_USER_ID));
        SetupStatus {
            claude: self.claude_status().await,
            chrome: self.chrome_status().await,
            backend: self.sync.status().await,
            profile: profile.completeness(),
            mock_mode: settings.mock_mode,
            setup_completed: settings.setup_completed,
            data_dir: self.paths.root.to_string_lossy().to_string(),
        }
    }

    pub async fn open_url(&self, url: &str) -> CoreResult<()> {
        let chrome = self.chrome_path().await;
        chrome::open_url(chrome.as_deref(), url)
    }

    // ---- events -------------------------------------------------------------------

    pub fn emit_event(&self, event: AgentEvent) {
        if event.kind != "CLAUDE_ACTIVITY" {
            if let Err(e) = self.store.put(&event) {
                tracing::warn!(error = %e, "could not persist agent event");
            }
        }
        self.agent.bus.emit(event);
    }

    // ---- candidate ------------------------------------------------------------------

    pub fn profile(&self) -> CoreResult<CandidateProfile> {
        self.store
            .find::<CandidateProfile>(|p| p.user_id == LOCAL_USER_ID)?
            .into_iter()
            .next()
            .ok_or(CoreError::NotFound {
                entity: "candidate_profile",
                id: LOCAL_USER_ID.into(),
            })
    }

    pub fn save_profile(&self, mut profile: CandidateProfile) -> CoreResult<CandidateProfile> {
        let existing = self.profile()?;
        profile.id = existing.id;
        profile.user_id = LOCAL_USER_ID.into();
        profile.created_at = existing.created_at;
        if profile.master_resume.is_none() {
            profile.master_resume = existing.master_resume;
        }
        profile.updated_at = now();
        self.store.put(&profile)?;
        Ok(profile)
    }

    pub fn experiences(&self) -> CoreResult<Vec<Experience>> {
        let mut v = self
            .store
            .find::<Experience>(|e| e.user_id == LOCAL_USER_ID)?;
        v.sort_by(|a, b| b.start_date.cmp(&a.start_date));
        Ok(v)
    }

    pub fn save_experience(&self, mut e: Experience) -> CoreResult<Experience> {
        e.user_id = LOCAL_USER_ID.into();
        if e.id.is_empty() {
            e.id = new_id();
        }
        e.updated_at = now();
        self.store.put(&e)?;
        Ok(e)
    }

    pub fn delete_experience(&self, id: &str) -> CoreResult<()> {
        self.store.delete::<Experience>(id)?;
        Ok(())
    }

    pub fn projects(&self) -> CoreResult<Vec<Project>> {
        self.store.find::<Project>(|p| p.user_id == LOCAL_USER_ID)
    }

    pub fn save_project(&self, mut p: Project) -> CoreResult<Project> {
        p.user_id = LOCAL_USER_ID.into();
        if p.id.is_empty() {
            p.id = new_id();
        }
        p.updated_at = now();
        self.store.put(&p)?;
        Ok(p)
    }

    pub fn delete_project(&self, id: &str) -> CoreResult<()> {
        self.store.delete::<Project>(id)?;
        Ok(())
    }

    pub async fn candidate_truth(&self) -> CoreResult<CandidateTruth> {
        let profile = self.profile()?;
        let master_resume_text = match &profile.master_resume {
            Some(m) => std::fs::read_to_string(&m.text_path).ok(),
            None => None,
        };
        Ok(CandidateTruth {
            profile,
            experiences: self.experiences()?,
            projects: self.projects()?,
            master_resume_text,
        })
    }

    pub fn import_master_resume(&self, source: &std::path::Path) -> CoreResult<MasterResumeRef> {
        let reference =
            crate::documents::import_master_resume(source, &self.paths.master_resume_dir())?;
        let mut profile = self.profile()?;
        profile.master_resume = Some(reference.clone());
        profile.updated_at = now();
        self.store.put(&profile)?;
        tracing::info!(file = %reference.original_file_name, chars = reference.text_chars, "master resume imported");
        Ok(reference)
    }

    pub fn master_resume_text(&self) -> CoreResult<Option<String>> {
        let profile = self.profile()?;
        Ok(profile
            .master_resume
            .and_then(|m| std::fs::read_to_string(m.text_path).ok()))
    }

    /// Parse the imported master resume with Claude and merge the result into
    /// the profile (only empty fields are filled; experiences/projects are
    /// added when none exist yet).
    pub async fn parse_master_resume(self: &Arc<Self>) -> CoreResult<CandidateProfile> {
        let profile = self.profile()?;
        let text = self
            .master_resume_text()?
            .ok_or_else(|| CoreError::Validation("Import a master resume first".into()))?;
        let runner = self.runner().await?;
        let run_dir = self.paths.run_dir("profile-parse");
        std::fs::create_dir_all(&run_dir)?;
        let settings = self.settings().await;
        let mut req = crate::claude::ClaudeRequest::new(
            "parse_profile",
            crate::prompts::profile_parse_prompt(&text, Some(&profile)),
            run_dir.clone(),
        );
        req.system_prompt_file = Some(crate::prompts::write_system_prompt(&run_dir)?);
        req.json_schema = Some(crate::prompts::schemas::profile_parse());
        req.model = settings.claude.model.clone();
        req.max_budget_usd = settings.claude.max_budget_usd_per_call;
        req.max_turns = 6;
        let resp = runner
            .run(
                req,
                crate::claude::protocol::noop_sink(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await?;
        let parsed = resp.structured.ok_or_else(|| CoreError::ClaudeRunFailed {
            message: "no structured profile returned".into(),
            details: crate::util::truncate(&resp.text, 500),
        })?;
        self.merge_parsed_profile(parsed)
    }

    fn merge_parsed_profile(&self, parsed: serde_json::Value) -> CoreResult<CandidateProfile> {
        #[derive(Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct ParsedExperience {
            company: String,
            role: String,
            #[serde(default)]
            location: String,
            #[serde(default)]
            start_date: String,
            #[serde(default)]
            end_date: Option<String>,
            #[serde(default)]
            description: String,
            #[serde(default)]
            technologies: Vec<String>,
            #[serde(default)]
            achievements: Vec<String>,
        }
        #[derive(Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct ParsedProject {
            name: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            role: String,
            #[serde(default)]
            technologies: Vec<String>,
            #[serde(default)]
            responsibilities: Vec<String>,
            #[serde(default)]
            achievements: Vec<String>,
            #[serde(default)]
            url: String,
            #[serde(default)]
            experience_company: String,
        }
        #[derive(Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct Parsed {
            #[serde(default)]
            personal: PersonalInfo,
            #[serde(default)]
            summary: String,
            #[serde(default)]
            skills: SkillGroups,
            #[serde(default)]
            experiences: Vec<ParsedExperience>,
            #[serde(default)]
            projects: Vec<ParsedProject>,
            #[serde(default)]
            education: Vec<Education>,
            #[serde(default)]
            certifications: Vec<Certification>,
            #[serde(default)]
            achievements: Vec<String>,
            #[serde(default)]
            languages: Vec<String>,
        }
        let parsed: Parsed = serde_json::from_value(parsed)?;
        let mut profile = self.profile()?;
        let fill = |target: &mut String, value: String| {
            if target.trim().is_empty() && !value.trim().is_empty() {
                *target = value;
            }
        };
        fill(&mut profile.personal.name, parsed.personal.name);
        fill(&mut profile.personal.email, parsed.personal.email);
        fill(&mut profile.personal.phone, parsed.personal.phone);
        fill(&mut profile.personal.location, parsed.personal.location);
        fill(&mut profile.personal.linkedin, parsed.personal.linkedin);
        fill(&mut profile.personal.github, parsed.personal.github);
        fill(&mut profile.personal.portfolio, parsed.personal.portfolio);
        fill(
            &mut profile.personal.current_title,
            parsed.personal.current_title,
        );
        fill(&mut profile.summary, parsed.summary);
        let merge_list = |target: &mut Vec<String>, incoming: Vec<String>| {
            for i in incoming {
                if !target.iter().any(|t| t.eq_ignore_ascii_case(&i)) {
                    target.push(i);
                }
            }
        };
        merge_list(&mut profile.skills.frontend, parsed.skills.frontend);
        merge_list(&mut profile.skills.backend, parsed.skills.backend);
        merge_list(&mut profile.skills.database, parsed.skills.database);
        merge_list(&mut profile.skills.devops, parsed.skills.devops);
        merge_list(&mut profile.skills.cloud, parsed.skills.cloud);
        merge_list(&mut profile.skills.testing, parsed.skills.testing);
        merge_list(&mut profile.skills.other, parsed.skills.other);
        merge_list(&mut profile.achievements, parsed.achievements);
        merge_list(&mut profile.languages, parsed.languages);
        if profile.education.is_empty() {
            profile.education = parsed
                .education
                .into_iter()
                .map(|mut e| {
                    e.id = new_id();
                    e
                })
                .collect();
        }
        if profile.certifications.is_empty() {
            profile.certifications = parsed
                .certifications
                .into_iter()
                .map(|mut c| {
                    c.id = new_id();
                    c
                })
                .collect();
        }
        if self.experiences()?.is_empty() {
            let mut company_ids = std::collections::HashMap::new();
            for pe in parsed.experiences {
                let ts = now();
                let e = Experience {
                    id: new_id(),
                    user_id: LOCAL_USER_ID.into(),
                    company: pe.company.clone(),
                    role: pe.role,
                    location: pe.location,
                    start_date: pe.start_date,
                    end_date: pe.end_date.filter(|s| !s.trim().is_empty()),
                    description: pe.description,
                    technologies: pe.technologies,
                    achievements: pe.achievements,
                    projects: vec![],
                    created_at: ts,
                    updated_at: ts,
                };
                company_ids.insert(pe.company.to_lowercase(), e.id.clone());
                self.store.put(&e)?;
            }
            if self.projects()?.is_empty() {
                for pp in parsed.projects {
                    let ts = now();
                    let p = Project {
                        id: new_id(),
                        user_id: LOCAL_USER_ID.into(),
                        experience_id: company_ids
                            .get(&pp.experience_company.to_lowercase())
                            .cloned(),
                        name: pp.name,
                        description: pp.description,
                        role: pp.role,
                        technologies: pp.technologies,
                        responsibilities: pp.responsibilities,
                        achievements: pp.achievements,
                        url: pp.url,
                        created_at: ts,
                        updated_at: ts,
                    };
                    self.store.put(&p)?;
                }
            }
        }
        profile.updated_at = now();
        self.store.put(&profile)?;
        Ok(profile)
    }

    // ---- jobs -----------------------------------------------------------------------

    pub fn list_jobs(&self) -> CoreResult<Vec<JobListItem>> {
        let analyses = self.store.list::<JobAnalysis>()?;
        let applications = self.store.list::<Application>()?;
        let mut jobs = self.store.list::<Job>()?;
        jobs.sort_by_key(|j| std::cmp::Reverse(j.discovered_at));
        Ok(jobs
            .into_iter()
            .map(|job| {
                let analysis = job
                    .analysis_id
                    .as_ref()
                    .and_then(|id| analyses.iter().find(|a| &a.id == id).cloned());
                let application_status = applications
                    .iter()
                    .find(|a| a.job_id == job.id)
                    .map(|a| a.status);
                JobListItem {
                    job,
                    analysis,
                    application_status,
                }
            })
            .collect())
    }

    pub fn job_detail(&self, job_id: &str) -> CoreResult<JobDetail> {
        let job: Job = self.store.require(job_id)?;
        let analysis = match &job.analysis_id {
            Some(id) => self.store.get::<JobAnalysis>(id)?,
            None => None,
        };
        let application = self
            .store
            .find::<Application>(|a| a.job_id == job.id)?
            .into_iter()
            .next();
        let mut resumes = self
            .store
            .find::<Resume>(|r| r.job_id.as_deref() == Some(job_id))?;
        resumes.sort_by_key(|r| std::cmp::Reverse(r.version));
        let resume = match application.as_ref().and_then(|a| a.resume_id.clone()) {
            Some(id) => self.store.get::<Resume>(&id)?,
            None => resumes.first().cloned(),
        };
        let cover_letter = match application.as_ref().and_then(|a| a.cover_letter_id.clone()) {
            Some(id) => self.store.get::<CoverLetter>(&id)?,
            None => self
                .store
                .find::<CoverLetter>(|c| c.job_id == job_id)?
                .into_iter()
                .max_by_key(|c| c.version),
        };
        Ok(JobDetail {
            job,
            analysis,
            application,
            resume,
            cover_letter,
            resumes,
        })
    }

    pub fn set_job_saved(&self, job_id: &str, saved: bool) -> CoreResult<Job> {
        let mut job: Job = self.store.require(job_id)?;
        job.saved = saved;
        job.touch();
        self.store.put(&job)?;
        Ok(job)
    }

    pub fn reject_job(&self, job_id: &str) -> CoreResult<Job> {
        let mut job: Job = self.store.require(job_id)?;
        if let Some(mut app) = self
            .store
            .find::<Application>(|a| a.job_id == job.id)?
            .into_iter()
            .next()
        {
            if app.status.can_transition_to(ApplicationStatus::Rejected) {
                app.transition(ApplicationStatus::Rejected, "rejected by user")?;
                self.store.put(&app)?;
            }
        }
        job.status = JobStatus::Rejected;
        job.touch();
        self.store.put(&job)?;
        Ok(job)
    }

    pub fn delete_job(&self, job_id: &str) -> CoreResult<()> {
        for app in self.store.find::<Application>(|a| a.job_id == job_id)? {
            self.store.delete::<Application>(&app.id)?;
        }
        for a in self.store.find::<JobAnalysis>(|a| a.job_id == job_id)? {
            self.store.delete::<JobAnalysis>(&a.id)?;
        }
        self.store.delete::<Job>(job_id)?;
        Ok(())
    }

    // ---- applications ---------------------------------------------------------------

    pub fn list_applications(&self) -> CoreResult<Vec<ApplicationListItem>> {
        let jobs = self.store.list::<Job>()?;
        let analyses = self.store.list::<JobAnalysis>()?;
        let mut apps = self.store.list::<Application>()?;
        apps.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
        Ok(apps
            .into_iter()
            .filter_map(|application| {
                let job = jobs.iter().find(|j| j.id == application.job_id)?.clone();
                let analysis = job
                    .analysis_id
                    .as_ref()
                    .and_then(|id| analyses.iter().find(|a| &a.id == id).cloned());
                Some(ApplicationListItem {
                    application,
                    job,
                    analysis,
                })
            })
            .collect())
    }

    pub fn application_detail(&self, application_id: &str) -> CoreResult<ApplicationDetail> {
        let application: Application = self.store.require(application_id)?;
        let job: Job = self.store.require(&application.job_id)?;
        let analysis = match &job.analysis_id {
            Some(id) => self.store.get::<JobAnalysis>(id)?,
            None => None,
        };
        let resume = match &application.resume_id {
            Some(id) => self.store.get::<Resume>(id)?,
            None => None,
        };
        let cover_letter = match &application.cover_letter_id {
            Some(id) => self.store.get::<CoverLetter>(id)?,
            None => None,
        };
        Ok(ApplicationDetail {
            application,
            job,
            analysis,
            resume,
            cover_letter,
        })
    }

    pub fn add_application_note(
        &self,
        application_id: &str,
        note: &str,
    ) -> CoreResult<Application> {
        let mut application: Application = self.store.require(application_id)?;
        if !application.notes.is_empty() {
            application.notes.push('\n');
        }
        application.notes.push_str(note.trim());
        application.updated_at = now();
        self.store.put(&application)?;
        Ok(application)
    }

    // ---- resumes / cover letters -------------------------------------------------------

    pub fn list_resumes(&self) -> CoreResult<Vec<Resume>> {
        let mut v = self.store.list::<Resume>()?;
        v.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        Ok(v)
    }

    pub async fn update_resume_content(
        &self,
        resume_id: &str,
        content: ResumeDocument,
    ) -> CoreResult<Resume> {
        let mut resume: Resume = self.store.require(resume_id)?;
        resume.content = Some(content);
        crate::agent::steps::rerender_resume(self, &mut resume).await?;
        Ok(resume)
    }

    pub async fn update_cover_letter(
        &self,
        cover_letter_id: &str,
        text: String,
    ) -> CoreResult<CoverLetter> {
        let mut cl: CoverLetter = self.store.require(cover_letter_id)?;
        let job: Job = self.store.require(&cl.job_id)?;
        cl.text = text;
        crate::agent::steps::rerender_cover_letter(self, &mut cl, &job.company).await?;
        Ok(cl)
    }

    // ---- answers ------------------------------------------------------------------------

    pub fn list_answers(&self) -> CoreResult<Vec<AnswerRecord>> {
        let mut v = self.store.list::<AnswerRecord>()?;
        v.sort_by_key(|a| a.question.to_lowercase());
        Ok(v)
    }

    pub fn save_answer(&self, mut record: AnswerRecord) -> CoreResult<AnswerRecord> {
        if record.question.trim().is_empty() || record.answer.trim().is_empty() {
            return Err(CoreError::Validation(
                "question and answer are required".into(),
            ));
        }
        record.user_id = LOCAL_USER_ID.into();
        if record.id.is_empty() {
            record.id = new_id();
        }
        record.normalize();
        record.updated_at = now();
        self.store.put(&record)?;
        Ok(record)
    }

    pub fn delete_answer(&self, id: &str) -> CoreResult<()> {
        self.store.delete::<AnswerRecord>(id)?;
        Ok(())
    }

    // ---- runs / events ------------------------------------------------------------------

    pub fn list_runs(&self, limit: usize) -> CoreResult<Vec<AgentRun>> {
        let mut v = self.store.list::<AgentRun>()?;
        v.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        v.truncate(limit);
        Ok(v)
    }

    pub fn run_events(&self, run_id: &str, limit: usize) -> CoreResult<Vec<AgentEvent>> {
        let mut v = self.store.find::<AgentEvent>(|e| e.run_id == run_id)?;
        v.sort_by_key(|e| e.at);
        if v.len() > limit {
            v = v.split_off(v.len() - limit);
        }
        Ok(v)
    }

    // ---- dashboard ------------------------------------------------------------------------

    pub async fn dashboard(&self) -> CoreResult<Dashboard> {
        let jobs = self.list_jobs()?;
        let applications = self.list_applications()?;
        let today_start: DateTime<Utc> = now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|d| DateTime::from_naive_utc_and_offset(d, Utc))
            .unwrap_or_else(now);
        let count = |since: Option<DateTime<Utc>>| {
            let mut c = DashboardCounts::default();
            for item in &jobs {
                if since.map(|s| item.job.discovered_at < s).unwrap_or(false) {
                    continue;
                }
                c.jobs_discovered += 1;
                if item.analysis.as_ref().map(|a| a.relevant).unwrap_or(false) {
                    c.relevant += 1;
                }
            }
            for item in &applications {
                if since
                    .map(|s| item.application.updated_at < s && item.application.created_at < s)
                    .unwrap_or(false)
                {
                    continue;
                }
                match item.application.status {
                    ApplicationStatus::ReadyForReview => c.awaiting_approval += 1,
                    ApplicationStatus::Approved | ApplicationStatus::Applying => c.approved += 1,
                    ApplicationStatus::Applied => c.applied += 1,
                    ApplicationStatus::ManualActionRequired => c.manual_action += 1,
                    ApplicationStatus::WaitingForUser => c.waiting_for_user += 1,
                    ApplicationStatus::Interview => c.interviews += 1,
                    ApplicationStatus::Rejected => c.rejected += 1,
                    ApplicationStatus::Offer => c.offers += 1,
                    _ => {}
                }
            }
            c
        };
        let today = count(Some(today_start));
        let total = count(None);
        let pending_actions: Vec<ApplicationListItem> = applications
            .iter()
            .filter(|a| {
                matches!(
                    a.application.status,
                    ApplicationStatus::ReadyForReview
                        | ApplicationStatus::WaitingForUser
                        | ApplicationStatus::ManualActionRequired
                )
            })
            .cloned()
            .collect();
        Ok(Dashboard {
            today,
            total,
            latest_jobs: jobs.into_iter().take(8).collect(),
            recent_applications: applications.into_iter().take(8).collect(),
            pending_actions,
            last_run: self.list_runs(1)?.into_iter().next(),
            agent: self.agent.status().await,
            sync: self.sync.status().await,
        })
    }
}
