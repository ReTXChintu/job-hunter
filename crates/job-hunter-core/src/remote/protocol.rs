//! The WebSocket envelope and request/response payload shapes spoken with
//! the relay. This is the Rust half of `docs/mobile-protocol.md`; the Dart
//! half lives in `apps/mobile/lib/models/envelope.dart`. Keep them in sync
//! by hand, the same way `packages/types` mirrors the Rust domain model.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::UserFacingError;
use crate::util::new_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub id: Value,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub payload: Value,
}

impl Envelope {
    pub fn response_ok(id: Value, data: Value) -> Self {
        Self {
            v: 1,
            id,
            kind: "response".into(),
            payload: serde_json::json!({ "ok": true, "data": data }),
        }
    }

    pub fn response_err(id: Value, error: &UserFacingError) -> Self {
        Self {
            v: 1,
            id,
            kind: "response".into(),
            payload: serde_json::json!({ "ok": false, "error": { "code": error.code, "message": error.message } }),
        }
    }

    pub fn push(kind: &str, payload: Value) -> Self {
        Self {
            v: 1,
            id: Value::String(new_id()),
            kind: kind.into(),
            payload,
        }
    }
}

/// Request payload shapes, parsed out of `Envelope::payload` by request
/// `type`. Every variant carries only what that request needs.
#[derive(Debug, Clone, Deserialize)]
pub struct WithId {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectPayload {
    pub id: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnswerPayload {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerQuestionsPayload {
    pub id: String,
    pub answers: Vec<AnswerPayload>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkManualCompletePayload {
    pub id: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStatusPayload {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestFilePayload {
    pub path: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_and_echoes_the_request_id() {
        let raw = r#"{"v":1,"id":"req-1","type":"list_applications","payload":{}}"#;
        let env: Envelope = serde_json::from_str(raw).unwrap();
        assert_eq!(env.kind, "list_applications");
        let resp = Envelope::response_ok(env.id.clone(), serde_json::json!([1, 2]));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"id\":\"req-1\""));
        assert!(s.contains("\"ok\":true"));
    }

    #[test]
    fn error_response_uses_the_same_shape_as_tauri_errors() {
        let error = UserFacingError {
            code: "AGENT_BUSY".into(),
            message: "busy".into(),
            details: None,
            recoverable: true,
        };
        let resp = Envelope::response_err(Value::String("x".into()), &error);
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"code\":\"AGENT_BUSY\""));
        assert!(s.contains("\"ok\":false"));
    }
}
