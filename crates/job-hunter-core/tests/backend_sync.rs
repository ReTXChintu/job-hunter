//! End-to-end proof of the desktop-to-backend sync path added in
//! `store/backend.rs` and `store/sync.rs`: a real minimal HTTP server
//! standing in for `@job-hunter/backend` (same auth/devices/data routes,
//! same wire shapes -- see `docs/backend.md` and `docs/mobile-protocol.md`
//! for why the two are kept wire-compatible), and a real `SyncWorker`
//! signing in, pushing local writes, and pulling remote ones. Nothing here
//! is mocked; this is a real HTTP server and a real `reqwest` client.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{json, Value};

use job_hunter_core::domain::*;
use job_hunter_core::AppContext;

/// In-memory stand-in for the Node backend's accounts + generic data
/// collections. Keyed by collection name then document id, exactly like
/// the real backend's per-account scoping (this test only ever uses one
/// account, so scoping itself isn't re-tested here -- that's covered by
/// `apps/backend/test/routes.data.test.ts`).
#[derive(Default)]
struct FakeBackend {
    docs: Mutex<HashMap<String, HashMap<String, Value>>>,
    valid_tokens: Mutex<Vec<String>>,
}

type SharedFake = Arc<FakeBackend>;

fn authorized(headers: &HeaderMap, fake: &FakeBackend) -> bool {
    let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(token) = auth.strip_prefix("Bearer ") else {
        return false;
    };
    // Accept the fixed access token used by /auth/* in this fake, or any
    // minted device token.
    token == "fake-access-token"
        || fake
            .valid_tokens
            .lock()
            .unwrap()
            .contains(&token.to_string())
}

async fn register(State(_fake): State<SharedFake>) -> Json<Value> {
    Json(json!({ "userId": "user-1", "accessToken": "fake-access-token" }))
}

async fn login(State(_fake): State<SharedFake>) -> Json<Value> {
    Json(json!({ "userId": "user-1", "accessToken": "fake-access-token" }))
}

async fn register_device(
    State(fake): State<SharedFake>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if !authorized(&headers, &fake) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let token = "fake-device-token".to_string();
    fake.valid_tokens.lock().unwrap().push(token.clone());
    Ok(Json(
        json!({ "deviceId": "device-1", "deviceToken": token }),
    ))
}

async fn list_devices(State(fake): State<SharedFake>, headers: HeaderMap) -> StatusCode {
    if authorized(&headers, &fake) {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn upsert_doc(
    State(fake): State<SharedFake>,
    headers: HeaderMap,
    Path((collection, id)): Path<(String, String)>,
    Json(value): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if !authorized(&headers, &fake) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    fake.docs
        .lock()
        .unwrap()
        .entry(collection)
        .or_default()
        .insert(id, value);
    Ok(Json(json!({})))
}

async fn delete_doc(
    State(fake): State<SharedFake>,
    headers: HeaderMap,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    if !authorized(&headers, &fake) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if let Some(c) = fake.docs.lock().unwrap().get_mut(&collection) {
        c.remove(&id);
    }
    Ok(Json(json!({})))
}

async fn fetch_all(
    State(fake): State<SharedFake>,
    headers: HeaderMap,
    Path(collection): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    if !authorized(&headers, &fake) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let docs = fake
        .docs
        .lock()
        .unwrap()
        .get(&collection)
        .map(|m| m.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    Ok(Json(json!({ "documents": docs })))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn spawn_fake_backend() -> (String, SharedFake) {
    let fake: SharedFake = Arc::new(FakeBackend::default());
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/devices/register", post(register_device))
        .route("/v1/devices", get(list_devices))
        .route(
            "/v1/data/{collection}/{id}",
            put(upsert_doc).delete(delete_doc),
        )
        .route("/v1/data/{collection}", get(fetch_all))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    (format!("http://{addr}"), fake)
}

async fn desktop() -> Arc<AppContext> {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    std::mem::forget(dir); // keep the temp dir alive for the test's lifetime
    ctx
}

#[tokio::test]
async fn signing_in_pushes_local_data_and_pulls_remote_data() {
    let (base_url, fake) = spawn_fake_backend().await;
    let ctx = desktop().await;

    // AppContext::init spawns SyncWorker::run() as a live background task
    // that pulls/flushes on its own timer. Disable it so this test's own
    // explicit pull_all()/flush() calls are deterministic instead of racing
    // an autonomous loop doing the same work concurrently.
    ctx.sync.set_enabled(false).await;

    // A real local write, made before signing in, exactly like a user who
    // used the app offline first.
    let job = Job::new(
        LOCAL_USER_ID,
        "linkedin",
        "https://example.com/job/1",
        "Acme",
        "Staff Engineer",
    );
    ctx.store.put(&job).unwrap();

    let status = ctx
        .sync
        .configure(
            &base_url,
            "alice@example.com",
            "password1234",
            true,
            "Test Desktop",
            true,
        )
        .await
        .unwrap();
    assert!(status.configured);
    assert!(status.target.unwrap().contains("alice@example.com"));

    // flush() pushes everything enqueue_everything() queued during configure().
    let flushed = ctx.sync.flush().await.unwrap();
    assert!(flushed >= 1, "expected at least the job to be pushed");
    let pushed_jobs = fake
        .docs
        .lock()
        .unwrap()
        .get("jobs")
        .cloned()
        .unwrap_or_default();
    assert_eq!(pushed_jobs.len(), 1);
    assert_eq!(pushed_jobs.values().next().unwrap()["company"], "Acme");

    // A document that exists only on the backend (as if written from
    // another device) is pulled in by pull_all().
    fake.docs.lock().unwrap().entry("jobs".into()).or_default().insert(
        "remote-job-1".into(),
        json!({
            "id": "remote-job-1", "userId": LOCAL_USER_ID, "discoveredAt": "2026-01-01T00:00:00Z",
            "postedAt": null, "source": "linkedin", "sourceJobId": null, "url": "https://example.com/2",
            "canonicalUrl": null, "company": "Globex", "title": "Principal Engineer", "location": "Remote",
            "employmentType": null, "remote": null, "salary": null, "seniority": null, "description": "",
            "requirements": [], "responsibilities": [], "skills": [], "sources": [], "status": "DISCOVERED",
            "runId": null, "analysisId": null, "applicationId": null, "saved": false, "detailsComplete": false,
            "dedupKey": "globex-principal-engineer", "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z",
        }),
    );
    let merged = ctx.sync.pull_all().await.unwrap();
    assert!(merged >= 1);
    let local_jobs = ctx.store.list::<Job>().unwrap();
    assert!(local_jobs.iter().any(|j| j.company == "Globex"));

    // Deleting locally and flushing removes it on the backend too.
    ctx.store.delete::<Job>(&job.id).unwrap();
    ctx.sync.flush().await.unwrap();
    let pushed_jobs_after = fake
        .docs
        .lock()
        .unwrap()
        .get("jobs")
        .cloned()
        .unwrap_or_default();
    assert!(!pushed_jobs_after.contains_key(&job.id));

    ctx.sync.sign_out().await.unwrap();
    assert!(!ctx.sync.status().await.configured);
}

#[tokio::test]
async fn a_revoked_or_wrong_token_surfaces_as_a_clean_error_not_a_panic() {
    let (base_url, _fake) = spawn_fake_backend().await;
    let ctx = desktop().await;
    ctx.sync.set_enabled(false).await;
    ctx.sync
        .configure(
            &base_url,
            "bob@example.com",
            "password1234",
            true,
            "Test Desktop",
            true,
        )
        .await
        .unwrap();

    // Corrupt the stored device token, then force the cached client to be
    // dropped so the next call re-reads it from secrets (the same thing a
    // fresh process start after a revoked token would do).
    ctx.secrets
        .set(
            job_hunter_core::secrets::BACKEND_DEVICE_TOKEN_KEY,
            "not-a-real-token",
        )
        .unwrap();
    ctx.sync.set_backend_url(base_url).await;
    let result = ctx.sync.flush().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connection_pings_without_requiring_credentials() {
    let (base_url, _fake) = spawn_fake_backend().await;
    let ctx = desktop().await;
    ctx.sync.test_connection(&base_url, true).await.unwrap();
    assert!(ctx
        .sync
        .test_connection("http://127.0.0.1:1", true)
        .await
        .is_err());
}
