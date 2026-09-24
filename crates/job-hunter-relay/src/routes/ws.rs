//! The one WebSocket endpoint. Desktop and mobile both connect here with
//! `?token=<deviceToken>`; everything after the upgrade is routed through
//! the `Hub`. See `docs/mobile-protocol.md` for the frame format — this
//! file never looks inside a frame's JSON payload, only at its device kind.

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::auth::sha256_hex;
use crate::hub::RegisterOutcome;
use crate::models::DeviceKind;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> axum::response::Response {
    let device = match state
        .db
        .find_device_by_token_hash(&sha256_hex(&query.token))
    {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "invalid device token").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "device lookup failed during ws upgrade");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    ws.on_upgrade(move |socket| {
        handle_socket(socket, state, device.user_id, device.id, device.kind)
    })
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const IDLE_TIMEOUT: Duration = Duration::from_secs(50);

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    user_id: String,
    device_id: String,
    kind: DeviceKind,
) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let outcome = state
        .hub
        .register(&user_id, &device_id, kind, tx.clone())
        .await;
    tracing::debug!(user_id = %user_id, device_id = %device_id, kind = kind.as_str(), "device connected");
    if kind == DeviceKind::Mobile {
        let RegisterOutcome::Registered { desktop_online } = outcome;
        let _ = tx.send(Message::Text(presence_frame(desktop_online).into()));
    }

    let writer = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
        ticker.tick().await; // first tick is immediate; skip it
        loop {
            tokio::select! {
                maybe_msg = rx.recv() => {
                    let Some(msg) = maybe_msg else { break };
                    let is_close = matches!(msg, Message::Close(_));
                    if sink.send(msg).await.is_err() || is_close {
                        break;
                    }
                }
                _ = ticker.tick() => {
                    if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
            }
        }
        let _ = sink.close().await;
    });

    loop {
        match tokio::time::timeout(IDLE_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let _ = state.db.touch_device_last_seen(&device_id, Utc::now());
                match kind {
                    DeviceKind::Mobile => {
                        if !state
                            .hub
                            .route_to_desktop(&user_id, Message::Text(text.clone()))
                            .await
                        {
                            let _ = tx.send(Message::Text(offline_response_frame(&text).into()));
                        }
                    }
                    DeviceKind::Desktop => {
                        state
                            .hub
                            .route_to_mobiles(&user_id, Message::Text(text))
                            .await;
                    }
                }
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => break,
            Ok(Some(Ok(_))) => {
                // Binary/Ping/Pong: counts as liveness only.
                let _ = state.db.touch_device_last_seen(&device_id, Utc::now());
            }
            Ok(Some(Err(_))) | Err(_) => break,
        }
    }

    writer.abort();
    state.hub.unregister(&user_id, &device_id, kind).await;
    tracing::debug!(user_id = %user_id, device_id = %device_id, "device disconnected");
}

fn presence_frame(desktop_online: bool) -> String {
    serde_json::json!({
        "v": 1,
        "id": uuid::Uuid::new_v4().to_string(),
        "type": "presence",
        "payload": { "device": "desktop", "online": desktop_online, "lastSeenAt": Utc::now().to_rfc3339() },
    })
    .to_string()
}

/// Echoes back the request's own `id` (falling back to a fresh one if the
/// inbound frame was not valid JSON) so the mobile client's pending-request
/// map resolves the same way it would for a real desktop reply.
fn offline_response_frame(inbound_text: &str) -> String {
    let id = serde_json::from_str::<serde_json::Value>(inbound_text)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .unwrap_or_else(|| serde_json::Value::String(uuid::Uuid::new_v4().to_string()));
    serde_json::json!({
        "v": 1,
        "id": id,
        "type": "response",
        "payload": { "ok": false, "error": { "code": "DESKTOP_OFFLINE", "message": "Your desktop is not online right now." } },
    })
    .to_string()
}
