//! Claude Code / Claude CLI integration.
//!
//! Job Hunter never talks to the Anthropic API. Every AI step spawns the
//! user's own authenticated `claude` executable in non-interactive print mode
//! (`claude -p --output-format stream-json`) and reads its structured output.

pub mod discover;
pub mod mock;
pub mod protocol;
pub mod runner;

pub use discover::{ClaudeCli, ClaudeStatus};
pub use mock::MockClaudeRunner;
pub use protocol::{ClaudeEvent, ClaudeRequest, ClaudeResponse, EventSink};
pub use runner::{ClaudeRunner, CliClaudeRunner};
