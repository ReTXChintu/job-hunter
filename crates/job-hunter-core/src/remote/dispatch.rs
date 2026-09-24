//! Turns one inbound request envelope from the mobile app into exactly the
//! same `AppContext` / `orchestrator` call the Tauri commands make, so a
//! phone can never do anything the desktop UI couldn't. Kept as a pure
//! function of `(kind, payload) -> Value` so it can be unit-tested directly
//! against the mock Claude runner, with no network involved.

use std::sync::Arc;

use base64::Engine;
use serde_json::{json, Value};

use crate::agent::orchestrator;
use crate::context::AppContext;
use crate::domain::{AnswerSource, ApplicationAnswer, ApplicationStatus};
use crate::error::{CoreError, CoreResult};

use super::protocol::{
    AnswerQuestionsPayload, MarkManualCompletePayload, RejectPayload, RequestFilePayload,
    SetStatusPayload, WithId,
};

/// Defensive cap on any single string field sent to the phone (a job
/// description, a note, ...): the mobile app is a review surface, not a
/// place to mirror arbitrarily large text.
const MAX_STRING_LEN: usize = 20_000;
/// Small resumes/cover letters only; anything bigger is opened on the
/// desktop instead of round-tripped over the relay as base64.
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

pub async fn dispatch(app: &Arc<AppContext>, kind: &str, payload: Value) -> CoreResult<Value> {
    match kind {
        "list_dashboard" => {
            let dashboard = app.dashboard().await?;
            let mut v = serde_json::to_value(dashboard)?;
            if let Some(agent) = v.get_mut("agent").and_then(|a| a.as_object_mut()) {
                agent.remove("currentActivity");
            }
            cap_strings(&mut v, MAX_STRING_LEN);
            Ok(v)
        }
        "list_applications" => {
            let list = app.list_applications()?;
            let mut v = serde_json::to_value(list)?;
            cap_strings(&mut v, MAX_STRING_LEN);
            Ok(v)
        }
        "get_application" => {
            let WithId { id } = parse(payload)?;
            let detail = app.application_detail(&id)?;
            let mut v = serde_json::to_value(detail)?;
            cap_strings(&mut v, MAX_STRING_LEN);
            Ok(v)
        }
        "approve_application" => {
            let WithId { id } = parse(payload)?;
            let application = orchestrator::approve_application(app.clone(), &id, true).await?;
            Ok(serde_json::to_value(application)?)
        }
        "reject_application" => {
            let p: RejectPayload = parse(payload)?;
            let application =
                orchestrator::reject_application(app.clone(), &p.id, &p.reason).await?;
            Ok(serde_json::to_value(application)?)
        }
        "apply_application" => {
            let WithId { id } = parse(payload)?;
            let run = orchestrator::apply_application(app.clone(), &id, false, None).await?;
            Ok(json!({ "runId": run.id }))
        }
        "answer_application_questions" => {
            let p: AnswerQuestionsPayload = parse(payload)?;
            let answers = p
                .answers
                .into_iter()
                .map(|a| ApplicationAnswer {
                    question: a.question,
                    answer: a.answer,
                    source: AnswerSource::User,
                })
                .collect();
            let application =
                orchestrator::answer_questions(app.clone(), &p.id, answers, true).await?;
            Ok(serde_json::to_value(application)?)
        }
        "mark_manual_application_complete" => {
            let p: MarkManualCompletePayload = parse(payload)?;
            let application =
                orchestrator::mark_manual_complete(app.clone(), &p.id, &p.note).await?;
            Ok(serde_json::to_value(application)?)
        }
        "set_application_status" => {
            let p: SetStatusPayload = parse(payload)?;
            let status = parse_status(&p.status)?;
            let application =
                orchestrator::set_application_status(app.clone(), &p.id, status, &p.note).await?;
            Ok(serde_json::to_value(application)?)
        }
        "request_file" => {
            let p: RequestFilePayload = parse(payload)?;
            request_file(app, &p.path).await
        }
        other => Err(CoreError::Validation(format!(
            "unknown request type '{other}'"
        ))),
    }
}

fn parse<T: serde::de::DeserializeOwned>(payload: Value) -> CoreResult<T> {
    serde_json::from_value(payload)
        .map_err(|e| CoreError::Validation(format!("invalid request: {e}")))
}

fn parse_status(s: &str) -> CoreResult<ApplicationStatus> {
    serde_json::from_value(Value::String(s.to_string()))
        .map_err(|_| CoreError::Validation(format!("invalid status '{s}'")))
}

async fn request_file(app: &Arc<AppContext>, path: &str) -> CoreResult<Value> {
    let p = std::path::PathBuf::from(path);
    if !crate::util::path_is_inside(&app.paths.root, &p) {
        return Err(CoreError::Validation(
            "that file is outside the Job Hunter data directory".into(),
        ));
    }
    let meta = tokio::fs::metadata(&p)
        .await
        .map_err(|_| CoreError::NotFound {
            entity: "file",
            id: path.to_string(),
        })?;
    if !meta.is_file() {
        return Err(CoreError::NotFound {
            entity: "file",
            id: path.to_string(),
        });
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(CoreError::Validation(
            "that file is too large to send to your phone; open it on the desktop instead".into(),
        ));
    }
    let bytes = tokio::fs::read(&p)
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?;
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    Ok(json!({ "name": name, "base64": base64::engine::general_purpose::STANDARD.encode(bytes) }))
}

fn cap_strings(value: &mut Value, max_len: usize) {
    match value {
        Value::String(s) => {
            if s.chars().count() > max_len {
                *s = s.chars().take(max_len).collect();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| cap_strings(v, max_len)),
        Value::Object(map) => map.values_mut().for_each(|v| cap_strings(v, max_len)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ApplicationStatus as S, JobStatus, LOCAL_USER_ID};

    async fn ctx() -> Arc<AppContext> {
        let dir = tempfile::tempdir().unwrap();
        // Keep the tempdir alive for the process; fine for a short test.
        let path = dir.keep();
        AppContext::init_mock(path).await.unwrap()
    }

    #[tokio::test]
    async fn unknown_request_type_is_a_validation_error_not_a_panic() {
        let app = ctx().await;
        let err = dispatch(&app, "delete_everything", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[tokio::test]
    async fn approve_from_mobile_goes_through_the_same_approval_gate() {
        let app = ctx().await;
        // No applications exist yet; approving a made-up id must fail like
        // it would from the desktop UI, not silently succeed.
        let err = dispatch(&app, "approve_application", json!({"id": "does-not-exist"}))
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[tokio::test]
    async fn list_dashboard_omits_current_activity_and_caps_long_strings() {
        let app = ctx().await;
        let mut job =
            crate::domain::Job::new(LOCAL_USER_ID, "LinkedIn", "https://x/1", "Acme", "Engineer");
        job.description = "x".repeat(MAX_STRING_LEN + 500);
        job.status = JobStatus::ReadyForReview;
        app.store.put(&job).unwrap();

        let dashboard = dispatch(&app, "list_dashboard", json!({})).await.unwrap();
        assert!(dashboard
            .get("agent")
            .unwrap()
            .get("currentActivity")
            .is_none());

        let apps = dispatch(&app, "list_applications", json!({}))
            .await
            .unwrap();
        assert!(apps.is_array());

        let mut probe = json!({ "d": "x".repeat(MAX_STRING_LEN + 10) });
        cap_strings(&mut probe, MAX_STRING_LEN);
        assert_eq!(probe["d"].as_str().unwrap().chars().count(), MAX_STRING_LEN);
    }

    #[tokio::test]
    async fn set_application_status_rejects_an_unknown_status_string() {
        let app = ctx().await;
        let err = dispatch(
            &app,
            "set_application_status",
            json!({"id": "x", "status": "NOT_A_REAL_STATUS"}),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
        // Sanity: a real status parses fine (still fails at NotFound, which
        // proves parsing succeeded and it went on to look the app up).
        let err = dispatch(
            &app,
            "set_application_status",
            json!({"id": "x", "status": S::Interview}),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[tokio::test]
    async fn request_file_refuses_paths_outside_the_data_directory() {
        let app = ctx().await;
        let outside = std::env::temp_dir().join("job-hunter-outside-test.txt");
        std::fs::write(&outside, "nope").unwrap();
        let err = dispatch(
            &app,
            "request_file",
            json!({"path": outside.to_string_lossy(), "kind": "pdf"}),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));

        let inside = app.paths.resumes_dir().join("probe.txt");
        std::fs::create_dir_all(app.paths.resumes_dir()).unwrap();
        std::fs::write(&inside, "hello").unwrap();
        let ok = dispatch(
            &app,
            "request_file",
            json!({"path": inside.to_string_lossy(), "kind": "pdf"}),
        )
        .await
        .unwrap();
        assert_eq!(ok["name"], "probe.txt");
        assert!(!ok["base64"].as_str().unwrap().is_empty());
    }
}
