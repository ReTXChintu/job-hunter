//! End-to-end test against a real, in-process axum server: register a
//! desktop account, pair a phone with a one-time code (no password typed on
//! the phone), connect both over real WebSockets, and prove presence,
//! request/response routing, offline handling, and device revocation all
//! work over the wire — not just against the in-memory `Hub` unit tests.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use futures_util::{SinkExt, Stream, StreamExt};
use job_hunter_relay::{build_router, AppState, Db, RelayConfig};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message as WsMessage;

async fn spawn_server() -> (String, String) {
    let config = RelayConfig {
        bind_addr: "127.0.0.1:0".into(),
        database_path: PathBuf::from("unused-in-memory-db"),
        jwt_secret: "test-secret-do-not-use-in-production".into(),
        access_token_ttl_secs: 900,
        refresh_token_ttl_days: 30,
        pairing_code_ttl_secs: 300,
        cors_origins: vec![],
    };
    let db = Db::in_memory().unwrap();
    let state = AppState::new(config, db);
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
    (format!("http://{addr}"), format!("ws://{addr}"))
}

async fn next_text<S>(ws: &mut S) -> Value
where
    S: Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let item = tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("timed out waiting for a frame")
            .expect("stream ended unexpectedly")
            .expect("websocket error");
        if let WsMessage::Text(t) = item {
            return serde_json::from_str(&t).expect("frame was not valid JSON");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_pair_presence_routing_and_revocation() {
    let (http, ws) = spawn_server().await;
    let client = reqwest::Client::new();

    // 1. Register an account.
    let reg: Value = client
        .post(format!("{http}/v1/auth/register"))
        .json(&json!({"email": "user@example.com", "password": "hunter2222"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let access_token = reg["accessToken"].as_str().unwrap().to_string();
    assert!(!reg["userId"].as_str().unwrap().is_empty());

    // A second registration with the same email is rejected.
    let dup = client
        .post(format!("{http}/v1/auth/register"))
        .json(&json!({"email": "USER@example.com", "password": "hunter2222"}))
        .send()
        .await
        .unwrap();
    assert_eq!(dup.status(), 409);

    // Login works with the same credentials.
    let login: Value = client
        .post(format!("{http}/v1/auth/login"))
        .json(&json!({"email": "user@example.com", "password": "hunter2222"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(login["userId"], reg["userId"]);
    // Wrong password is rejected.
    let bad = client
        .post(format!("{http}/v1/auth/login"))
        .json(&json!({"email": "user@example.com", "password": "wrong-password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 401);

    // 2. Register the desktop device.
    let dev: Value = client
        .post(format!("{http}/v1/devices/register"))
        .bearer_auth(&access_token)
        .json(&json!({"name": "Study PC", "kind": "desktop", "platform": "windows"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let desktop_token = dev["deviceToken"].as_str().unwrap().to_string();

    // 3. Desktop mints a pairing code; the phone redeems it with no password.
    let pairing: Value = client
        .post(format!("{http}/v1/pairing/create"))
        .bearer_auth(&access_token)
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let code = pairing["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 6);

    let redeemed: Value = client
        .post(format!("{http}/v1/pairing/redeem"))
        .json(&json!({"code": code, "name": "Pixel", "platform": "android"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mobile_token = redeemed["deviceToken"].as_str().unwrap().to_string();
    let mobile_id = redeemed["deviceId"].as_str().unwrap().to_string();
    assert_eq!(redeemed["userId"], reg["userId"]);

    // The same code cannot be redeemed twice.
    let second = client
        .post(format!("{http}/v1/pairing/redeem"))
        .json(&json!({"code": code, "name": "Another phone", "platform": "android"}))
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), 400);

    // 4. Connect the desktop first, then the mobile device.
    let (mut desktop_ws, _) =
        tokio_tungstenite::connect_async(format!("{ws}/v1/ws?token={desktop_token}"))
            .await
            .unwrap();
    let (mut mobile_ws, _) =
        tokio_tungstenite::connect_async(format!("{ws}/v1/ws?token={mobile_token}"))
            .await
            .unwrap();

    // The phone learns immediately that the desktop is online.
    let presence = next_text(&mut mobile_ws).await;
    assert_eq!(presence["type"], "presence");
    assert_eq!(presence["payload"]["online"], true);

    // 5. A mobile request is routed to the desktop, and its reply back to
    //    the mobile device, correlated by id.
    mobile_ws
        .send(WsMessage::Text(
            json!({"v": 1, "id": "req-1", "type": "list_applications", "payload": {}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let received = next_text(&mut desktop_ws).await;
    assert_eq!(received["id"], "req-1");
    assert_eq!(received["type"], "list_applications");

    desktop_ws
        .send(WsMessage::Text(
            json!({"v": 1, "id": "req-1", "type": "response", "payload": {"ok": true, "data": []}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let reply = next_text(&mut mobile_ws).await;
    assert_eq!(reply["id"], "req-1");
    assert_eq!(reply["payload"]["ok"], true);

    // 6. The device list (as the desktop's Settings page would see it)
    //    shows both devices online.
    let devices: Value = client
        .get(format!("{http}/v1/devices"))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let list = devices["devices"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().all(|d| d["online"] == true));

    // 7. Revoking the mobile device closes its live socket immediately...
    let resp = client
        .delete(format!("{http}/v1/devices/{mobile_id}"))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let closed = tokio::time::timeout(Duration::from_secs(2), mobile_ws.next())
        .await
        .unwrap();
    assert!(
        matches!(closed, Some(Ok(WsMessage::Close(_))) | None),
        "revoked device's socket should close, got {closed:?}"
    );

    // ...and a brand new connection with the revoked (now unknown) token is refused.
    let reconnect =
        tokio_tungstenite::connect_async(format!("{ws}/v1/ws?token={mobile_token}")).await;
    assert!(
        reconnect.is_err(),
        "a revoked device token must not be able to reconnect"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mobile_request_without_a_connected_desktop_gets_a_synthetic_offline_reply() {
    let (http, ws) = spawn_server().await;
    let client = reqwest::Client::new();
    let reg: Value = client
        .post(format!("{http}/v1/auth/register"))
        .json(&json!({"email": "solo@example.com", "password": "hunter2222"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let access_token = reg["accessToken"].as_str().unwrap().to_string();
    let dev: Value = client
        .post(format!("{http}/v1/devices/register"))
        .bearer_auth(&access_token)
        .json(&json!({"name": "Phone", "kind": "mobile", "platform": "ios"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mobile_token = dev["deviceToken"].as_str().unwrap().to_string();

    let (mut mobile_ws, _) =
        tokio_tungstenite::connect_async(format!("{ws}/v1/ws?token={mobile_token}"))
            .await
            .unwrap();
    let presence = next_text(&mut mobile_ws).await;
    assert_eq!(presence["payload"]["online"], false);

    mobile_ws
        .send(WsMessage::Text(
            json!({"v": 1, "id": "req-9", "type": "list_applications", "payload": {}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let reply = next_text(&mut mobile_ws).await;
    assert_eq!(reply["id"], "req-9");
    assert_eq!(reply["payload"]["ok"], false);
    assert_eq!(reply["payload"]["error"]["code"], "DESKTOP_OFFLINE");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_token_rotates_and_old_token_stops_working() {
    let (http, _ws) = spawn_server().await;
    let client = reqwest::Client::new();
    let reg: Value = client
        .post(format!("{http}/v1/auth/register"))
        .json(&json!({"email": "rot@example.com", "password": "hunter2222"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let refresh_token = reg["refreshToken"].as_str().unwrap().to_string();

    let refreshed: Value = client
        .post(format!("{http}/v1/auth/refresh"))
        .json(&json!({"refreshToken": refresh_token}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!refreshed["accessToken"].as_str().unwrap().is_empty());
    let new_refresh = refreshed["refreshToken"].as_str().unwrap().to_string();
    assert_ne!(new_refresh, refresh_token);

    let reuse = client
        .post(format!("{http}/v1/auth/refresh"))
        .json(&json!({"refreshToken": refresh_token}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        reuse.status(),
        401,
        "a rotated-away refresh token must not work again"
    );
}
