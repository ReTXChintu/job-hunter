//! End-to-end proof of the mobile companion path: a real
//! `job-hunter-relay` server, a real desktop `RemoteClient` connected to
//! it, and a raw WebSocket standing in for the phone. Nothing here is
//! mocked except Claude (via the same `MockClaudeRunner` the rest of the
//! test suite uses) -- the relay, both WebSocket connections, and the
//! approval-gated orchestrator calls are all real.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, Stream, StreamExt};
use job_hunter_core::domain::*;
use job_hunter_core::remote::RemoteClient;
use job_hunter_core::AppContext;
use job_hunter_relay::{build_router, AppState as RelayState, Db as RelayDb, RelayConfig};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message as WsMessage;

async fn spawn_relay() -> String {
    let config = RelayConfig {
        bind_addr: "127.0.0.1:0".into(),
        database_path: PathBuf::from("unused-in-memory-db"),
        jwt_secret: "test-secret-do-not-use-in-production".into(),
        access_token_ttl_secs: 900,
        refresh_token_ttl_days: 30,
        pairing_code_ttl_secs: 300,
        cors_origins: vec![],
    };
    let db = RelayDb::in_memory().unwrap();
    let state = RelayState::new(config, db);
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}")
}

async fn desktop_with_one_ready_application() -> Arc<AppContext> {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    std::mem::forget(dir); // keep the temp dir alive for the test's lifetime

    let fixture = job_hunter_core::claude::MockClaudeRunner::fixture_candidate();
    let mut profile: CandidateProfile = serde_json::from_value(fixture["profile"].clone()).unwrap();
    profile.user_id = LOCAL_USER_ID.into();
    ctx.save_profile(profile).unwrap();

    let mut job = Job::new(
        LOCAL_USER_ID,
        "LinkedIn",
        "https://example.test/jobs/1",
        "Acme",
        "Senior Full Stack Developer",
    );
    job.status = JobStatus::ReadyForReview;
    ctx.store.put(&job).unwrap();
    let mut application = Application::new(LOCAL_USER_ID, &job.id, &job.url, &job.source);
    application
        .transition(ApplicationStatus::Analyzed, "seed")
        .unwrap();
    application
        .transition(ApplicationStatus::ReadyForReview, "seed")
        .unwrap();
    ctx.store.put(&application).unwrap();
    ctx
}

async fn next_json<S>(ws: &mut S) -> Value
where
    S: Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let item = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for a frame")
            .expect("stream ended")
            .expect("websocket error");
        if let WsMessage::Text(t) = item {
            return serde_json::from_str(&t).expect("frame was not valid JSON");
        }
    }
}

/// Waits for the `response` frame matching `expected_id`, discarding any
/// `presence`/`changed`/`agent_status` pushes in between -- exactly what a
/// real client's pending-request map would do, and necessary here because
/// the background apply task can legitimately emit a `changed` push before
/// (or interleaved with) the direct reply to a request.
async fn recv_response<S>(ws: &mut S, expected_id: &str) -> Value
where
    S: Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let v = next_json(ws).await;
        if v["type"] == "response" && v["id"] == expected_id {
            return v;
        }
    }
}

/// Redeems a pairing code as a raw HTTP client, the way the Flutter app
/// would, without depending on any mobile-side code.
async fn redeem_pairing_code(relay_http: &str, code: &str) -> String {
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{relay_http}/v1/pairing/redeem"))
        .json(&json!({ "code": code, "name": "Test Phone", "platform": "test" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    resp["deviceToken"].as_str().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phone_sees_presence_lists_and_approves_an_application_through_a_real_relay() {
    let relay_http = spawn_relay().await;
    let relay_ws = relay_http.replacen("http://", "ws://", 1);
    let ctx = desktop_with_one_ready_application().await;

    // Configure and sign the desktop up to the relay, then start its
    // background connection.
    let remote = RemoteClient::new(ctx.clone());
    remote
        .register(&relay_http, "desktop@example.com", "hunter2222", true)
        .await
        .expect("desktop registration should succeed");
    let run_handle = tokio::spawn(remote.clone().run());

    // Wait for the desktop's RemoteClient to actually be connected before
    // pairing a phone, so the first presence push the phone gets is "online".
    for _ in 0..50 {
        if remote.status().await.connected {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        remote.status().await.connected,
        "desktop should be connected to the relay"
    );

    // Pair a "phone" with a one-time code, exactly like the real app would.
    let (code, _expires_at) = remote.create_pairing_code().await.unwrap();
    let mobile_token = redeem_pairing_code(&relay_http, &code).await;

    let (mut mobile_ws, _) =
        tokio_tungstenite::connect_async(format!("{relay_ws}/v1/ws?token={mobile_token}"))
            .await
            .unwrap();
    let presence = next_json(&mut mobile_ws).await;
    assert_eq!(presence["type"], "presence");
    assert_eq!(
        presence["payload"]["online"], true,
        "the phone should see the desktop as online"
    );

    // 1. list_applications over the real wire reaches the real desktop data.
    mobile_ws
        .send(WsMessage::Text(
            json!({"v": 1, "id": "req-list", "type": "list_applications", "payload": {}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let reply = recv_response(&mut mobile_ws, "req-list").await;
    assert_eq!(reply["payload"]["ok"], true);
    let apps = reply["payload"]["data"].as_array().unwrap();
    assert_eq!(apps.len(), 1);
    let application_id = apps[0]["application"]["id"].as_str().unwrap().to_string();
    assert_eq!(apps[0]["application"]["status"], "READY_FOR_REVIEW");

    // 2. approve_application over the real wire actually approves it in the
    //    real AppContext -- through the same orchestrator/approval gate the
    //    desktop UI uses, not a shortcut. The response comes back as soon as
    //    the Approved transition lands (matching the desktop's own
    //    "kick off and return" behaviour); the automatic apply that follows
    //    runs in the background, the same as clicking Approve & Apply does.
    mobile_ws.send(WsMessage::Text(json!({"v": 1, "id": "req-approve", "type": "approve_application", "payload": {"id": application_id}}).to_string().into())).await.unwrap();
    let reply = recv_response(&mut mobile_ws, "req-approve").await;
    assert_eq!(
        reply["payload"]["ok"], true,
        "approve should succeed: {reply}"
    );
    assert_eq!(reply["payload"]["data"]["status"], "APPROVED");

    let stored: Application = ctx.store.require(&application_id).unwrap();
    assert!(
        stored.approved_at.is_some(),
        "the approval must be reflected in the real store, not just echoed back"
    );

    // The background apply (mock mode never submits) eventually lands on
    // MANUAL_ACTION_REQUIRED, same as the desktop's own Agent page would show.
    let mut final_status = stored.status;
    for _ in 0..100 {
        if !ctx.agent.status().await.state.is_running() {
            final_status = ctx
                .store
                .require::<Application>(&application_id)
                .unwrap()
                .status;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        final_status,
        ApplicationStatus::ManualActionRequired,
        "mock apply never submits, so approval leads here"
    );

    // 3. An unknown application id fails the same way it would from the
    //    desktop UI -- the phone cannot bypass validation.
    mobile_ws.send(WsMessage::Text(json!({"v": 1, "id": "req-bad", "type": "get_application", "payload": {"id": "does-not-exist"}}).to_string().into())).await.unwrap();
    let reply = recv_response(&mut mobile_ws, "req-bad").await;
    assert_eq!(reply["payload"]["ok"], false);
    assert_eq!(reply["payload"]["error"]["code"], "NOT_FOUND");

    // 4. The desktop's own device list (as Settings would show it) lists
    //    both devices, and revoking the phone closes its live socket.
    let devices = remote.list_devices().await.unwrap();
    assert_eq!(devices.len(), 2);
    let phone_device_id = devices
        .iter()
        .find(|d| d.kind == "mobile")
        .unwrap()
        .id
        .clone();
    remote.revoke_device(&phone_device_id).await.unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(2), mobile_ws.next())
        .await
        .unwrap();
    assert!(
        matches!(closed, Some(Ok(WsMessage::Close(_))) | None),
        "revoked phone's socket should close, got {closed:?}"
    );

    run_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_forgets_local_credentials_and_stops_reconnecting() {
    let relay_http = spawn_relay().await;
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let remote = RemoteClient::new(ctx.clone());
    remote
        .register(&relay_http, "solo@example.com", "hunter2222", true)
        .await
        .unwrap();
    assert!(remote.status().await.configured);

    remote.logout().await.unwrap();
    let status = remote.status().await;
    assert!(!status.configured);
    assert!(status.relay_url.is_empty());
    // The account still exists on the relay; signing back in (not
    // registering again) reconnects it with a fresh device registration.
    remote
        .login(&relay_http, "solo@example.com", "hunter2222", true)
        .await
        .unwrap();
    assert!(remote.status().await.configured);
}
