use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::error::{CoreError, CoreResult};

/// Platform-appropriate application data directory layout.
///
/// Windows: `%APPDATA%\JobHunter`
/// macOS:   `~/Library/Application Support/JobHunter`
/// Linux:   `~/.local/share/job-hunter`
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
}

impl AppPaths {
    pub fn detect() -> CoreResult<Self> {
        if let Ok(custom) = std::env::var("JOB_HUNTER_DATA_DIR") {
            if !custom.trim().is_empty() {
                return Self::at(PathBuf::from(custom));
            }
        }
        let base = BaseDirs::new()
            .ok_or_else(|| CoreError::other("Could not determine the home directory"))?;
        let root = if cfg!(target_os = "windows") {
            base.config_dir().join("JobHunter")
        } else if cfg!(target_os = "macos") {
            base.home_dir()
                .join("Library")
                .join("Application Support")
                .join("JobHunter")
        } else {
            base.data_local_dir().join("job-hunter")
        };
        Self::at(root)
    }

    pub fn at(root: PathBuf) -> CoreResult<Self> {
        let paths = Self { root };
        paths.ensure()?;
        Ok(paths)
    }

    pub fn ensure(&self) -> CoreResult<()> {
        for dir in [
            self.root.clone(),
            self.database_dir(),
            self.resumes_dir(),
            self.master_resume_dir(),
            self.generated_dir(),
            self.cover_letters_dir(),
            self.logs_dir(),
            self.runs_dir(),
            self.temp_dir(),
            self.exports_dir(),
        ] {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    pub fn database_dir(&self) -> PathBuf {
        self.root.join("database")
    }
    pub fn resumes_dir(&self) -> PathBuf {
        self.root.join("resumes")
    }
    pub fn master_resume_dir(&self) -> PathBuf {
        self.resumes_dir().join("master")
    }
    pub fn generated_dir(&self) -> PathBuf {
        self.resumes_dir().join("generated")
    }
    pub fn cover_letters_dir(&self) -> PathBuf {
        self.root.join("cover-letters")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }
    pub fn temp_dir(&self) -> PathBuf {
        self.root.join("temp")
    }
    pub fn exports_dir(&self) -> PathBuf {
        self.root.join("exports")
    }
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(run_id)
    }
    pub fn company_resume_dir(&self, company_slug: &str) -> PathBuf {
        self.generated_dir().join(company_slug)
    }
}

pub fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
