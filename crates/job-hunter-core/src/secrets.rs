//! Secret storage backed by the operating system credential store
//! (Windows Credential Manager, macOS Keychain, Linux Secret Service).
//!
//! The MongoDB connection string is the only secret Job Hunter holds. Claude
//! authentication belongs to Claude Code and is never read or stored here.

use crate::error::{CoreError, CoreResult};

const SERVICE: &str = "JobHunter";
pub const MONGODB_URI_KEY: &str = "mongodb-uri";

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
    fn delete(&self, key: &str) -> CoreResult<()>;
}

/// OS keychain implementation.
pub struct KeyringSecretStore;

impl SecretStore for KeyringSecretStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        let entry =
            keyring::Entry::new(SERVICE, key).map_err(|e| CoreError::Secrets(e.to_string()))?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CoreError::Secrets(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        let entry =
            keyring::Entry::new(SERVICE, key).map_err(|e| CoreError::Secrets(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| CoreError::Secrets(e.to_string()))
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        let entry =
            keyring::Entry::new(SERVICE, key).map_err(|e| CoreError::Secrets(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CoreError::Secrets(e.to_string())),
        }
    }
}

/// In-memory store for tests and mock mode. Never persists anything.
#[derive(Default)]
pub struct MemorySecretStore {
    inner: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl SecretStore for MemorySecretStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.inner.lock().unwrap().get(key).cloned())
    }
    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.inner.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> CoreResult<()> {
        self.inner.lock().unwrap().remove(key);
        Ok(())
    }
}

/// Mask a MongoDB URI for display/logging: keeps scheme, user and host, hides
/// the password.
pub fn redact_mongo_uri(uri: &str) -> String {
    if let Some(at) = uri.rfind('@') {
        if let Some(scheme_end) = uri.find("://") {
            let creds = &uri[scheme_end + 3..at];
            let user = creds.split(':').next().unwrap_or("");
            return format!("{}://{}:***@{}", &uri[..scheme_end], user, &uri[at + 1..]);
        }
    }
    uri.to_string()
}

/// Strip anything that looks like a credential from a free-form string before
/// it is logged.
pub fn redact_secrets(text: &str) -> String {
    static PATTERNS: once_cell::sync::Lazy<Vec<regex::Regex>> = once_cell::sync::Lazy::new(|| {
        vec![
            regex::Regex::new(r"(?i)(mongodb(\+srv)?://)([^:/\s]+):([^@\s]+)@").unwrap(),
            regex::Regex::new(r"(?i)(sk-ant-[A-Za-z0-9_\-]{8,})").unwrap(),
            regex::Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._\-]{10,}").unwrap(),
            regex::Regex::new(r#"(?i)("?(password|passwd|secret|token|api[_-]?key|cookie)"?\s*[:=]\s*"?)([^"\s,;]+)"#).unwrap(),
        ]
    });
    let mut out = text.to_string();
    out = PATTERNS[0].replace_all(&out, "${1}${3}:***@").to_string();
    out = PATTERNS[1].replace_all(&out, "sk-ant-***").to_string();
    out = PATTERNS[2].replace_all(&out, "${1}***").to_string();
    out = PATTERNS[3].replace_all(&out, "${1}***").to_string();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_mongo_password() {
        let uri = "mongodb+srv://alice:s3cret@cluster0.example.mongodb.net/db?retryWrites=true";
        assert_eq!(
            redact_mongo_uri(uri),
            "mongodb+srv://alice:***@cluster0.example.mongodb.net/db?retryWrites=true"
        );
        let logged = redact_secrets(&format!("connecting to {uri}"));
        assert!(!logged.contains("s3cret"));
        assert!(logged.contains("alice:***@"));
    }

    #[test]
    fn redacts_generic_tokens() {
        let s = redact_secrets(
            r#"{"token": "abcdef123456", "password": "hunter2"} sk-ant-api03-abcdefghijk Bearer abcdefghijklmnop"#,
        );
        assert!(!s.contains("abcdef123456"));
        assert!(!s.contains("hunter2"));
        assert!(!s.contains("api03-abcdefghijk"));
        assert!(!s.contains("abcdefghijklmnop"));
    }

    #[test]
    fn memory_store_round_trip() {
        let s = MemorySecretStore::default();
        assert_eq!(s.get("k").unwrap(), None);
        s.set("k", "v").unwrap();
        assert_eq!(s.get("k").unwrap().as_deref(), Some("v"));
        s.delete("k").unwrap();
        assert_eq!(s.get("k").unwrap(), None);
    }
}
