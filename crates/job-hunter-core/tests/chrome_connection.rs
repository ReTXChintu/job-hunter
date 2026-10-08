//! A browser task that finds Chrome's Claude extension not connected checks
//! the connection, runs once more, and stops with a clear error if it is
//! still unreachable. Tasks that don't use the browser are unaffected.

use job_hunter_core::agent::steps::StepCtx;
use job_hunter_core::claude::ClaudeRequest;
use job_hunter_core::error::CoreError;
use job_hunter_core::AppContext;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn an_unreachable_chrome_is_reported_as_such() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    let step = StepCtx::new(ctx.clone(), "chrome-test", CancellationToken::new());

    let mut req = ClaudeRequest::new("chrome_check", String::new(), dir.path().into());
    req.chrome = true;
    req.mock_context = json!({ "mockChromeDisconnected": true });
    let err = step.run_claude(req.clone()).await.unwrap_err();
    assert!(
        matches!(err, CoreError::ChromeNotConnected { .. }),
        "{err:?}"
    );
    assert!(err.user_message().contains("Claude icon"));

    req.chrome = false;
    assert!(step.run_claude(req).await.is_ok());
}
