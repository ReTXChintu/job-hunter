//! Shared axum extractor for REST endpoints that require a logged-in user.
//!
//! Accepts either a short-lived JWT access token (fresh from `/auth/login`
//! or `/auth/register`) or a long-lived device token, in the same
//! `Authorization: Bearer <token>` header. Both resolve to the same
//! account id. In practice this means neither app needs to hold on to a
//! password or refresh token past its very first login: once a device is
//! registered, its device token alone is enough to call `/devices` and
//! `/pairing/create` from then on, exactly as `docs/mobile-protocol.md`
//! describes.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::auth::{sha256_hex, verify_access_token};
use crate::error::RelayError;
use crate::state::AppState;

/// The authenticated user id, resolved from either token kind.
pub struct AuthUser(pub String);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = RelayError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(RelayError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(RelayError::Unauthorized)?;
        if let Ok(user_id) = verify_access_token(&state.config.jwt_secret, token) {
            return Ok(AuthUser(user_id));
        }
        match state.db.find_device_by_token_hash(&sha256_hex(token))? {
            Some(device) => Ok(AuthUser(device.user_id)),
            None => Err(RelayError::Unauthorized),
        }
    }
}
