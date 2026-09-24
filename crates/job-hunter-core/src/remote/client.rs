//! Talks to a self-hosted `job-hunter-relay`: registers/logs in this
//! desktop as a device, then maintains one WebSocket connection, answering
//! requests from paired phones (via `dispatch::dispatch`, which reuses the
//! exact same `AppContext`/`orchestrator` calls the Tauri UI uses) and
//! pushing lightweight "something changed" / agent-status notifications.
//!
//! No AI, no Claude, no Chrome and no job data live in this module beyond
//! what a dispatched request already reads through `AppContext` — this is
//! purely the desktop's half of the wire protocol in
//! `docs/mobile-protocol.md`.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, Mutex, Notify, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_util::sync::CancellationToken;

use super::dispatch::dispatch;
use super::protocol::Envelope;
use crate::context::AppContext;
use crate::error::{CoreError, CoreResult};
use crate::secrets::REMOTE_DEVICE_TOKEN_KEY;
use crate::util::now;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    pub configured: bool,
    pub connected: bool,
    pub relay_url: String,
    pub account_email: String,
    pub device_name: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDevice {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub platform: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub online: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    #[allow(dead_code)]
    user_id: String,
    access_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceResponse {
    device_id: String,
    device_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairingCreateResponse {
    code: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct ListDevicesResponse {
    devices: Vec<RemoteDevice>,
}

pub struct RemoteClient {
    app: Arc<AppContext>,
    http: reqwest::Client,
    status: RwLock<RemoteStatus>,
    status_tx: broadcast::Sender<RemoteStatus>,
    kick: Notify,
    cancel_current: Mutex<Option<CancellationToken>>,
}

impl RemoteClient {
    pub fn new(app: Arc<AppContext>) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        let (status_tx, _) = broadcast::channel(32);
        Arc::new(Self {
            app,
            http,
            status: RwLock::new(RemoteStatus::default()),
            status_tx,
            kick: Notify::new(),
            cancel_current: Mutex::new(None),
        })
    }

    /// Live updates whenever the connection or account state changes
    /// (connected/disconnected, signed in/out, an error) -- the desktop UI
    /// forwards these as a Tauri event the same way it does `sync:status`.
    pub fn subscribe(&self) -> broadcast::Receiver<RemoteStatus> {
        self.status_tx.subscribe()
    }

    pub async fn status(&self) -> RemoteStatus {
        let mut s = self.status.read().await.clone();
        let settings = self.app.settings().await;
        s.configured = self.has_device_token();
        s.relay_url = settings.remote.relay_url;
        s.account_email = settings.remote.account_email;
        s.device_name = settings.remote.device_name;
        s
    }

    async fn publish_status(&self) {
        let _ = self.status_tx.send(self.status().await);
    }

    fn has_device_token(&self) -> bool {
        self.app
            .secrets
            .get(REMOTE_DEVICE_TOKEN_KEY)
            .ok()
            .flatten()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn kick(&self) {
        self.kick.notify_one();
    }

    async fn disconnect_current(&self) {
        if let Some(c) = self.cancel_current.lock().await.take() {
            c.cancel();
        }
    }

    fn normalize_relay_url(relay_url: &str, allow_insecure: bool) -> CoreResult<String> {
        let url = relay_url.trim().trim_end_matches('/');
        if url.is_empty() {
            return Err(CoreError::Validation(
                "Enter your relay server's address".into(),
            ));
        }
        if url.starts_with("http://") {
            if !allow_insecure {
                return Err(CoreError::Validation("This address is not encrypted (http://). Use https://, or explicitly allow an insecure address for local testing in Settings.".into()));
            }
        } else if !url.starts_with("https://") {
            return Err(CoreError::Validation("The relay address must start with https:// (or http:// only if you explicitly allow an insecure address for local testing)".into()));
        }
        Ok(url.to_string())
    }

    fn ws_url(relay_url: &str, device_token: &str) -> CoreResult<String> {
        let mut url = url::Url::parse(relay_url)
            .map_err(|e| CoreError::Validation(format!("invalid relay address: {e}")))?;
        let ws_scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(ws_scheme)
            .map_err(|_| CoreError::Validation("invalid relay address".into()))?;
        url.set_path("/v1/ws");
        url.set_query(Some(&format!("token={device_token}")));
        Ok(url.to_string())
    }

    async fn finish_login(
        &self,
        relay_url: &str,
        access_token: &str,
        account_email: &str,
    ) -> CoreResult<()> {
        let settings = self.app.settings().await;
        let device_name = settings.remote.device_name.clone();
        let platform = std::env::consts::OS.to_string();
        let resp = self
            .http
            .post(format!("{relay_url}/v1/devices/register"))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "name": device_name, "kind": "desktop", "platform": platform }))
            .send()
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let device = parse_relay_response::<RegisterDeviceResponse>(resp).await?;

        self.app
            .secrets
            .set(REMOTE_DEVICE_TOKEN_KEY, &device.device_token)?;
        let mut settings = self.app.settings().await;
        settings.remote.relay_url = relay_url.to_string();
        settings.remote.account_email = account_email.to_string();
        settings.remote.device_id = Some(device.device_id);
        self.app.save_settings(settings).await?;
        tracing::info!("signed in to the mobile relay");
        self.kick();
        self.publish_status().await;
        Ok(())
    }

    /// Create a new relay account and register this desktop as its first
    /// device. Fails if a device is already configured (log out first).
    pub async fn register(
        &self,
        relay_url: &str,
        email: &str,
        password: &str,
        allow_insecure: bool,
    ) -> CoreResult<()> {
        if self.has_device_token() {
            return Err(CoreError::Validation(
                "Already signed in to a relay account; sign out first".into(),
            ));
        }
        let relay_url = Self::normalize_relay_url(relay_url, allow_insecure)?;
        let resp = self
            .http
            .post(format!("{relay_url}/v1/auth/register"))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let auth = parse_relay_response::<AuthResponse>(resp).await?;
        self.finish_login(&relay_url, &auth.access_token, email)
            .await
    }

    /// Sign in to an existing relay account and register this desktop as a
    /// new device on it.
    pub async fn login(
        &self,
        relay_url: &str,
        email: &str,
        password: &str,
        allow_insecure: bool,
    ) -> CoreResult<()> {
        if self.has_device_token() {
            return Err(CoreError::Validation(
                "Already signed in to a relay account; sign out first".into(),
            ));
        }
        let relay_url = Self::normalize_relay_url(relay_url, allow_insecure)?;
        let resp = self
            .http
            .post(format!("{relay_url}/v1/auth/login"))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let auth = parse_relay_response::<AuthResponse>(resp).await?;
        self.finish_login(&relay_url, &auth.access_token, email)
            .await
    }

    /// Forgets this desktop's relay credentials. Does not delete the
    /// account or other devices; use `revoke_device` for that.
    pub async fn logout(&self) -> CoreResult<()> {
        self.disconnect_current().await;
        let _ = self.app.secrets.delete(REMOTE_DEVICE_TOKEN_KEY);
        let mut settings = self.app.settings().await;
        settings.remote = Default::default();
        self.app.save_settings(settings).await?;
        {
            let mut s = self.status.write().await;
            s.connected = false;
            s.last_error = None;
        }
        self.kick();
        self.publish_status().await;
        Ok(())
    }

    fn device_token(&self) -> CoreResult<String> {
        self.app
            .secrets
            .get(REMOTE_DEVICE_TOKEN_KEY)?
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| CoreError::Validation("Not signed in to a relay account".into()))
    }

    async fn relay_base(&self) -> CoreResult<String> {
        let url = self.app.settings().await.remote.relay_url;
        if url.trim().is_empty() {
            return Err(CoreError::Validation(
                "Not signed in to a relay account".into(),
            ));
        }
        Ok(url)
    }

    /// Mints a short-lived pairing code the phone can redeem with no
    /// password. Any signed-in device (usually the desktop) may call this.
    pub async fn create_pairing_code(&self) -> CoreResult<(String, chrono::DateTime<chrono::Utc>)> {
        let relay_url = self.relay_base().await?;
        let token = self.device_token()?;
        let resp = self
            .http
            .post(format!("{relay_url}/v1/pairing/create"))
            .bearer_auth(&token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let created = parse_relay_response::<PairingCreateResponse>(resp).await?;
        Ok((created.code, created.expires_at))
    }

    pub async fn list_devices(&self) -> CoreResult<Vec<RemoteDevice>> {
        let relay_url = self.relay_base().await?;
        let token = self.device_token()?;
        let resp = self
            .http
            .get(format!("{relay_url}/v1/devices"))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        Ok(parse_relay_response::<ListDevicesResponse>(resp)
            .await?
            .devices)
    }

    pub async fn revoke_device(&self, device_id: &str) -> CoreResult<()> {
        let relay_url = self.relay_base().await?;
        let token = self.device_token()?;
        let resp = self
            .http
            .delete(format!("{relay_url}/v1/devices/{device_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        parse_relay_response::<serde_json::Value>(resp).await?;
        // If we just revoked our own device, disconnect and forget locally too.
        if self.app.settings().await.remote.device_id.as_deref() == Some(device_id) {
            self.logout().await?;
        }
        Ok(())
    }

    async fn record_error(&self, e: &CoreError) {
        tracing::warn!(error = %e, "mobile relay connection failed; will retry");
        {
            let mut s = self.status.write().await;
            s.connected = false;
            s.last_error = Some(e.user_message());
        }
        self.publish_status().await;
    }

    async fn set_connected(&self, connected: bool) {
        {
            let mut s = self.status.write().await;
            s.connected = connected;
            if connected {
                s.last_error = None;
            }
        }
        self.publish_status().await;
    }

    /// Long-running loop: spawn with `tokio::spawn(client.clone().run())`.
    /// Connects whenever a device token is configured, reconnecting with
    /// backoff on any drop, and does nothing (just waits to be kicked) when
    /// not configured.
    pub async fn run(self: Arc<Self>) {
        let mut backoff = Duration::from_secs(2);
        loop {
            let token = self
                .app
                .secrets
                .get(REMOTE_DEVICE_TOKEN_KEY)
                .ok()
                .flatten()
                .filter(|t| !t.trim().is_empty());
            let relay_url = self.app.settings().await.remote.relay_url;
            match token {
                Some(token) if !relay_url.trim().is_empty() => {
                    match Self::ws_url(&relay_url, &token) {
                        Ok(url) => {
                            let cancel = CancellationToken::new();
                            *self.cancel_current.lock().await = Some(cancel.clone());
                            let result = tokio::select! {
                                r = self.handle_connection(&url) => r,
                                _ = cancel.cancelled() => Ok(()),
                            };
                            *self.cancel_current.lock().await = None;
                            match result {
                                Ok(()) => backoff = Duration::from_secs(2),
                                Err(e) => {
                                    self.record_error(&e).await;
                                    backoff = (backoff * 2).min(Duration::from_secs(60));
                                }
                            }
                        }
                        Err(e) => self.record_error(&e).await,
                    }
                }
                _ => self.set_connected(false).await,
            }
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = self.kick.notified() => { backoff = Duration::from_secs(2); }
            }
        }
    }

    async fn handle_connection(self: &Arc<Self>, ws_url: &str) -> CoreResult<()> {
        let (stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let (mut sink, mut source) = stream.split();
        self.set_connected(true).await;
        tracing::info!("connected to the mobile relay");

        let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

        let push_task = {
            let app = self.app.clone();
            let tx = tx.clone();
            tokio::spawn(async move { forward_changes(app, tx).await })
        };

        let writer = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
            let _ = sink.close().await;
        });

        while let Some(item) = source.next().await {
            match item {
                Ok(WsMessage::Text(text)) => {
                    let Ok(env) = serde_json::from_str::<Envelope>(&text) else {
                        continue;
                    };
                    if env.kind == "presence" || env.kind == "response" {
                        // The desktop never issues its own requests today,
                        // and only cares about phones' presence indirectly
                        // (via their requests); nothing to do here.
                        continue;
                    }
                    let app = self.app.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let id = env.id.clone();
                        let reply = match dispatch(&app, &env.kind, env.payload).await {
                            Ok(data) => Envelope::response_ok(id, data),
                            Err(e) => Envelope::response_err(id, &e.to_user_facing()),
                        };
                        if let Ok(text) = serde_json::to_string(&reply) {
                            let _ = tx.send(WsMessage::Text(text.into()));
                        }
                    });
                }
                Ok(WsMessage::Close(_)) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }

        push_task.abort();
        writer.abort();
        self.set_connected(false).await;
        Ok(())
    }
}

async fn forward_changes(app: Arc<AppContext>, tx: mpsc::UnboundedSender<WsMessage>) {
    let mut store_rx = app.store.subscribe();
    let mut status_rx = app.agent.bus.subscribe_status();
    let mut pending: BTreeSet<String> = BTreeSet::new();
    loop {
        tokio::select! {
            c = store_rx.recv() => match c {
                Ok(collection) => {
                    pending.insert(collection);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    while let Ok(c) = store_rx.try_recv() { pending.insert(c); }
                    let env = Envelope::push("changed", serde_json::json!({ "collections": pending.iter().cloned().collect::<Vec<_>>() }));
                    pending.clear();
                    if !send_envelope(&tx, &env) { break; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
            s = status_rx.recv() => match s {
                Ok(status) => {
                    let trimmed = serde_json::json!({ "state": status.state, "paused": status.paused, "runKind": status.run_kind, "at": now() });
                    let env = Envelope::push("agent_status", trimmed);
                    if !send_envelope(&tx, &env) { break; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
        }
    }
}

fn send_envelope(tx: &mpsc::UnboundedSender<WsMessage>, env: &Envelope) -> bool {
    match serde_json::to_string(env) {
        Ok(text) => tx.send(WsMessage::Text(text.into())).is_ok(),
        Err(_) => true,
    }
}

async fn parse_relay_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> CoreResult<T> {
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    if !status.is_success() {
        #[derive(Deserialize)]
        struct ErrBody {
            error: ErrDetail,
        }
        #[derive(Deserialize)]
        struct ErrDetail {
            message: String,
        }
        let message = serde_json::from_slice::<ErrBody>(&bytes)
            .map(|b| b.error.message)
            .unwrap_or_else(|_| format!("relay returned {status}"));
        return Err(CoreError::Validation(message));
    }
    serde_json::from_slice(&bytes)
        .map_err(|e| CoreError::Other(format!("unexpected response from relay: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_url_is_normalized_and_insecure_urls_are_refused_by_default() {
        assert_eq!(
            RemoteClient::normalize_relay_url("https://relay.example.com/", false).unwrap(),
            "https://relay.example.com"
        );
        assert!(RemoteClient::normalize_relay_url("http://localhost:8787", false).is_err());
        assert_eq!(
            RemoteClient::normalize_relay_url("http://localhost:8787", true).unwrap(),
            "http://localhost:8787"
        );
        assert!(RemoteClient::normalize_relay_url("not-a-url", false).is_err());
        assert!(RemoteClient::normalize_relay_url("  ", false).is_err());
    }

    #[test]
    fn ws_url_swaps_http_scheme_for_ws_and_carries_the_token() {
        let url = RemoteClient::ws_url("https://relay.example.com", "tok123").unwrap();
        assert_eq!(url, "wss://relay.example.com/v1/ws?token=tok123");
        let url = RemoteClient::ws_url("http://localhost:8787", "tok123").unwrap();
        assert_eq!(url, "ws://localhost:8787/v1/ws?token=tok123");
    }
}
