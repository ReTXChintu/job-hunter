//! In-memory connection registry: who is online, and routing of frames
//! between the one desktop connection and any mobile connections for the
//! same account. This is pure plumbing — the hub never looks inside a
//! frame's payload, only at which device sent it and which device(s) it
//! should go to.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::Message;
use chrono::Utc;
use serde_json::json;
use tokio::sync::{mpsc, RwLock};

use crate::models::DeviceKind;

type Sender = mpsc::UnboundedSender<Message>;

struct Connection {
    device_id: String,
    sender: Sender,
}

#[derive(Default)]
struct UserChannels {
    desktop: Option<Connection>,
    mobiles: Vec<Connection>,
}

#[derive(Default)]
pub struct Hub {
    users: RwLock<HashMap<String, UserChannels>>,
}

pub enum RegisterOutcome {
    Registered { desktop_online: bool },
}

impl Hub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Registers a live connection for `device_id` (kind `kind`) under
    /// `user_id`. A second desktop connection for the same account replaces
    /// (and closes) the previous one — only one desktop is meant to be
    /// online per account at a time. Returns whether a desktop is online
    /// for this account right after registering, so the caller can send the
    /// mobile client an immediate presence snapshot.
    pub async fn register(
        &self,
        user_id: &str,
        device_id: &str,
        kind: DeviceKind,
        sender: Sender,
    ) -> RegisterOutcome {
        let mut users = self.users.write().await;
        let channels = users.entry(user_id.to_string()).or_default();
        match kind {
            DeviceKind::Desktop => {
                if let Some(old) = channels.desktop.replace(Connection {
                    device_id: device_id.to_string(),
                    sender,
                }) {
                    let _ = old.sender.send(close_message());
                }
                broadcast_presence(&channels.mobiles, true);
            }
            DeviceKind::Mobile => {
                channels.mobiles.retain(|c| c.device_id != device_id);
                channels.mobiles.push(Connection {
                    device_id: device_id.to_string(),
                    sender,
                });
            }
        }
        RegisterOutcome::Registered {
            desktop_online: channels.desktop.is_some(),
        }
    }

    /// Removes a connection. If it was the desktop's, every connected
    /// mobile is told it just went offline. No-ops if this connection was
    /// already replaced (e.g. a fast reconnect).
    pub async fn unregister(&self, user_id: &str, device_id: &str, kind: DeviceKind) {
        let mut users = self.users.write().await;
        let Some(channels) = users.get_mut(user_id) else {
            return;
        };
        match kind {
            DeviceKind::Desktop => {
                let still_current = channels
                    .desktop
                    .as_ref()
                    .is_some_and(|c| c.device_id == device_id);
                if still_current {
                    channels.desktop = None;
                    broadcast_presence(&channels.mobiles, false);
                }
            }
            DeviceKind::Mobile => {
                channels.mobiles.retain(|c| c.device_id != device_id);
            }
        }
        if channels.desktop.is_none() && channels.mobiles.is_empty() {
            users.remove(user_id);
        }
    }

    pub async fn is_desktop_online(&self, user_id: &str) -> bool {
        self.users
            .read()
            .await
            .get(user_id)
            .is_some_and(|c| c.desktop.is_some())
    }

    /// Whether the specific device (desktop or mobile) currently has a live
    /// connection. Used to annotate the device list shown in Settings.
    pub async fn is_online(&self, user_id: &str, device_id: &str) -> bool {
        let users = self.users.read().await;
        let Some(channels) = users.get(user_id) else {
            return false;
        };
        channels
            .desktop
            .as_ref()
            .is_some_and(|d| d.device_id == device_id)
            || channels.mobiles.iter().any(|m| m.device_id == device_id)
    }

    /// Forwards a frame from a mobile device to the connected desktop.
    /// Returns `false` if no desktop is connected (the caller should send
    /// the mobile device a synthetic `DESKTOP_OFFLINE` response).
    pub async fn route_to_desktop(&self, user_id: &str, message: Message) -> bool {
        let users = self.users.read().await;
        match users.get(user_id).and_then(|c| c.desktop.as_ref()) {
            Some(conn) => conn.sender.send(message).is_ok(),
            None => false,
        }
    }

    /// Forwards a frame from the desktop to every connected mobile device
    /// for the same account (pushes: `changed`, `agent_status`, and desktop
    /// responses to mobile-initiated requests are also sent this way since
    /// only the desktop knows which mobile device originated a given `id` —
    /// it addresses the response with the `id` it was given; the hub simply
    /// fans it out and every mobile device ignores ids it did not send).
    pub async fn route_to_mobiles(&self, user_id: &str, message: Message) {
        let users = self.users.read().await;
        if let Some(channels) = users.get(user_id) {
            for conn in &channels.mobiles {
                let _ = conn.sender.send(message.clone());
            }
        }
    }

    /// Forces a specific device's live socket closed, if connected. Used
    /// when a device is revoked so access ends immediately, not just on its
    /// next reconnect attempt.
    pub async fn force_disconnect(&self, user_id: &str, device_id: &str) {
        let users = self.users.read().await;
        let Some(channels) = users.get(user_id) else {
            return;
        };
        if let Some(d) = &channels.desktop {
            if d.device_id == device_id {
                let _ = d.sender.send(close_message());
            }
        }
        for m in &channels.mobiles {
            if m.device_id == device_id {
                let _ = m.sender.send(close_message());
            }
        }
    }
}

fn broadcast_presence(mobiles: &[Connection], online: bool) {
    let payload = json!({
        "v": 1,
        "id": uuid::Uuid::new_v4().to_string(),
        "type": "presence",
        "payload": { "device": "desktop", "online": online, "lastSeenAt": Utc::now().to_rfc3339() },
    });
    let msg = Message::Text(payload.to_string().into());
    for conn in mobiles {
        let _ = conn.sender.send(msg.clone());
    }
}

fn close_message() -> Message {
    Message::Close(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;

    fn text(msg: &Message) -> String {
        match msg {
            Message::Text(t) => t.to_string(),
            _ => String::new(),
        }
    }

    #[tokio::test]
    async fn mobile_learns_presence_on_desktop_connect_and_disconnect() {
        let hub = Hub::new();
        let (mtx, mut mrx) = unbounded_channel();
        hub.register("u1", "mobile-1", DeviceKind::Mobile, mtx)
            .await;
        assert!(!hub.is_desktop_online("u1").await);

        let (dtx, _drx) = unbounded_channel();
        let outcome = hub
            .register("u1", "desktop-1", DeviceKind::Desktop, dtx)
            .await;
        assert!(matches!(
            outcome,
            RegisterOutcome::Registered {
                desktop_online: true
            }
        ));
        let presence = mrx.recv().await.unwrap();
        assert!(text(&presence).contains("\"online\":true"));
        assert!(hub.is_desktop_online("u1").await);

        hub.unregister("u1", "desktop-1", DeviceKind::Desktop).await;
        let presence = mrx.recv().await.unwrap();
        assert!(text(&presence).contains("\"online\":false"));
        assert!(!hub.is_desktop_online("u1").await);
    }

    #[tokio::test]
    async fn requests_route_to_desktop_and_replies_fan_out_to_mobiles() {
        let hub = Hub::new();
        let (dtx, mut drx) = unbounded_channel();
        hub.register("u1", "desktop-1", DeviceKind::Desktop, dtx)
            .await;
        let (mtx, mut mrx) = unbounded_channel();
        hub.register("u1", "mobile-1", DeviceKind::Mobile, mtx)
            .await;

        let ok = hub
            .route_to_desktop("u1", Message::Text("hello".into()))
            .await;
        assert!(ok);
        assert_eq!(text(&drx.recv().await.unwrap()), "hello");

        hub.route_to_mobiles("u1", Message::Text("reply".into()))
            .await;
        assert_eq!(text(&mrx.recv().await.unwrap()), "reply");
    }

    #[tokio::test]
    async fn routing_to_desktop_fails_cleanly_when_none_connected() {
        let hub = Hub::new();
        let ok = hub
            .route_to_desktop("no-such-user", Message::Text("hi".into()))
            .await;
        assert!(!ok);
    }

    #[tokio::test]
    async fn second_desktop_connection_replaces_and_closes_the_first() {
        let hub = Hub::new();
        let (d1tx, mut d1rx) = unbounded_channel();
        hub.register("u1", "desktop-1", DeviceKind::Desktop, d1tx)
            .await;
        let (d2tx, _d2rx) = unbounded_channel();
        hub.register("u1", "desktop-2", DeviceKind::Desktop, d2tx)
            .await;
        assert!(matches!(d1rx.recv().await.unwrap(), Message::Close(None)));
    }

    #[tokio::test]
    async fn force_disconnect_closes_only_the_named_device() {
        let hub = Hub::new();
        let (mtx, mut mrx) = unbounded_channel();
        hub.register("u1", "mobile-1", DeviceKind::Mobile, mtx)
            .await;
        hub.force_disconnect("u1", "mobile-1").await;
        assert!(matches!(mrx.recv().await.unwrap(), Message::Close(None)));
    }
}
