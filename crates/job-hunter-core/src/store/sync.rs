use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, Notify, RwLock};

use super::backend::{self, BackendClientHandle};
use super::local::{LocalStore, SyncOp};
use crate::domain::COLLECTIONS;
use crate::error::{CoreError, CoreResult};
use crate::secrets::{SecretStore, BACKEND_DEVICE_TOKEN_KEY};
use crate::util::now;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub configured: bool,
    pub connected: bool,
    pub pending: usize,
    #[serde(default)]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    pub syncing: bool,
}

/// Background worker that mirrors the local store into a self-hosted
/// `@job-hunter/backend` account (see `apps/backend`, `docs/backend.md`).
pub struct SyncWorker {
    store: Arc<LocalStore>,
    secrets: Arc<dyn SecretStore>,
    backend_url: RwLock<String>,
    account_email: RwLock<String>,
    client: Mutex<Option<BackendClientHandle>>,
    status: RwLock<SyncStatus>,
    status_tx: broadcast::Sender<SyncStatus>,
    kick: Notify,
    enabled: RwLock<bool>,
}

impl SyncWorker {
    pub fn new(
        store: Arc<LocalStore>,
        secrets: Arc<dyn SecretStore>,
        backend_url: String,
        account_email: String,
    ) -> Arc<Self> {
        let (status_tx, _) = broadcast::channel(64);
        Arc::new(Self {
            store,
            secrets,
            backend_url: RwLock::new(backend_url),
            account_email: RwLock::new(account_email),
            client: Mutex::new(None),
            status: RwLock::new(SyncStatus::default()),
            status_tx,
            kick: Notify::new(),
            enabled: RwLock::new(true),
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SyncStatus> {
        self.status_tx.subscribe()
    }

    pub async fn status(&self) -> SyncStatus {
        let mut s = self.status.read().await.clone();
        s.pending = self.store.pending_count();
        s.configured = self.is_configured();
        s
    }

    pub fn is_configured(&self) -> bool {
        self.token().is_some()
    }

    fn token(&self) -> Option<String> {
        self.secrets
            .get(BACKEND_DEVICE_TOKEN_KEY)
            .ok()
            .flatten()
            .filter(|t| !t.trim().is_empty())
    }

    /// Wake the worker (called after every local write).
    pub fn kick(&self) {
        self.kick.notify_one();
    }

    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
        self.kick();
    }

    pub async fn set_backend_url(&self, backend_url: String) {
        *self.backend_url.write().await = backend_url;
        *self.client.lock().await = None;
        self.kick();
    }

    pub async fn set_account_email(&self, account_email: String) {
        *self.account_email.write().await = account_email;
    }

    /// Pings a backend address with no credentials, to confirm it's
    /// reachable before the caller commits an email/password to it.
    pub async fn test_connection(&self, backend_url: &str, allow_insecure: bool) -> CoreResult<()> {
        let backend_url = backend::normalize_backend_url(backend_url, allow_insecure)?;
        let http = http_client();
        backend::ping_backend(&http, &backend_url).await
    }

    /// Registers a new account (`is_register: true`) or signs in to an
    /// existing one, registers this desktop as a device on it, stores the
    /// resulting device token, and queues all existing local data for
    /// upload.
    #[allow(clippy::too_many_arguments)]
    pub async fn configure(
        &self,
        backend_url: &str,
        email: &str,
        password: &str,
        is_register: bool,
        device_name: &str,
        allow_insecure: bool,
    ) -> CoreResult<SyncStatus> {
        let backend_url = backend::normalize_backend_url(backend_url, allow_insecure)?;
        let http = http_client();
        let access_token = if is_register {
            backend::register_account(&http, &backend_url, email, password).await?
        } else {
            backend::login_account(&http, &backend_url, email, password).await?
        };
        let (_device_id, device_token) =
            backend::register_device(&http, &backend_url, &access_token, device_name).await?;
        self.secrets.set(BACKEND_DEVICE_TOKEN_KEY, &device_token)?;
        *self.backend_url.write().await = backend_url.clone();
        *self.account_email.write().await = email.to_string();
        let label = backend::label_for(&backend_url, email);
        let handle = BackendClientHandle::new(http, backend_url, device_token, label.clone());
        *self.client.lock().await = Some(handle);
        self.store.enqueue_everything()?;
        {
            let mut s = self.status.write().await;
            s.configured = true;
            s.connected = true;
            s.last_error = None;
            s.target = Some(label);
        }
        self.publish().await;
        self.kick();
        Ok(self.status().await)
    }

    /// Forgets this desktop's backend credentials locally. Does not delete
    /// the account or other devices.
    pub async fn sign_out(&self) -> CoreResult<()> {
        self.secrets.delete(BACKEND_DEVICE_TOKEN_KEY)?;
        *self.client.lock().await = None;
        *self.backend_url.write().await = String::new();
        *self.account_email.write().await = String::new();
        let mut s = self.status.write().await;
        s.configured = false;
        s.connected = false;
        s.target = None;
        drop(s);
        self.publish().await;
        Ok(())
    }

    async fn publish(&self) {
        let s = self.status().await;
        let _ = self.status_tx.send(s);
    }

    async fn client(&self) -> CoreResult<BackendClientHandle> {
        let mut guard = self.client.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(c.clone());
        }
        let token = self.token().ok_or(CoreError::BackendNotConfigured)?;
        let backend_url = self.backend_url.read().await.clone();
        if backend_url.trim().is_empty() {
            return Err(CoreError::BackendNotConfigured);
        }
        let email = self.account_email.read().await.clone();
        let label = backend::label_for(&backend_url, &email);
        let handle = BackendClientHandle::new(http_client(), backend_url, token, label);
        *guard = Some(handle.clone());
        Ok(handle)
    }

    /// Pull every remote document and merge the newer ones into the local
    /// store. Called once after connecting.
    pub async fn pull_all(&self) -> CoreResult<usize> {
        let client = self.client().await?;
        let mut merged = 0;
        for name in COLLECTIONS {
            let docs = client.fetch_all(name).await?;
            for d in docs {
                if self.store.merge_remote(name, d)? {
                    merged += 1;
                }
            }
        }
        Ok(merged)
    }

    /// Push queued operations. Returns the number of operations flushed.
    pub async fn flush(&self) -> CoreResult<usize> {
        let client = self.client().await?;
        let ops = self.store.pending_ops();
        let mut flushed = 0;
        for op in ops {
            let result = match op.op {
                SyncOp::Upsert => match self.store.get_raw(&op.collection, &op.id) {
                    Some(doc) => client.upsert(&op.collection, &op.id, &doc).await,
                    None => Ok(()), // deleted locally since; the delete op will follow
                },
                SyncOp::Delete => client.delete(&op.collection, &op.id).await,
            };
            match result {
                Ok(()) => {
                    self.store.ack(&op)?;
                    flushed += 1;
                }
                Err(e) => {
                    self.store.bump_attempt(&op);
                    *self.client.lock().await = None;
                    return Err(e);
                }
            }
        }
        Ok(flushed)
    }

    /// A lightweight authenticated request that tells the server this
    /// desktop is running (see `apps/backend/src/presence.ts`), and confirms
    /// the server is reachable and the device token still valid. The run
    /// loop sends one whenever a cycle has nothing else to send, i.e. about
    /// once a minute while idle.
    pub async fn heartbeat(&self) -> CoreResult<()> {
        let result = self.client().await?.ping().await;
        if result.is_err() {
            *self.client.lock().await = None;
        }
        result
    }

    /// Long-running loop. Spawn with `tokio::spawn(worker.run())`.
    pub async fn run(self: Arc<Self>) {
        let mut pulled_once = false;
        let mut backoff = Duration::from_secs(15);
        loop {
            let enabled = *self.enabled.read().await;
            if enabled && self.is_configured() {
                {
                    let mut s = self.status.write().await;
                    s.syncing = true;
                }
                self.publish().await;
                let mut ok = true;
                if !pulled_once {
                    match self.pull_all().await {
                        Ok(n) => {
                            pulled_once = true;
                            tracing::info!(merged = n, "pulled remote documents from the backend");
                        }
                        Err(e) => {
                            ok = false;
                            self.record_error(&e).await;
                        }
                    }
                }
                if ok {
                    let result = match self.flush().await {
                        // Nothing to send: check in anyway, so the server
                        // knows this desktop is running (sync holds no open
                        // connection that would show it otherwise).
                        Ok(0) => self.heartbeat().await.map(|()| 0),
                        other => other,
                    };
                    match result {
                        Ok(n) => {
                            let mut s = self.status.write().await;
                            s.connected = true;
                            s.last_error = None;
                            s.last_sync_at = Some(now());
                            if let Some(c) = self.client.lock().await.as_ref() {
                                s.target = Some(c.label().to_string());
                            }
                            if n > 0 {
                                tracing::info!(flushed = n, "synced local changes to the backend");
                            }
                            backoff = Duration::from_secs(15);
                        }
                        Err(e) => {
                            ok = false;
                            self.record_error(&e).await;
                        }
                    }
                }
                {
                    let mut s = self.status.write().await;
                    s.syncing = false;
                }
                self.publish().await;
                if !ok {
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => {},
                        _ = self.kick.notified() => {},
                    }
                    backoff = (backoff * 2).min(Duration::from_secs(300));
                    continue;
                }
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {},
                _ = self.kick.notified() => {},
            }
        }
    }

    async fn record_error(&self, e: &CoreError) {
        tracing::warn!(error = %e, "backend sync failed; will retry");
        let mut s = self.status.write().await;
        s.connected = false;
        s.last_error = Some(e.user_message());
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default()
}
