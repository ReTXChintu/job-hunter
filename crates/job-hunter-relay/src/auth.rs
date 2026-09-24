//! Password hashing, opaque token generation/hashing, and JWT access tokens.
//!
//! Three different kinds of secret flow through the relay, deliberately
//! handled differently:
//! - **Passwords**: low entropy, user-chosen -> Argon2id (slow, salted).
//! - **Refresh tokens / device tokens**: high entropy, randomly generated ->
//!   SHA-256 is enough (we're hashing to avoid storing the bearer value
//!   itself, not to resist guessing; the token itself is already
//!   unguessable).
//! - **Access tokens**: short-lived, stateless -> signed JWT (HS256).

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{RelayError, RelayResult};

pub fn hash_password(password: &str) -> RelayResult<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| RelayError::Internal(format!("password hashing failed: {e}")))
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), stored_hash)
        .is_ok()
}

fn secure_random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    // Only fails on a broken OS RNG source; there is nothing safer to do
    // than abort token issuance in that case, so we treat it as fatal.
    getrandom::fill(&mut buf).expect("system random number generator is unavailable");
    buf
}

/// A high-entropy opaque token, hex-encoded, suitable for refresh tokens,
/// device tokens and as the WebSocket bearer.
pub fn random_token_hex() -> String {
    hex_encode(&secure_random_bytes::<32>())
}

/// A short, human-typeable pairing code. Excludes visually ambiguous
/// characters (0/O, 1/I/L) since it is meant to be read off one screen and
/// typed on another.
pub fn random_pairing_code() -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
    let buf = secure_random_bytes::<6>();
    buf.iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    iat: i64,
    exp: i64,
}

pub fn issue_access_token(
    secret: &str,
    user_id: &str,
    ttl_secs: i64,
    now: DateTime<Utc>,
) -> RelayResult<String> {
    let claims = AccessClaims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::seconds(ttl_secs)).timestamp(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| RelayError::Internal(format!("token signing failed: {e}")))
}

/// Returns the user id encoded in a valid, unexpired access token.
pub fn verify_access_token(secret: &str, token: &str) -> RelayResult<String> {
    let data = decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| RelayError::Unauthorized)?;
    Ok(data.claims.sub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_round_trips_and_rejects_wrong_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("wrong password", &hash));
        assert!(!hash.contains("correct horse"));
    }

    #[test]
    fn random_tokens_and_codes_are_unique_and_well_formed() {
        let a = random_token_hex();
        let b = random_token_hex();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));

        let code = random_pairing_code();
        assert_eq!(code.len(), 6);
        assert!(code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        assert!(
            !code.contains('0')
                && !code.contains('O')
                && !code.contains('1')
                && !code.contains('I')
                && !code.contains('L')
        );
    }

    #[test]
    fn sha256_hex_is_deterministic() {
        assert_eq!(sha256_hex("abc"), sha256_hex("abc"));
        assert_ne!(sha256_hex("abc"), sha256_hex("abd"));
    }

    #[test]
    fn access_token_round_trips_and_rejects_tampering_and_expiry() {
        let now = Utc::now();
        let token = issue_access_token("secret", "user-1", 60, now).unwrap();
        assert_eq!(verify_access_token("secret", &token).unwrap(), "user-1");
        assert!(verify_access_token("different-secret", &token).is_err());
        // jsonwebtoken applies a default 60s leeway for clock skew, so use
        // an expiry well outside that window rather than "expired by 1s".
        let expired = issue_access_token("secret", "user-1", -120, now).unwrap();
        assert!(verify_access_token("secret", &expired).is_err());
    }
}
