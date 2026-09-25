use std::path::PathBuf;

/// Relay configuration, loaded from environment variables (see the "Legacy
/// relay" section of the repository's single root `.env.example`; `dotenvy`
/// finds that `.env` by searching up from the working directory). Every value has a safe local-dev default except the
/// JWT signing secret, which the process refuses to start without in a
/// non-debug build.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub bind_addr: String,
    pub database_path: PathBuf,
    pub jwt_secret: String,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_days: i64,
    pub pairing_code_ttl_secs: i64,
    pub cors_origins: Vec<String>,
}

impl RelayConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        let jwt_secret = std::env::var("JOB_HUNTER_RELAY_JWT_SECRET").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                tracing::warn!("JOB_HUNTER_RELAY_JWT_SECRET not set; using an insecure development default. Set it before deploying.");
                "dev-only-insecure-secret-change-me".to_string()
            } else {
                panic!("JOB_HUNTER_RELAY_JWT_SECRET must be set to a long random value in production");
            }
        });
        Self {
            bind_addr: std::env::var("JOB_HUNTER_RELAY_BIND")
                .unwrap_or_else(|_| "0.0.0.0:8787".into()),
            database_path: std::env::var("JOB_HUNTER_RELAY_DB")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("relay.sqlite3")),
            jwt_secret,
            access_token_ttl_secs: env_i64("JOB_HUNTER_RELAY_ACCESS_TTL_SECS", 15 * 60),
            refresh_token_ttl_days: env_i64("JOB_HUNTER_RELAY_REFRESH_TTL_DAYS", 30),
            pairing_code_ttl_secs: env_i64("JOB_HUNTER_RELAY_PAIRING_TTL_SECS", 5 * 60),
            cors_origins: std::env::var("JOB_HUNTER_RELAY_CORS_ORIGINS")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
