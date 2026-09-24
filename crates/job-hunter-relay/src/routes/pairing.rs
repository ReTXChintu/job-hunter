//! Pairing codes: the normal way to add a phone. The desktop (already
//! logged in) mints a short-lived, single-use code; the phone redeems it
//! for a device token without ever typing an email or password.

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::auth::{random_pairing_code, random_token_hex, sha256_hex};
use crate::error::{RelayError, RelayResult};
use crate::extractor::AuthUser;
use crate::models::DeviceKind;
use crate::state::AppState;
use crate::util::new_id;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePairingResponse {
    pub code: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub async fn create_pairing_code(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> RelayResult<Json<CreatePairingResponse>> {
    let now = Utc::now();
    let expires_at = now + chrono::Duration::seconds(state.config.pairing_code_ttl_secs);
    // Astronomically unlikely to collide, but retry a couple of times
    // rather than surface an internal error to the user.
    for _ in 0..5 {
        let code = random_pairing_code();
        match state
            .db
            .create_pairing_code(&code, &user_id, now, expires_at)
        {
            Ok(()) => return Ok(Json(CreatePairingResponse { code, expires_at })),
            Err(RelayError::Internal(_)) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(RelayError::Internal(
        "could not generate a unique pairing code".into(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemPairingRequest {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub platform: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemPairingResponse {
    pub device_id: String,
    pub device_token: String,
    pub user_id: String,
}

pub async fn redeem_pairing_code(
    State(state): State<AppState>,
    Json(req): Json<RedeemPairingRequest>,
) -> RelayResult<Json<RedeemPairingResponse>> {
    let code = req.code.trim().to_uppercase();
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(RelayError::Validation(
            "Device name must be 1-80 characters".into(),
        ));
    }
    let user_id = state
        .db
        .redeem_pairing_code(&code, Utc::now())?
        .ok_or(RelayError::InvalidPairingCode)?;
    let token = random_token_hex();
    let device = state.db.create_device(
        &new_id(),
        &user_id,
        name,
        DeviceKind::Mobile,
        req.platform.trim(),
        &sha256_hex(&token),
        Utc::now(),
    )?;
    tracing::info!(user_id = %user_id, device_id = %device.id, "device paired via code");
    Ok(Json(RedeemPairingResponse {
        device_id: device.id,
        device_token: token,
        user_id,
    }))
}
