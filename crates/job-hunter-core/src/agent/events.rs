use tokio::sync::broadcast;

use crate::domain::{AgentEvent, AgentStatus};

/// Fan-out of agent events and status snapshots to any number of listeners
/// (the Tauri layer forwards them to the UI).
pub struct EventBus {
    events: broadcast::Sender<AgentEvent>,
    status: broadcast::Sender<AgentStatus>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(2048);
        let (status, _) = broadcast::channel(256);
        Self { events, status }
    }

    pub fn emit(&self, event: AgentEvent) {
        let _ = self.events.send(event);
    }

    pub fn emit_status(&self, status: AgentStatus) {
        let _ = self.status.send(status);
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentEvent> {
        self.events.subscribe()
    }

    pub fn subscribe_status(&self) -> broadcast::Receiver<AgentStatus> {
        self.status.subscribe()
    }
}
