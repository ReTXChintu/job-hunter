//! The one server address this desktop build talks to.
//!
//! There is no URL field in the app: the address is baked in when the
//! installer is built (CI sets `JOB_HUNTER_BACKEND_URL` from a GitHub
//! secret, see `.github/workflows/build.yml`). Both backend data sync
//! (`store::sync`) and the mobile-app connection (`remote::client`) use it,
//! since the backend is wire-compatible with the relay's protocol.
//!
//! Resolution order:
//! 1. `JOB_HUNTER_BACKEND_URL` at **compile time** (release builds).
//! 2. `JOB_HUNTER_BACKEND_URL` at **run time** (a repo-level `.env` in
//!    development, loaded by `AppContext::init`).
//! 3. [`DEV_DEFAULT`], a backend running locally.
//!
//! The address is chosen by whoever builds the app, so it is trusted as-is:
//! plain `http://<ip>:<port>` is allowed, with no per-user "allow insecure"
//! opt-in.

/// Used only when no address was provided at build or run time.
pub const DEV_DEFAULT: &str = "http://127.0.0.1:8788";

const ENV_KEY: &str = "JOB_HUNTER_BACKEND_URL";

fn clean(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/');
    (!url.is_empty()).then(|| url.to_string())
}

/// The backend base URL, without a trailing slash.
pub fn configured() -> String {
    option_env!("JOB_HUNTER_BACKEND_URL")
        .and_then(clean)
        .or_else(|| std::env::var(ENV_KEY).ok().as_deref().and_then(clean))
        .unwrap_or_else(|| DEV_DEFAULT.to_string())
}

/// True when the address was fixed at build time (a CI/release build).
pub fn is_built_in() -> bool {
    option_env!("JOB_HUNTER_BACKEND_URL")
        .and_then(clean)
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_trims_whitespace_and_trailing_slashes_and_rejects_blank() {
        assert_eq!(
            clean(" http://10.0.0.5:8788/ ").as_deref(),
            Some("http://10.0.0.5:8788")
        );
        assert_eq!(clean("   "), None);
    }

    #[test]
    fn a_compiled_in_address_wins() {
        // Only meaningful when built with JOB_HUNTER_BACKEND_URL set, as CI does.
        if let Some(built) = option_env!("JOB_HUNTER_BACKEND_URL").and_then(clean) {
            assert!(is_built_in());
            assert_eq!(configured(), built);
        }
    }

    #[test]
    fn configured_is_never_empty() {
        assert!(!configured().is_empty());
    }
}
