//! Shared axum extractor for REST endpoints that require a logged-in user
//! (a valid, unexpired access token in the `Authorization: Bearer` header).

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::auth::verify_access_token;
use crate::error::RelayError;
use crate::state::AppState;

/// The authenticated user id, extracted and verified from the request's
/// `Authorization: Bearer <accessToken>` header.
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
        let user_id = verify_access_token(&state.config.jwt_secret, token)?;
        Ok(AuthUser(user_id))
    }
}
