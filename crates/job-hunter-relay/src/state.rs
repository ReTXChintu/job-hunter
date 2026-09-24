use std::sync::Arc;
use std::time::Duration;

use crate::config::RelayConfig;
use crate::db::Db;
use crate::hub::Hub;
use crate::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RelayConfig>,
    pub db: Arc<Db>,
    pub hub: Arc<Hub>,
    pub auth_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(config: RelayConfig, db: Db) -> Self {
        Self {
            config: Arc::new(config),
            db: Arc::new(db),
            hub: Hub::new(),
            // 10 attempts per email/IP per 5 minutes on login/register.
            auth_limiter: Arc::new(RateLimiter::new(10, Duration::from_secs(300))),
        }
    }
}
