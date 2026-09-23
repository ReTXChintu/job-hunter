use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, Notify, RwLock};

use super::local::{LocalStore, SyncOp};
use super::mongo::MongoClientHandle;
use crate::domain::COLLECTIONS;
use crate::error::{CoreError, CoreResult};
use crate::secrets::{SecretStore, MONGODB_URI_KEY};
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

/// Background worker that mirrors the local store into MongoDB Atlas.
pub struct SyncWorker {
    store: Arc<LocalStore>,
    secrets: Arc<dyn SecretStore>,
    database: RwLock<String>,
    client: Mutex<Option<MongoClientHandle>>,
    status: RwLock<SyncStatus>,
    status_tx: broadcast::Sender<SyncStatus>,
    kick: Notify,
    enabled: RwLock<bool>,
}

impl SyncWorker {
    pub fn new(
        store: Arc<LocalStore>,
        secrets: Arc<dyn SecretStore>,
        database: String,
    ) -> Arc<Self> {
        let (status_tx, _) = broadcast::channel(64);
        Arc::new(Self {
            store,
            secrets,
            database: RwLock::new(database),
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
        self.secrets
            .get(MONGODB_URI_KEY)
            .ok()
            .flatten()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false)
            || std::env::var("MONGODB_URI")
                .map(|u| !u.trim().is_empty())
                .unwrap_or(false)
    }

    fn uri(&self) -> Option<String> {
        self.secrets
            .get(MONGODB_URI_KEY)
            .ok()
            .flatten()
            .filter(|u| !u.trim().is_empty())
            .or_else(|| {
                std::env::var("MONGODB_URI")
                    .ok()
                    .filter(|u| !u.trim().is_empty())
            })
    }

    /// Wake the worker (called after every local write).
    pub fn kick(&self) {
        self.kick.notify_one();
    }

    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
        self.kick();
    }

    pub async fn set_database(&self, database: String) {
        *self.database.write().await = database;
        *self.client.lock().await = None;
        self.kick();
    }

    /// Store a new connection string (in the OS credential store), verify it,
    /// and queue all existing local data for upload.
    pub async fn configure(&self, uri: &str, database: &str) -> CoreResult<SyncStatus> {
        let uri = uri.trim();
        if !(uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://")) {
            return Err(CoreError::Validation(
                "The connection string must start with mongodb:// or mongodb+srv://".into(),
            ));
        }
        let handle = MongoClientHandle::connect(uri, database).await?;
        handle.ensure_indexes().await?;
        self.secrets.set(MONGODB_URI_KEY, uri)?;
        *self.database.write().await = database.to_string();
        *self.client.lock().await = Some(handle.clone());
        self.store.enqueue_everything()?;
        {
            let mut s = self.status.write().await;
            s.configured = true;
            s.connected = true;
            s.last_error = None;
            s.target = Some(handle.label().to_string());
        }
        self.publish().await;
        self.kick();
        Ok(self.status().await)
    }

    pub async fn test_connection(&self, uri: &str, database: &str) -> CoreResult<String> {
        let handle = MongoClientHandle::connect(uri.trim(), database).await?;
        Ok(handle.label().to_string())
    }

    pub async fn clear_configuration(&self) -> CoreResult<()> {
        self.secrets.delete(MONGODB_URI_KEY)?;
        *self.client.lock().await = None;
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

    async fn client(&self) -> CoreResult<MongoClientHandle> {
        let mut guard = self.client.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(c.clone());
        }
        let uri = self.uri().ok_or(CoreError::MongoNotConfigured)?;
        let db = self.database.read().await.clone();
        let handle = MongoClientHandle::connect(&uri, &db).await?;
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
                            tracing::info!(
                                merged = n,
                                "pulled remote documents from MongoDB Atlas"
                            );
                        }
                        Err(e) => {
                            ok = false;
                            self.record_error(&e).await;
                        }
                    }
                }
                if ok {
                    match self.flush().await {
                        Ok(n) => {
                            let mut s = self.status.write().await;
                            s.connected = true;
                            s.last_error = None;
                            s.last_sync_at = Some(now());
                            if let Some(c) = self.client.lock().await.as_ref() {
                                s.target = Some(c.label().to_string());
                            }
                            if n > 0 {
                                tracing::info!(
                                    flushed = n,
                                    "synced local changes to MongoDB Atlas"
                                );
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
        tracing::warn!(error = %e, "MongoDB sync failed; will retry");
        let mut s = self.status.write().await;
        s.connected = false;
        s.last_error = Some(e.user_message());
    }
}
