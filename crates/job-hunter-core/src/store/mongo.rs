use std::time::Duration;

use mongodb::bson::{doc, Bson, Document};
use mongodb::options::{ClientOptions, ReplaceOptions};
use mongodb::{Client, Database};
use serde_json::Value;

use crate::error::{CoreError, CoreResult};
use crate::secrets::redact_mongo_uri;

/// Thin wrapper over the official driver. Documents are stored with `_id`
/// equal to the application id string so upserts are idempotent.
#[derive(Clone)]
pub struct MongoClientHandle {
    db: Database,
    label: String,
}

impl MongoClientHandle {
    pub async fn connect(uri: &str, database: &str) -> CoreResult<Self> {
        let mut options =
            ClientOptions::parse(uri)
                .await
                .map_err(|e| CoreError::MongoUnavailable {
                    message: sanitize(&e.to_string(), uri),
                })?;
        options.app_name = Some("JobHunter".into());
        options.server_selection_timeout = Some(Duration::from_secs(8));
        options.connect_timeout = Some(Duration::from_secs(8));
        let client = Client::with_options(options).map_err(|e| CoreError::MongoUnavailable {
            message: sanitize(&e.to_string(), uri),
        })?;
        let db = client.database(database);
        let handle = Self {
            db,
            label: redact_mongo_uri(uri),
        };
        handle.ping().await?;
        Ok(handle)
    }

    pub async fn ping(&self) -> CoreResult<()> {
        self.db
            .run_command(doc! { "ping": 1 })
            .await
            .map(|_| ())
            .map_err(|e| CoreError::MongoUnavailable {
                message: e.to_string(),
            })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub async fn upsert(&self, collection: &str, id: &str, value: &Value) -> CoreResult<()> {
        let mut document = to_document(value)?;
        document.insert("_id", id);
        self.db
            .collection::<Document>(collection)
            .replace_one(doc! { "_id": id }, document)
            .with_options(ReplaceOptions::builder().upsert(true).build())
            .await
            .map_err(|e| CoreError::MongoUnavailable {
                message: e.to_string(),
            })?;
        Ok(())
    }

    pub async fn delete(&self, collection: &str, id: &str) -> CoreResult<()> {
        self.db
            .collection::<Document>(collection)
            .delete_one(doc! { "_id": id })
            .await
            .map_err(|e| CoreError::MongoUnavailable {
                message: e.to_string(),
            })?;
        Ok(())
    }

    pub async fn fetch_all(&self, collection: &str) -> CoreResult<Vec<Value>> {
        use futures::TryStreamExt;
        let mut cursor = self
            .db
            .collection::<Document>(collection)
            .find(doc! {})
            .await
            .map_err(|e| CoreError::MongoUnavailable {
                message: e.to_string(),
            })?;
        let mut out = Vec::new();
        while let Some(mut d) =
            cursor
                .try_next()
                .await
                .map_err(|e| CoreError::MongoUnavailable {
                    message: e.to_string(),
                })?
        {
            d.remove("_id");
            let v: Value = Bson::Document(d).into();
            out.push(v);
        }
        Ok(out)
    }

    pub async fn ensure_indexes(&self) -> CoreResult<()> {
        use mongodb::IndexModel;
        let jobs = self.db.collection::<Document>("jobs");
        let _ = jobs
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "userId": 1, "dedupKey": 1 })
                    .build(),
            )
            .await;
        let _ = jobs
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "userId": 1, "status": 1 })
                    .build(),
            )
            .await;
        let apps = self.db.collection::<Document>("applications");
        let _ = apps
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "userId": 1, "jobId": 1 })
                    .build(),
            )
            .await;
        let events = self.db.collection::<Document>("agent_events");
        let _ = events
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "runId": 1, "at": 1 })
                    .build(),
            )
            .await;
        Ok(())
    }
}

fn to_document(value: &Value) -> CoreResult<Document> {
    match mongodb::bson::to_bson(value) {
        Ok(Bson::Document(d)) => Ok(d),
        Ok(_) => Err(CoreError::Validation("document is not an object".into())),
        Err(e) => Err(CoreError::Other(format!("bson conversion failed: {e}"))),
    }
}

fn sanitize(message: &str, uri: &str) -> String {
    message.replace(uri, &redact_mongo_uri(uri))
}
