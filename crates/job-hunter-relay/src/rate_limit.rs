//! A small in-memory sliding-window rate limiter for the auth endpoints.
//! State is not persisted across restarts, which is an acceptable trade-off
//! for a personal-use relay; a restart is itself a full reset of any
//! in-progress brute force attempt.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    max_attempts: usize,
    window: Duration,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window: Duration) -> Self {
        Self {
            max_attempts,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records an attempt for `key` and returns `true` if it should be
    /// allowed (fewer than `max_attempts` recorded within the window).
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max_attempts {
            return false;
        }
        entry.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_then_blocks() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.check("1.2.3.4"));
        assert!(limiter.check("1.2.3.4"));
        assert!(limiter.check("1.2.3.4"));
        assert!(
            !limiter.check("1.2.3.4"),
            "fourth attempt within the window must be blocked"
        );
        assert!(
            limiter.check("5.6.7.8"),
            "a different key has its own budget"
        );
    }
}
