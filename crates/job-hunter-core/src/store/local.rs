use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

use crate::domain::{Entity, COLLECTIONS};
use crate::error::{CoreError, CoreResult};
use crate::util::now;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyncOp {
    Upsert,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingOp {
    pub collection: String,
    pub id: String,
    pub op: SyncOp,
    pub queued_at: DateTime<Utc>,
    #[serde(default)]
    pub attempts: u32,
}

#[derive(Default)]
struct CollectionData {
    docs: BTreeMap<String, Value>,
}

/// JSON-file backed document store with an in-memory cache and a persisted
/// outbound sync queue.
pub struct LocalStore {
    dir: PathBuf,
    collections: RwLock<HashMap<String, CollectionData>>,
    queue: RwLock<BTreeMap<String, PendingOp>>,
    changes: broadcast::Sender<String>,
}

impl LocalStore {
    pub fn open(dir: &Path) -> CoreResult<Self> {
        std::fs::create_dir_all(dir)?;
        let mut collections = HashMap::new();
        for name in COLLECTIONS {
            let path = dir.join(format!("{name}.json"));
            let docs = if path.exists() {
                let raw = std::fs::read_to_string(&path)?;
                match serde_json::from_str::<Vec<Value>>(&raw) {
                    Ok(list) => list
                        .into_iter()
                        .filter_map(|v| {
                            v.get("id")
                                .and_then(|i| i.as_str())
                                .map(|id| (id.to_string(), v.clone()))
                        })
                        .collect(),
                    Err(e) => {
                        tracing::error!(collection = name, error = %e, "corrupt collection file, keeping a backup");
                        let backup = dir.join(format!("{name}.corrupt-{}.json", now().timestamp()));
                        let _ = std::fs::copy(&path, backup);
                        BTreeMap::new()
                    }
                }
            } else {
                BTreeMap::new()
            };
            collections.insert(name.to_string(), CollectionData { docs });
        }
        let queue_path = dir.join("_sync_queue.json");
        let queue: BTreeMap<String, PendingOp> = if queue_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&queue_path)?).unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        let (changes, _) = broadcast::channel(256);
        Ok(Self {
            dir: dir.to_path_buf(),
            collections: RwLock::new(collections),
            queue: RwLock::new(queue),
            changes,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.changes.subscribe()
    }

    fn notify(&self, collection: &str) {
        let _ = self.changes.send(collection.to_string());
    }

    fn write_collection(&self, name: &str, data: &CollectionData) -> CoreResult<()> {
        let path = self.dir.join(format!("{name}.json"));
        let tmp = self.dir.join(format!("{name}.json.tmp"));
        let list: Vec<&Value> = data.docs.values().collect();
        std::fs::write(&tmp, serde_json::to_vec(&list)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn write_queue(&self, queue: &BTreeMap<String, PendingOp>) -> CoreResult<()> {
        let path = self.dir.join("_sync_queue.json");
        let tmp = self.dir.join("_sync_queue.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(queue)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn enqueue(&self, collection: &str, id: &str, op: SyncOp) -> CoreResult<()> {
        let mut q = self.queue.write().unwrap();
        q.insert(
            format!("{collection}:{id}"),
            PendingOp {
                collection: collection.into(),
                id: id.into(),
                op,
                queued_at: now(),
                attempts: 0,
            },
        );
        self.write_queue(&q)
    }

    // ---- raw API (used by sync) -------------------------------------------------

    pub fn upsert_raw(
        &self,
        collection: &str,
        value: Value,
        queue_for_sync: bool,
    ) -> CoreResult<()> {
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Validation("document has no id".into()))?
            .to_string();
        {
            let mut cols = self.collections.write().unwrap();
            let col = cols.entry(collection.to_string()).or_default();
            col.docs.insert(id.clone(), value);
            self.write_collection(collection, col)?;
        }
        if queue_for_sync {
            self.enqueue(collection, &id, SyncOp::Upsert)?;
        }
        self.notify(collection);
        Ok(())
    }

    pub fn get_raw(&self, collection: &str, id: &str) -> Option<Value> {
        let cols = self.collections.read().unwrap();
        cols.get(collection).and_then(|c| c.docs.get(id).cloned())
    }

    pub fn list_raw(&self, collection: &str) -> Vec<Value> {
        let cols = self.collections.read().unwrap();
        cols.get(collection)
            .map(|c| c.docs.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn delete_raw(&self, collection: &str, id: &str, queue_for_sync: bool) -> CoreResult<bool> {
        let removed = {
            let mut cols = self.collections.write().unwrap();
            let col = cols.entry(collection.to_string()).or_default();
            let removed = col.docs.remove(id).is_some();
            if removed {
                self.write_collection(collection, col)?;
            }
            removed
        };
        if removed {
            if queue_for_sync {
                self.enqueue(collection, id, SyncOp::Delete)?;
            }
            self.notify(collection);
        }
        Ok(removed)
    }

    /// Merge a remote document: only applied when it is newer than the local
    /// copy (by `updatedAt`) or the local copy is missing.
    pub fn merge_remote(&self, collection: &str, value: Value) -> CoreResult<bool> {
        let Some(id) = value.get("id").and_then(|v| v.as_str()).map(String::from) else {
            return Ok(false);
        };
        let remote_ts = updated_at_of(&value);
        let local_ts = self
            .get_raw(collection, &id)
            .as_ref()
            .and_then(updated_at_of);
        let apply = match (remote_ts, local_ts) {
            (_, None) => true,
            (Some(r), Some(l)) => r > l,
            (None, Some(_)) => false,
        };
        if apply {
            self.upsert_raw(collection, value, false)?;
        }
        Ok(apply)
    }

    // ---- typed API ---------------------------------------------------------------

    pub fn put<T: Entity>(&self, doc: &T) -> CoreResult<()> {
        let value = serde_json::to_value(doc)?;
        self.upsert_raw(T::COLLECTION, value, true)
    }

    pub fn get<T: Entity>(&self, id: &str) -> CoreResult<Option<T>> {
        match self.get_raw(T::COLLECTION, id) {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    pub fn require<T: Entity>(&self, id: &str) -> CoreResult<T> {
        self.get::<T>(id)?.ok_or_else(|| CoreError::NotFound {
            entity: T::COLLECTION,
            id: id.to_string(),
        })
    }

    pub fn list<T: Entity>(&self) -> CoreResult<Vec<T>> {
        let mut out = Vec::new();
        for v in self.list_raw(T::COLLECTION) {
            match serde_json::from_value::<T>(v.clone()) {
                Ok(doc) => out.push(doc),
                Err(e) => {
                    tracing::warn!(collection = T::COLLECTION, error = %e, "skipping undecodable document")
                }
            }
        }
        Ok(out)
    }

    pub fn find<T: Entity>(&self, pred: impl Fn(&T) -> bool) -> CoreResult<Vec<T>> {
        Ok(self.list::<T>()?.into_iter().filter(|d| pred(d)).collect())
    }

    pub fn delete<T: Entity>(&self, id: &str) -> CoreResult<bool> {
        self.delete_raw(T::COLLECTION, id, true)
    }

    pub fn count(&self, collection: &str) -> usize {
        let cols = self.collections.read().unwrap();
        cols.get(collection).map(|c| c.docs.len()).unwrap_or(0)
    }

    // ---- sync queue ----------------------------------------------------------------

    pub fn pending_ops(&self) -> Vec<PendingOp> {
        self.queue.read().unwrap().values().cloned().collect()
    }

    pub fn pending_count(&self) -> usize {
        self.queue.read().unwrap().len()
    }

    pub fn ack(&self, op: &PendingOp) -> CoreResult<()> {
        let mut q = self.queue.write().unwrap();
        let key = format!("{}:{}", op.collection, op.id);
        // Only remove if the queued op is still the same one (a newer write may
        // have replaced it while we were syncing).
        if q.get(&key)
            .map(|p| p.queued_at == op.queued_at)
            .unwrap_or(false)
        {
            q.remove(&key);
            self.write_queue(&q)?;
        }
        Ok(())
    }

    pub fn bump_attempt(&self, op: &PendingOp) {
        let mut q = self.queue.write().unwrap();
        if let Some(p) = q.get_mut(&format!("{}:{}", op.collection, op.id)) {
            p.attempts += 1;
        }
        let _ = self.write_queue(&q);
    }

    /// Queue every local document for upload (used after MongoDB is first
    /// configured so existing local data reaches Atlas).
    pub fn enqueue_everything(&self) -> CoreResult<usize> {
        let mut n = 0;
        let names: Vec<String> = self.collections.read().unwrap().keys().cloned().collect();
        for name in names {
            for id in self
                .collections
                .read()
                .unwrap()
                .get(&name)
                .map(|c| c.docs.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
            {
                self.enqueue(&name, &id, SyncOp::Upsert)?;
                n += 1;
            }
        }
        Ok(n)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

fn updated_at_of(v: &Value) -> Option<DateTime<Utc>> {
    let s = v
        .get("updatedAt")
        .or_else(|| v.get("at"))
        .and_then(|x| x.as_str())?;
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Job, JobStatus};

    #[test]
    fn put_get_list_delete_and_queue() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::open(dir.path()).unwrap();
        let job = Job::new("u", "LinkedIn", "https://x/1", "Acme", "Engineer");
        store.put(&job).unwrap();
        assert_eq!(store.pending_count(), 1);
        let got: Job = store.require(&job.id).unwrap();
        assert_eq!(got.company, "Acme");
        assert_eq!(store.list::<Job>().unwrap().len(), 1);

        // reopen from disk
        let store2 = LocalStore::open(dir.path()).unwrap();
        assert_eq!(store2.list::<Job>().unwrap().len(), 1);
        assert_eq!(store2.pending_count(), 1);
        let ops = store2.pending_ops();
        store2.ack(&ops[0]).unwrap();
        assert_eq!(store2.pending_count(), 0);

        assert!(store2.delete::<Job>(&job.id).unwrap());
        assert_eq!(store2.pending_ops()[0].op, SyncOp::Delete);
    }

    #[test]
    fn merge_remote_respects_updated_at() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::open(dir.path()).unwrap();
        let mut job = Job::new("u", "LinkedIn", "https://x/1", "Acme", "Engineer");
        store.put(&job).unwrap();
        // older remote copy is ignored
        let mut older = serde_json::to_value(&job).unwrap();
        older["updatedAt"] = Value::String("2000-01-01T00:00:00Z".into());
        older["company"] = Value::String("Old".into());
        assert!(!store.merge_remote("jobs", older).unwrap());
        // newer remote copy wins
        job.company = "Newer".into();
        job.status = JobStatus::Analyzed;
        job.updated_at = now() + chrono::Duration::seconds(5);
        assert!(store
            .merge_remote("jobs", serde_json::to_value(&job).unwrap())
            .unwrap());
        let got: Job = store.require(&job.id).unwrap();
        assert_eq!(got.company, "Newer");
        // remote merges are not re-queued
        assert_eq!(store.pending_count(), 1);
    }
}
