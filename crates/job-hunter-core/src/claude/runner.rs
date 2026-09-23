//! Spawns `claude -p` and streams its output.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

use super::discover::ClaudeCli;
use super::protocol::{ClaudeEvent, ClaudeRequest, ClaudeResponse, EventSink, StreamParser};
use crate::error::{CoreError, CoreResult};
use crate::secrets::redact_secrets;

#[async_trait]
pub trait ClaudeRunner: Send + Sync {
    async fn run(
        &self,
        request: ClaudeRequest,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> CoreResult<ClaudeResponse>;
    fn is_mock(&self) -> bool {
        false
    }
}

pub struct CliClaudeRunner {
    cli: ClaudeCli,
}

impl CliClaudeRunner {
    pub fn new(cli: ClaudeCli) -> Self {
        Self { cli }
    }

    pub fn cli(&self) -> &ClaudeCli {
        &self.cli
    }

    fn build_args(req: &ClaudeRequest) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "-p".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--permission-mode".into(),
            "dontAsk".into(),
            "--strict-mcp-config".into(),
            "--max-turns".into(),
            req.max_turns.to_string(),
        ];
        if req.chrome {
            args.push("--chrome".into());
        } else {
            args.push("--no-chrome".into());
        }
        if req.disable_builtin_tools {
            args.push("--tools".into());
            args.push(String::new());
        }
        if !req.allowed_tools.is_empty() {
            args.push("--allowedTools".into());
            args.push(req.allowed_tools.join(","));
        }
        if let Some(model) = req.model.as_deref().filter(|m| !m.trim().is_empty()) {
            args.push("--model".into());
            args.push(model.trim().to_string());
        }
        if req.max_budget_usd > 0.0 {
            args.push("--max-budget-usd".into());
            args.push(format!("{:.2}", req.max_budget_usd));
        }
        if let Some(schema) = &req.json_schema {
            args.push("--json-schema".into());
            args.push(schema.to_string());
        }
        if let Some(file) = &req.system_prompt_file {
            args.push("--append-system-prompt-file".into());
            args.push(file.to_string_lossy().to_string());
        }
        if let Some(sid) = &req.resume_session {
            args.push("--resume".into());
            args.push(sid.clone());
        }
        args
    }
}

#[async_trait]
impl ClaudeRunner for CliClaudeRunner {
    async fn run(
        &self,
        req: ClaudeRequest,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> CoreResult<ClaudeResponse> {
        std::fs::create_dir_all(&req.cwd)?;
        let args = Self::build_args(&req);
        let log_path: PathBuf = req.cwd.join(format!(
            "{}-{}.ndjson",
            req.label,
            chrono::Utc::now().format("%Y%m%dT%H%M%S")
        ));
        let prompt_path = req.cwd.join(format!("{}-prompt.md", req.label));
        std::fs::write(&prompt_path, &req.prompt)?;

        tracing::info!(label = %req.label, chrome = req.chrome, max_turns = req.max_turns, "starting claude");
        tracing::debug!(args = ?args.iter().map(|a| crate::util::truncate(a, 120)).collect::<Vec<_>>(), "claude arguments");

        let mut cmd = self.cli.command();
        cmd.args(&args)
            .current_dir(&req.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| CoreError::ClaudeCliUnavailable {
            details: format!("failed to start claude: {e}"),
        })?;

        // Feed the prompt and close stdin.
        if let Some(mut stdin) = child.stdin.take() {
            let prompt = req.prompt.clone();
            tokio::spawn(async move {
                let _ = stdin.write_all(prompt.as_bytes()).await;
                let _ = stdin.shutdown().await;
            });
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::other("no stdout from claude"))?;
        let stderr = child.stderr.take();

        let stderr_sink = sink.clone();
        let stderr_task = tokio::spawn(async move {
            let mut collected = String::new();
            if let Some(stderr) = stderr {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let line = redact_secrets(&line);
                    if collected.len() < 8000 {
                        collected.push_str(&line);
                        collected.push('\n');
                    }
                    if !line.trim().is_empty() {
                        stderr_sink(ClaudeEvent::Stderr { text: line });
                    }
                }
            }
            collected
        });

        let mut parser = StreamParser::new();
        let mut log_file = tokio::fs::File::create(&log_path).await.ok();
        let mut lines = BufReader::new(stdout).lines();
        let started = std::time::Instant::now();
        let mut timed_out = false;
        let mut cancelled = false;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => { cancelled = true; break; }
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(std::time::Instant::now() + req.timeout.saturating_sub(started.elapsed()))) => { timed_out = true; break; }
                line = lines.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(f) = log_file.as_mut() {
                                let _ = f.write_all(redact_secrets(&line).as_bytes()).await;
                                let _ = f.write_all(b"\n").await;
                            }
                            for ev in parser.parse_line(&line) {
                                sink(ev);
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            tracing::warn!(error = %e, "error reading claude output");
                            break;
                        }
                    }
                }
            }
        }

        if cancelled || timed_out {
            let _ = child.kill().await;
            let _ = child.wait().await;
            stderr_task.abort();
            if cancelled {
                tracing::info!(label = %req.label, "claude run cancelled");
                return Err(CoreError::Cancelled);
            }
            tracing::warn!(label = %req.label, "claude run timed out");
            return Err(CoreError::ClaudeRunFailed {
                message: format!(
                    "Claude did not finish within {} seconds",
                    req.timeout.as_secs()
                ),
                details: format!("log: {}", log_path.display()),
            });
        }

        let status = child.wait().await.map_err(|e| CoreError::ClaudeRunFailed {
            message: "process error".into(),
            details: e.to_string(),
        })?;
        let stderr_text = stderr_task.await.unwrap_or_default();
        let mut response = parser.into_response();
        response.raw_log_path = Some(log_path.to_string_lossy().to_string());

        if response.subtype == "no_result" {
            let details = format!(
                "exit status: {status}\nstderr:\n{stderr_text}\nlog: {}",
                log_path.display()
            );
            let lower = stderr_text.to_lowercase();
            if lower.contains("not logged in")
                || lower.contains("please run /login")
                || lower.contains("authentication")
            {
                return Err(CoreError::ClaudeNotAuthenticated { details });
            }
            return Err(CoreError::ClaudeRunFailed {
                message: "Claude produced no result".into(),
                details,
            });
        }
        if response.is_error {
            let message = if response.text.is_empty() {
                format!("Claude ended with {}", response.subtype)
            } else {
                crate::util::truncate(&response.text, 300)
            };
            if response.subtype.contains("max_turns") {
                tracing::warn!(label = %req.label, "claude hit the turn limit");
            } else if response.subtype.contains("budget") {
                tracing::warn!(label = %req.label, "claude hit the budget limit");
            }
            return Err(CoreError::ClaudeRunFailed {
                message,
                details: format!(
                    "subtype: {}\nstderr:\n{}\nlog: {}",
                    response.subtype,
                    stderr_text,
                    log_path.display()
                ),
            });
        }
        tracing::info!(label = %req.label, cost_usd = response.cost_usd, turns = response.num_turns, "claude finished");
        Ok(response)
    }
}

/// Shared handle type used throughout the agent.
pub type SharedRunner = Arc<dyn ClaudeRunner>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn args_are_locked_down() {
        let mut req = ClaudeRequest::new("analyze", "hi".into(), PathBuf::from("."));
        req.json_schema = Some(serde_json::json!({"type":"object"}));
        req.chrome = true;
        req.allowed_tools = vec!["mcp__claude-in-chrome__navigate".into()];
        req.max_budget_usd = 2.0;
        req.timeout = Duration::from_secs(1);
        let args = CliClaudeRunner::build_args(&req);
        assert!(args.contains(&"--permission-mode".to_string()));
        assert!(args.contains(&"dontAsk".to_string()));
        assert!(args.contains(&"--chrome".to_string()));
        assert!(args.contains(&"--json-schema".to_string()));
        assert!(args.contains(&"--max-budget-usd".to_string()));
        assert!(args.contains(&"mcp__claude-in-chrome__navigate".to_string()));
        assert!(!args.iter().any(|a| a.contains("dangerously")));
        assert!(!args.iter().any(|a| a == "bypassPermissions"));
    }
}
