//! Structured logging: a tracing layer that keeps a redacted in-memory ring
//! buffer (for the in-app log viewer), appends to a daily log file, and
//! broadcasts entries to subscribers (the UI).

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

use crate::secrets::redact_secrets;
use crate::util::{new_id, now};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl From<&Level> for LogLevel {
    fn from(l: &Level) -> Self {
        match *l {
            Level::ERROR => LogLevel::Error,
            Level::WARN => LogLevel::Warn,
            Level::INFO => LogLevel::Info,
            _ => LogLevel::Debug,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub at: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

pub struct LogBuffer {
    entries: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
    tx: broadcast::Sender<LogEntry>,
    file_dir: Option<PathBuf>,
}

impl LogBuffer {
    pub fn new(capacity: usize, file_dir: Option<PathBuf>) -> Arc<Self> {
        let (tx, _) = broadcast::channel(512);
        Arc::new(Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            tx,
            file_dir,
        })
    }

    pub fn push(&self, entry: LogEntry) {
        {
            let mut e = self.entries.lock().unwrap();
            if e.len() >= self.capacity {
                e.pop_front();
            }
            e.push_back(entry.clone());
        }
        if let Some(dir) = &self.file_dir {
            let _ = append_to_file(dir, &entry);
        }
        let _ = self.tx.send(entry);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.tx.subscribe()
    }

    pub fn entries(&self, min_level: LogLevel, limit: usize) -> Vec<LogEntry> {
        let e = self.entries.lock().unwrap();
        e.iter()
            .rev()
            .filter(|x| x.level >= min_level)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    pub fn export_text(&self) -> String {
        let e = self.entries.lock().unwrap();
        let mut out = String::new();
        for entry in e.iter() {
            let _ = writeln!(out, "{}", format_line(entry));
        }
        out
    }
}

fn format_line(e: &LogEntry) -> String {
    format!(
        "{} {:5} [{}] {}",
        e.at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        format!("{:?}", e.level).to_uppercase(),
        e.target,
        e.message
    )
}

fn append_to_file(dir: &Path, entry: &LogEntry) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let file = dir.join(format!("job-hunter-{}.log", entry.at.format("%Y-%m-%d")));
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?;
    writeln!(f, "{}", format_line(entry))
}

/// tracing layer feeding the buffer.
pub struct BufferLayer {
    buffer: Arc<LogBuffer>,
}

impl BufferLayer {
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        Self { buffer }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }
}

impl<S: Subscriber> Layer<S> for BufferLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let target = event.metadata().target();
        // Only keep our own crates and warnings from dependencies.
        let ours = target.starts_with("job_hunter");
        if !ours && *event.metadata().level() > Level::WARN {
            return;
        }
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let mut message = visitor.message;
        if !visitor.fields.is_empty() {
            let extra: Vec<String> = visitor
                .fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            if !message.is_empty() {
                message.push(' ');
            }
            message.push_str(&extra.join(" "));
        }
        self.buffer.push(LogEntry {
            id: new_id(),
            at: now(),
            level: LogLevel::from(event.metadata().level()),
            target: target
                .trim_start_matches("job_hunter_core::")
                .trim_start_matches("job_hunter_desktop::")
                .to_string(),
            message: redact_secrets(&message),
        });
    }
}

/// Install the global subscriber. Safe to call once per process.
pub fn init(buffer: Arc<LogBuffer>) {
    use tracing_subscriber::prelude::*;
    let filter =
        tracing_subscriber::EnvFilter::try_from_env("JOB_HUNTER_LOG").unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(
                "info,job_hunter_core=debug,job_hunter_desktop=debug",
            )
        });
    let fmt = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact();
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt)
        .with(BufferLayer::new(buffer))
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_is_bounded_and_filtered() {
        let buf = LogBuffer::new(3, None);
        for i in 0..5 {
            buf.push(LogEntry {
                id: new_id(),
                at: now(),
                level: if i % 2 == 0 {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                },
                target: "t".into(),
                message: format!("m{i}"),
            });
        }
        let all = buf.entries(LogLevel::Debug, 100);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].message, "m2");
        let errors = buf.entries(LogLevel::Error, 100);
        assert_eq!(errors.len(), 1);
        assert!(buf.export_text().contains("m4"));
        buf.clear();
        assert!(buf.entries(LogLevel::Debug, 10).is_empty());
    }
}
