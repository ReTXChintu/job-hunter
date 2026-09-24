use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::auth::{
    hash_password, issue_access_token, random_token_hex, sha256_hex, verify_password,
};
use crate::error::{RelayError, RelayResult};
use crate::extractor::AuthUser;
use crate::state::AppState;
use crate::util::new_id;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
}

fn normalize_email(email: &str) -> RelayResult<String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 5 || email.len() > 254 {
        return Err(RelayError::Validation("Enter a valid email address".into()));
    }
    Ok(email)
}

fn validate_password(password: &str) -> RelayResult<()> {
    if password.chars().count() < 8 {
        return Err(RelayError::Validation(
            "Password must be at least 8 characters".into(),
        ));
    }
    Ok(())
}

fn rate_limit_key(headers: &HeaderMap, addr: &SocketAddr, email: &str) -> String {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .unwrap_or(&addr.ip().to_string())
        .to_string();
    format!("{ip}:{}", email.to_lowercase())
}

async fn issue_session(state: &AppState, user_id: &str) -> RelayResult<AuthResponse> {
    let now = Utc::now();
    let access_token = issue_access_token(
        &state.config.jwt_secret,
        user_id,
        state.config.access_token_ttl_secs,
        now,
    )?;
    let refresh_token = random_token_hex();
    state.db.insert_refresh_token(
        &sha256_hex(&refresh_token),
        user_id,
        now,
        now + chrono::Duration::days(state.config.refresh_token_ttl_days),
    )?;
    Ok(AuthResponse {
        user_id: user_id.to_string(),
        access_token,
        refresh_token,
    })
}

pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> RelayResult<Json<AuthResponse>> {
    let email = normalize_email(&req.email)?;
    if !state
        .auth_limiter
        .check(&rate_limit_key(&headers, &addr, &email))
    {
        return Err(RelayError::RateLimited);
    }
    validate_password(&req.password)?;
    let display_name = if req.display_name.trim().is_empty() {
        email.split('@').next().unwrap_or(&email).to_string()
    } else {
        req.display_name.trim().to_string()
    };
    let password_hash = hash_password(&req.password)?;
    let user =
        state
            .db
            .create_user(&new_id(), &email, &password_hash, &display_name, Utc::now())?;
    tracing::info!(user_id = %user.id, "account registered");
    Ok(Json(issue_session(&state, &user.id).await?))
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> RelayResult<Json<AuthResponse>> {
    let email = normalize_email(&req.email)?;
    if !state
        .auth_limiter
        .check(&rate_limit_key(&headers, &addr, &email))
    {
        return Err(RelayError::RateLimited);
    }
    let Some((user, password_hash)) = state.db.find_user_by_email(&email)? else {
        return Err(RelayError::InvalidCredentials);
    };
    if !verify_password(&req.password, &password_hash) {
        return Err(RelayError::InvalidCredentials);
    }
    tracing::info!(user_id = %user.id, "login succeeded");
    Ok(Json(issue_session(&state, &user.id).await?))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> RelayResult<Json<RefreshResponse>> {
    let now = Utc::now();
    let user_id = state
        .db
        .consume_refresh_token(&sha256_hex(&req.refresh_token), now)?
        .ok_or(RelayError::InvalidRefreshToken)?;
    let access_token = issue_access_token(
        &state.config.jwt_secret,
        &user_id,
        state.config.access_token_ttl_secs,
        now,
    )?;
    let refresh_token = random_token_hex();
    state.db.insert_refresh_token(
        &sha256_hex(&refresh_token),
        &user_id,
        now,
        now + chrono::Duration::days(state.config.refresh_token_ttl_days),
    )?;
    Ok(Json(RefreshResponse {
        access_token,
        refresh_token,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> RelayResult<Json<serde_json::Value>> {
    // Consuming (rather than merely looking up) is enough to invalidate it;
    // we don't need to know whether it was valid to report success.
    let _ = state
        .db
        .consume_refresh_token(&sha256_hex(&req.refresh_token), Utc::now());
    Ok(Json(serde_json::json!({})))
}

/// Not part of the public router; kept here so the extractor stays exercised
/// by at least one handler signature during compilation, and available for
/// a future "who am I" endpoint.
#[allow(dead_code)]
pub async fn whoami(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> RelayResult<Json<serde_json::Value>> {
    let user = state
        .db
        .find_user_by_id(&user_id)?
        .ok_or(RelayError::Unauthorized)?;
    Ok(Json(
        serde_json::json!({ "userId": user.id, "email": user.email, "displayName": user.display_name }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalization_rejects_junk() {
        assert_eq!(
            normalize_email(" User@Example.com ").unwrap(),
            "user@example.com"
        );
        assert!(normalize_email("not-an-email").is_err());
        assert!(normalize_email("a@b").is_err());
    }

    #[test]
    fn password_validation_enforces_minimum_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("longenough").is_ok());
    }
}
