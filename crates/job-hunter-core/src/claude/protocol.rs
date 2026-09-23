//! Request/response types for a single Claude CLI invocation, plus the parser
//! for the `stream-json` protocol emitted by `claude -p --output-format
//! stream-json --verbose`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One non-interactive Claude Code invocation.
#[derive(Debug, Clone)]
pub struct ClaudeRequest {
    /// Short machine label used for log file names and by the mock runner
    /// (e.g. `discover`, `analyze`, `generate_resume`).
    pub label: String,
    /// Prompt sent on stdin.
    pub prompt: String,
    /// File appended to the system prompt (`--append-system-prompt-file`).
    pub system_prompt_file: Option<PathBuf>,
    /// JSON schema for structured output (`--json-schema`).
    pub json_schema: Option<Value>,
    /// Enable Claude in Chrome (`--chrome`).
    pub chrome: bool,
    /// Exact tool names pre-approved for the run (`--allowedTools`).
    pub allowed_tools: Vec<String>,
    /// Disable every built-in tool (`--tools ""`). MCP tools are unaffected.
    pub disable_builtin_tools: bool,
    pub max_turns: u32,
    pub model: Option<String>,
    pub max_budget_usd: f64,
    /// Resume a previous session (`--resume <id>`).
    pub resume_session: Option<String>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    /// Free-form data for the mock runner; ignored by the real runner.
    pub mock_context: Value,
}

impl ClaudeRequest {
    pub fn new(label: &str, prompt: String, cwd: PathBuf) -> Self {
        Self {
            label: label.into(),
            prompt,
            system_prompt_file: None,
            json_schema: None,
            chrome: false,
            allowed_tools: vec![],
            disable_builtin_tools: true,
            max_turns: 12,
            model: None,
            max_budget_usd: 0.0,
            resume_session: None,
            cwd,
            timeout: Duration::from_secs(600),
            mock_context: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeResponse {
    pub session_id: Option<String>,
    pub subtype: String,
    pub is_error: bool,
    pub text: String,
    pub structured: Option<Value>,
    pub cost_usd: f64,
    pub num_turns: u32,
    pub permission_denials: Vec<String>,
    pub raw_log_path: Option<String>,
    pub duration_ms: u64,
}

/// Live events surfaced while Claude works, shown in the Agent Activity feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaudeEvent {
    Init { session_id: String, tools: usize },
    Text { text: String },
    ToolUse { name: String, summary: String },
    ToolResult { ok: bool, summary: String },
    Result { subtype: String, is_error: bool },
    Stderr { text: String },
}

pub type EventSink = Arc<dyn Fn(ClaudeEvent) + Send + Sync>;

pub fn noop_sink() -> EventSink {
    Arc::new(|_| {})
}

/// Parse a single stream-json line into zero or more events; also captures
/// the final result.
pub struct StreamParser {
    pub session_id: Option<String>,
    pub result: Option<Value>,
    pub last_text: String,
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            session_id: None,
            result: None,
            last_text: String::new(),
        }
    }

    pub fn parse_line(&mut self, line: &str) -> Vec<ClaudeEvent> {
        let line = line.trim();
        if line.is_empty() {
            return vec![];
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return vec![];
        };
        let mut events = Vec::new();
        match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "system" => {
                if v.get("subtype").and_then(|s| s.as_str()) == Some("init") {
                    let sid = v
                        .get("session_id")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let tools = v
                        .get("tools")
                        .and_then(|t| t.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    self.session_id = Some(sid.clone());
                    events.push(ClaudeEvent::Init {
                        session_id: sid,
                        tools,
                    });
                }
            }
            "assistant" => {
                if let Some(content) = v.pointer("/message/content").and_then(|c| c.as_array()) {
                    for block in content {
                        match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                            "text" => {
                                let text = block
                                    .get("text")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .trim()
                                    .to_string();
                                if !text.is_empty() {
                                    self.last_text = text.clone();
                                    events.push(ClaudeEvent::Text { text });
                                }
                            }
                            "tool_use" => {
                                let name = block
                                    .get("name")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("tool")
                                    .to_string();
                                let input = block.get("input").cloned().unwrap_or(Value::Null);
                                events.push(ClaudeEvent::ToolUse {
                                    summary: describe_tool_use(&name, &input),
                                    name,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            "user" => {
                if let Some(content) = v.pointer("/message/content").and_then(|c| c.as_array()) {
                    for block in content {
                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                            let is_error = block
                                .get("is_error")
                                .and_then(|b| b.as_bool())
                                .unwrap_or(false);
                            let text = tool_result_text(block);
                            events.push(ClaudeEvent::ToolResult {
                                ok: !is_error,
                                summary: crate::util::truncate(&text, 160),
                            });
                        }
                    }
                }
            }
            "result" => {
                let subtype = v
                    .get("subtype")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_error = v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false);
                if let Some(sid) = v.get("session_id").and_then(|s| s.as_str()) {
                    self.session_id = Some(sid.to_string());
                }
                self.result = Some(v.clone());
                events.push(ClaudeEvent::Result { subtype, is_error });
            }
            _ => {}
        }
        events
    }

    pub fn into_response(self) -> ClaudeResponse {
        let Some(r) = self.result else {
            return ClaudeResponse {
                session_id: self.session_id,
                subtype: "no_result".into(),
                is_error: true,
                text: self.last_text,
                ..Default::default()
            };
        };
        let denials = r
            .get("permission_denials")
            .and_then(|d| d.as_array())
            .map(|a| {
                a.iter()
                    .map(|d| {
                        d.get("tool_name")
                            .and_then(|t| t.as_str())
                            .unwrap_or("?")
                            .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let text = r
            .get("result")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        let structured = r
            .get("structured_output")
            .cloned()
            .filter(|s| !s.is_null())
            .or_else(|| {
                // Recover JSON embedded in text when the schema tool was not used.
                crate::util::extract_json(&text).and_then(|j| serde_json::from_str(&j).ok())
            });
        ClaudeResponse {
            session_id: r
                .get("session_id")
                .and_then(|s| s.as_str())
                .map(String::from)
                .or(self.session_id),
            subtype: r
                .get("subtype")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            is_error: r.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false),
            text,
            structured,
            cost_usd: r
                .get("total_cost_usd")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0),
            num_turns: r.get("num_turns").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
            permission_denials: denials,
            raw_log_path: None,
            duration_ms: r.get("duration_ms").and_then(|n| n.as_u64()).unwrap_or(0),
        }
    }
}

fn tool_result_text(block: &Value) -> String {
    match block.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("text").and_then(|t| t.as_str()))
            .filter(|t| !t.starts_with("<system-reminder>"))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Human-readable description of a tool call for the activity feed.
pub fn describe_tool_use(name: &str, input: &Value) -> String {
    let short = name.strip_prefix("mcp__claude-in-chrome__").unwrap_or(name);
    let s = |k: &str| {
        input
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    match short {
        "navigate" => format!("Opening {}", crate::util::truncate(&s("url"), 100)),
        "tabs_create_mcp" => "Opening a new browser tab".into(),
        "tabs_context_mcp" => "Checking browser tabs".into(),
        "get_page_text" | "read_page" => "Reading the page".into(),
        "find" => format!("Looking for: {}", crate::util::truncate(&s("query"), 80)),
        "form_input" => "Filling a form field".into(),
        "file_upload" => "Uploading a file".into(),
        "computer" => match s("action").as_str() {
            "screenshot" => "Taking a screenshot".into(),
            "left_click" | "click" => "Clicking".into(),
            "type" => "Typing".into(),
            "scroll" => "Scrolling".into(),
            "key" => "Pressing a key".into(),
            other if !other.is_empty() => format!("Browser action: {other}"),
            _ => "Interacting with the page".into(),
        },
        "browser_batch" => {
            let n = input
                .get("actions")
                .and_then(|a| a.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            format!("Performing {n} browser actions")
        }
        "javascript_tool" => "Inspecting the page".into(),
        "StructuredOutput" => "Preparing structured result".into(),
        other => format!("Using {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_init_tool_use_and_result() {
        let mut p = StreamParser::new();
        let init = r#"{"type":"system","subtype":"init","session_id":"abc","tools":["a","b"]}"#;
        let ev = p.parse_line(init);
        assert!(
            matches!(ev[0], ClaudeEvent::Init { ref session_id, tools: 2 } if session_id == "abc")
        );
        let tool = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__claude-in-chrome__navigate","input":{"url":"https://x.test"}}]}}"#;
        let ev = p.parse_line(tool);
        assert!(
            matches!(&ev[0], ClaudeEvent::ToolUse { summary, .. } if summary == "Opening https://x.test")
        );
        let result = r#"{"type":"result","subtype":"success","is_error":false,"result":"done","session_id":"abc","total_cost_usd":0.5,"num_turns":3,"structured_output":{"ok":true},"permission_denials":[{"tool_name":"Bash"}]}"#;
        p.parse_line(result);
        let resp = p.into_response();
        assert_eq!(resp.session_id.as_deref(), Some("abc"));
        assert_eq!(resp.structured.unwrap()["ok"], Value::Bool(true));
        assert_eq!(resp.permission_denials, vec!["Bash"]);
        assert!((resp.cost_usd - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn recovers_json_from_text_when_no_structured_output() {
        let mut p = StreamParser::new();
        p.parse_line(r#"{"type":"result","subtype":"success","is_error":false,"result":"```json\n{\"jobs\":[]}\n```"}"#);
        let resp = p.into_response();
        assert_eq!(resp.structured.unwrap()["jobs"], serde_json::json!([]));
    }

    #[test]
    fn missing_result_is_an_error() {
        let p = StreamParser::new();
        let resp = p.into_response();
        assert!(resp.is_error);
        assert_eq!(resp.subtype, "no_result");
    }
}
