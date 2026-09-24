//! SQLite persistence for accounts, devices, refresh tokens and pairing
//! codes. This is the entire data footprint of the relay: no job or
//! application content is ever written here.

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{RelayError, RelayResult};
use crate::models::{Device, DeviceKind, User};

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> RelayResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| RelayError::Internal(e.to_string()))?;
            }
        }
        let conn = Connection::open(path).map_err(|e| RelayError::Internal(e.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory() -> RelayResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| RelayError::Internal(e.to_string()))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ---- users --------------------------------------------------------------------

    pub fn create_user(
        &self,
        id: &str,
        email: &str,
        password_hash: &str,
        display_name: &str,
        now: DateTime<Utc>,
    ) -> RelayResult<User> {
        let conn = self.lock();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM users WHERE email = ?1 COLLATE NOCASE",
                params![email],
                |r| r.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Err(RelayError::EmailTaken);
        }
        conn.execute(
            "INSERT INTO users (id, email, password_hash, display_name, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, email, password_hash, display_name, now.to_rfc3339()],
        )?;
        Ok(User {
            id: id.to_string(),
            email: email.to_string(),
            display_name: display_name.to_string(),
            created_at: now,
        })
    }

    /// Returns the user along with the stored password hash (needed by the
    /// caller to verify, never returned to clients).
    pub fn find_user_by_email(&self, email: &str) -> RelayResult<Option<(User, String)>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, email, display_name, created_at, password_hash FROM users WHERE email = ?1 COLLATE NOCASE",
            params![email],
            |r| {
                let created_at: String = r.get(3)?;
                Ok((
                    User { id: r.get(0)?, email: r.get(1)?, display_name: r.get(2)?, created_at: parse_ts(&created_at) },
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn find_user_by_id(&self, id: &str) -> RelayResult<Option<User>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, email, display_name, created_at FROM users WHERE id = ?1",
            params![id],
            |r| {
                let created_at: String = r.get(3)?;
                Ok(User {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    display_name: r.get(2)?,
                    created_at: parse_ts(&created_at),
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    // ---- refresh tokens -------------------------------------------------------------

    pub fn insert_refresh_token(
        &self,
        token_hash: &str,
        user_id: &str,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> RelayResult<()> {
        self.lock()
            .execute(
                "INSERT INTO refresh_tokens (token_hash, user_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
                params![token_hash, user_id, now.to_rfc3339(), expires_at.to_rfc3339()],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Deletes the token (single use / rotate-on-refresh) and returns the
    /// owning user id if it existed and had not expired.
    pub fn consume_refresh_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> RelayResult<Option<String>> {
        let conn = self.lock();
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT user_id, expires_at FROM refresh_tokens WHERE token_hash = ?1",
                params![token_hash],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        conn.execute(
            "DELETE FROM refresh_tokens WHERE token_hash = ?1",
            params![token_hash],
        )?;
        match row {
            Some((user_id, expires_at)) if parse_ts(&expires_at) > now => Ok(Some(user_id)),
            _ => Ok(None),
        }
    }

    pub fn revoke_all_refresh_tokens(&self, user_id: &str) -> RelayResult<()> {
        self.lock()
            .execute(
                "DELETE FROM refresh_tokens WHERE user_id = ?1",
                params![user_id],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    // ---- devices --------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn create_device(
        &self,
        id: &str,
        user_id: &str,
        name: &str,
        kind: DeviceKind,
        platform: &str,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> RelayResult<Device> {
        self.lock().execute(
            "INSERT INTO devices (id, user_id, name, kind, platform, token_hash, created_at, last_seen_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![id, user_id, name, kind.as_str(), platform, token_hash, now.to_rfc3339()],
        )?;
        Ok(Device {
            id: id.into(),
            user_id: user_id.into(),
            name: name.into(),
            kind,
            platform: platform.into(),
            created_at: now,
            last_seen_at: None,
            online: false,
        })
    }

    pub fn find_device_by_token_hash(&self, token_hash: &str) -> RelayResult<Option<Device>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, user_id, name, kind, platform, created_at, last_seen_at FROM devices WHERE token_hash = ?1",
            params![token_hash],
            device_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn find_device_by_id(&self, user_id: &str, device_id: &str) -> RelayResult<Option<Device>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, user_id, name, kind, platform, created_at, last_seen_at FROM devices WHERE id = ?1 AND user_id = ?2",
            params![device_id, user_id],
            device_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_devices(&self, user_id: &str) -> RelayResult<Vec<Device>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT id, user_id, name, kind, platform, created_at, last_seen_at FROM devices WHERE user_id = ?1 ORDER BY created_at ASC")?;
        let rows = stmt.query_map(params![user_id], device_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_device(&self, user_id: &str, device_id: &str) -> RelayResult<bool> {
        let n = self.lock().execute(
            "DELETE FROM devices WHERE id = ?1 AND user_id = ?2",
            params![device_id, user_id],
        )?;
        Ok(n > 0)
    }

    pub fn touch_device_last_seen(&self, device_id: &str, now: DateTime<Utc>) -> RelayResult<()> {
        self.lock()
            .execute(
                "UPDATE devices SET last_seen_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), device_id],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    // ---- pairing codes ----------------------------------------------------------------

    pub fn create_pairing_code(
        &self,
        code: &str,
        user_id: &str,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> RelayResult<()> {
        self.lock()
            .execute(
                "INSERT INTO pairing_codes (code, user_id, created_at, expires_at, redeemed) VALUES (?1, ?2, ?3, ?4, 0)",
                params![code, user_id, now.to_rfc3339(), expires_at.to_rfc3339()],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Atomically marks the code redeemed and returns the owning user id,
    /// only if it exists, has not been redeemed, and has not expired.
    pub fn redeem_pairing_code(
        &self,
        code: &str,
        now: DateTime<Utc>,
    ) -> RelayResult<Option<String>> {
        let conn = self.lock();
        let row: Option<(String, String, i64)> = conn
            .query_row(
                "SELECT user_id, expires_at, redeemed FROM pairing_codes WHERE code = ?1",
                params![code],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((user_id, expires_at, redeemed)) = row else {
            return Ok(None);
        };
        if redeemed != 0 || parse_ts(&expires_at) <= now {
            return Ok(None);
        }
        conn.execute(
            "UPDATE pairing_codes SET redeemed = 1 WHERE code = ?1",
            params![code],
        )?;
        Ok(Some(user_id))
    }

    /// Best-effort housekeeping: drop expired refresh tokens and pairing
    /// codes. Safe to call periodically; nothing depends on it running.
    pub fn cleanup_expired(&self, now: DateTime<Utc>) -> RelayResult<()> {
        let conn = self.lock();
        conn.execute(
            "DELETE FROM refresh_tokens WHERE expires_at <= ?1",
            params![now.to_rfc3339()],
        )?;
        conn.execute(
            "DELETE FROM pairing_codes WHERE expires_at <= ?1",
            params![now.to_rfc3339()],
        )?;
        Ok(())
    }
}

fn device_from_row(r: &rusqlite::Row) -> rusqlite::Result<Device> {
    let kind_str: String = r.get(3)?;
    let created_at: String = r.get(5)?;
    let last_seen_at: Option<String> = r.get(6)?;
    Ok(Device {
        id: r.get(0)?,
        user_id: r.get(1)?,
        name: r.get(2)?,
        kind: DeviceKind::parse(&kind_str).unwrap_or(DeviceKind::Mobile),
        platform: r.get(4)?,
        created_at: parse_ts(&created_at),
        last_seen_at: last_seen_at.as_deref().map(parse_ts),
        online: false,
    })
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE COLLATE NOCASE,
  password_hash TEXT NOT NULL,
  display_name TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
  token_hash TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON refresh_tokens(user_id);

CREATE TABLE IF NOT EXISTS devices (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  platform TEXT NOT NULL DEFAULT '',
  token_hash TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  last_seen_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);
CREATE INDEX IF NOT EXISTS idx_devices_token_hash ON devices(token_hash);

CREATE TABLE IF NOT EXISTS pairing_codes (
  code TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  redeemed INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_pairing_codes_user ON pairing_codes(user_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DeviceKind;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn user_email_is_unique_case_insensitively() {
        let db = Db::in_memory().unwrap();
        db.create_user("u1", "a@example.com", "hash", "A", now())
            .unwrap();
        let err = db
            .create_user("u2", "A@Example.com", "hash2", "A2", now())
            .unwrap_err();
        assert!(matches!(err, RelayError::EmailTaken));
        let (user, hash) = db.find_user_by_email("A@EXAMPLE.COM").unwrap().unwrap();
        assert_eq!(user.id, "u1");
        assert_eq!(hash, "hash");
    }

    #[test]
    fn refresh_token_is_single_use_and_expiry_checked() {
        let db = Db::in_memory().unwrap();
        db.create_user("u1", "a@example.com", "h", "A", now())
            .unwrap();
        db.insert_refresh_token("tok1", "u1", now(), now() + chrono::Duration::days(1))
            .unwrap();
        assert_eq!(
            db.consume_refresh_token("tok1", now()).unwrap(),
            Some("u1".to_string())
        );
        // Second use fails: it was deleted.
        assert_eq!(db.consume_refresh_token("tok1", now()).unwrap(), None);

        db.insert_refresh_token("tok2", "u1", now(), now() - chrono::Duration::seconds(1))
            .unwrap();
        assert_eq!(
            db.consume_refresh_token("tok2", now()).unwrap(),
            None,
            "expired token must not validate"
        );
    }

    #[test]
    fn device_lifecycle_and_lookup_by_token_hash() {
        let db = Db::in_memory().unwrap();
        db.create_user("u1", "a@example.com", "h", "A", now())
            .unwrap();
        let d = db
            .create_device(
                "d1",
                "u1",
                "Pixel",
                DeviceKind::Mobile,
                "android",
                "hashed-token",
                now(),
            )
            .unwrap();
        assert_eq!(d.id, "d1");
        let found = db
            .find_device_by_token_hash("hashed-token")
            .unwrap()
            .unwrap();
        assert_eq!(found.user_id, "u1");
        assert_eq!(db.list_devices("u1").unwrap().len(), 1);
        assert!(db.delete_device("u1", "d1").unwrap());
        assert!(db
            .find_device_by_token_hash("hashed-token")
            .unwrap()
            .is_none());
        assert!(
            !db.delete_device("u1", "d1").unwrap(),
            "deleting again reports not found"
        );
    }

    #[test]
    fn pairing_code_is_single_use_and_expiry_checked() {
        let db = Db::in_memory().unwrap();
        db.create_user("u1", "a@example.com", "h", "A", now())
            .unwrap();
        db.create_pairing_code("ABC123", "u1", now(), now() + chrono::Duration::minutes(5))
            .unwrap();
        assert_eq!(
            db.redeem_pairing_code("ABC123", now()).unwrap(),
            Some("u1".to_string())
        );
        assert_eq!(
            db.redeem_pairing_code("ABC123", now()).unwrap(),
            None,
            "cannot redeem twice"
        );

        db.create_pairing_code("XYZ999", "u1", now(), now() - chrono::Duration::seconds(1))
            .unwrap();
        assert_eq!(
            db.redeem_pairing_code("XYZ999", now()).unwrap(),
            None,
            "expired code must not validate"
        );

        assert_eq!(
            db.redeem_pairing_code("NOPE00", now()).unwrap(),
            None,
            "unknown code"
        );
    }
}
