use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::auth::{random_token_hex, sha256_hex};
use crate::error::{RelayError, RelayResult};
use crate::extractor::AuthUser;
use crate::models::{DeviceKind, DeviceView};
use crate::state::AppState;
use crate::util::new_id;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub platform: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceResponse {
    pub device_id: String,
    pub device_token: String,
}

pub async fn register_device(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RegisterDeviceRequest>,
) -> RelayResult<Json<RegisterDeviceResponse>> {
    let kind = DeviceKind::parse(&req.kind)
        .ok_or_else(|| RelayError::Validation("kind must be \"desktop\" or \"mobile\"".into()))?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(RelayError::Validation(
            "Device name must be 1-80 characters".into(),
        ));
    }
    let token = random_token_hex();
    let device = state.db.create_device(
        &new_id(),
        &user_id,
        name,
        kind,
        req.platform.trim(),
        &sha256_hex(&token),
        Utc::now(),
    )?;
    tracing::info!(user_id = %user_id, device_id = %device.id, kind = kind.as_str(), "device registered");
    Ok(Json(RegisterDeviceResponse {
        device_id: device.id,
        device_token: token,
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDevicesResponse {
    pub devices: Vec<DeviceView>,
}

pub async fn list_devices(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> RelayResult<Json<ListDevicesResponse>> {
    let mut devices = state.db.list_devices(&user_id)?;
    for d in &mut devices {
        d.online = state.hub.is_online(&user_id, &d.id).await;
    }
    Ok(Json(ListDevicesResponse {
        devices: devices.into_iter().map(DeviceView::from).collect(),
    }))
}

pub async fn delete_device(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> RelayResult<Json<serde_json::Value>> {
    let existed = state.db.delete_device(&user_id, &device_id)?;
    if !existed {
        return Err(RelayError::DeviceNotFound);
    }
    state.hub.force_disconnect(&user_id, &device_id).await;
    tracing::info!(user_id = %user_id, device_id = %device_id, "device revoked");
    Ok(Json(serde_json::json!({})))
}
