//! The agent: an explicit state machine, an event bus and an orchestrator that
//! runs the job-hunt pipeline step by step, persisting after each step.

pub mod events;
pub mod handle;
pub mod orchestrator;
pub mod state;
pub mod steps;

pub use events::EventBus;
pub use handle::AgentHandle;
pub use state::can_transition;
