//! Job Hunter core.
//!
//! Everything that is not UI lives here: the domain model, local-first
//! persistence with MongoDB Atlas synchronisation, Claude CLI process
//! integration, Chrome discovery, document import/rendering, the agent state
//! machine and the orchestrator that runs a job hunt end to end.
//!
//! The crate deliberately has no dependency on Tauri so that it can be unit
//! tested with plain `cargo test`.

pub mod agent;
pub mod chrome;
pub mod claude;
pub mod context;
pub mod dedup;
pub mod documents;
pub mod domain;
pub mod error;
pub mod logging;
pub mod paths;
pub mod prompts;
pub mod remote;
pub mod secrets;
pub mod settings;
pub mod store;
pub mod util;

pub use context::AppContext;
pub use error::{CoreError, CoreResult};
