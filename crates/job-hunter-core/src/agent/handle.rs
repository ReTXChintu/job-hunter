use std::sync::Arc;

use tokio::sync::{watch, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::events::EventBus;
use super::state::can_transition;
use crate::domain::*;
use crate::error::{CoreError, CoreResult};
use crate::util::now;

/// Live agent state shared between the orchestrator task and the UI commands.
pub struct AgentHandle {
    status: RwLock<AgentStatus>,
    cancel: Mutex<Option<CancellationToken>>,
    task: Mutex<Option<JoinHandle<()>>>,
    paused: watch::Sender<bool>,
    pub bus: EventBus,
}

impl Default for AgentHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandle {
    pub fn new() -> Self {
        let (paused, _) = watch::channel(false);
        Self {
            status: RwLock::new(AgentStatus::default()),
            cancel: Mutex::new(None),
            task: Mutex::new(None),
            paused,
            bus: EventBus::new(),
        }
    }

    pub async fn status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    pub async fn is_busy(&self) -> bool {
        let s = self.status.read().await;
        s.state.is_running() || s.paused
    }

    /// Claim the agent for a new run. Fails when a run is in progress.
    pub async fn begin(&self, run: &AgentRun) -> CoreResult<CancellationToken> {
        let mut status = self.status.write().await;
        if status.state.is_running() || status.paused {
            return Err(CoreError::AgentBusy(format!(
                "agent is {}",
                status.state.as_str()
            )));
        }
        if !can_transition(status.state, AgentState::Initializing) {
            return Err(CoreError::InvalidTransition(format!(
                "{} -> INITIALIZING",
                status.state.as_str()
            )));
        }
        let token = CancellationToken::new();
        *self.cancel.lock().await = Some(token.clone());
        self.paused.send_replace(false);
        *status = AgentStatus {
            state: AgentState::Initializing,
            run_id: Some(run.id.clone()),
            run_kind: Some(run.kind),
            current_activity: Some("Starting".into()),
            progress: None,
            stats: RunStats::default(),
            started_at: Some(run.started_at),
            error: None,
            mock: run.mock,
            paused: false,
        };
        self.bus.emit_status(status.clone());
        Ok(token)
    }

    pub async fn set_task(&self, task: JoinHandle<()>) {
        *self.task.lock().await = Some(task);
    }

    pub async fn transition(&self, to: AgentState, run_id: &str) -> CoreResult<()> {
        let mut status = self.status.write().await;
        let from = status.state;
        if !can_transition(from, to) {
            return Err(CoreError::InvalidTransition(format!(
                "{} -> {}",
                from.as_str(),
                to.as_str()
            )));
        }
        if from != to {
            status.state = to;
            if to.is_terminal() {
                status.paused = false;
            }
            self.bus.emit(
                AgentEvent::new(
                    run_id,
                    EventLevel::Info,
                    "STATE_CHANGED",
                    format!("{} → {}", from.as_str(), to.as_str()),
                )
                .with_data(serde_json::json!({ "from": from, "to": to })),
            );
            self.bus.emit_status(status.clone());
        }
        Ok(())
    }

    pub async fn update<F: FnOnce(&mut AgentStatus)>(&self, f: F) {
        let mut status = self.status.write().await;
        f(&mut status);
        self.bus.emit_status(status.clone());
    }

    pub async fn finish(&self, final_state: AgentState, error: Option<String>, run_id: &str) {
        let mut status = self.status.write().await;
        let from = status.state;
        status.state = final_state;
        status.error = error.clone();
        status.paused = false;
        status.current_activity = None;
        status.progress = None;
        drop(status);
        *self.cancel.lock().await = None;
        self.paused.send_replace(false);
        let level = if error.is_some() {
            EventLevel::Error
        } else {
            EventLevel::Success
        };
        self.bus.emit(
            AgentEvent::new(
                run_id,
                level,
                "RUN_FINISHED",
                error
                    .clone()
                    .unwrap_or_else(|| format!("Finished: {}", final_state.as_str())),
            )
            .with_data(serde_json::json!({ "from": from, "to": final_state, "at": now() })),
        );
        self.bus.emit_status(self.status.read().await.clone());
    }

    pub async fn request_stop(&self) -> bool {
        let token = self.cancel.lock().await.clone();
        self.paused.send_replace(false);
        match token {
            Some(t) => {
                t.cancel();
                let mut status = self.status.write().await;
                if status.state.is_running() || status.paused {
                    status.state = AgentState::Stopping;
                    status.paused = false;
                    status.current_activity = Some("Stopping".into());
                    self.bus.emit_status(status.clone());
                }
                true
            }
            None => false,
        }
    }

    pub async fn pause(&self) -> CoreResult<()> {
        let mut status = self.status.write().await;
        if !status.state.is_running() {
            return Err(CoreError::InvalidTransition("agent is not running".into()));
        }
        status.paused = true;
        self.paused.send_replace(true);
        self.bus.emit_status(status.clone());
        if let Some(run_id) = status.run_id.clone() {
            self.bus.emit(AgentEvent::new(
                &run_id,
                EventLevel::Warn,
                "MESSAGE",
                "Pause requested; the agent will pause after the current step",
            ));
        }
        Ok(())
    }

    pub async fn resume(&self) -> CoreResult<()> {
        let mut status = self.status.write().await;
        if !status.paused {
            return Err(CoreError::InvalidTransition("agent is not paused".into()));
        }
        status.paused = false;
        self.paused.send_replace(false);
        self.bus.emit_status(status.clone());
        if let Some(run_id) = status.run_id.clone() {
            self.bus.emit(AgentEvent::new(
                &run_id,
                EventLevel::Info,
                "MESSAGE",
                "Resumed",
            ));
        }
        Ok(())
    }

    /// Await here between units of work; returns Err(Cancelled) if stopped
    /// while paused.
    pub async fn checkpoint(&self, cancel: &CancellationToken) -> CoreResult<()> {
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        let mut rx = self.paused.subscribe();
        if *rx.borrow() {
            let previous = {
                let mut status = self.status.write().await;
                let prev = status.state;
                if can_transition(prev, AgentState::Paused) {
                    status.state = AgentState::Paused;
                    status.current_activity = Some("Paused".into());
                    self.bus.emit_status(status.clone());
                }
                prev
            };
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return Err(CoreError::Cancelled),
                    changed = rx.changed() => {
                        if changed.is_err() || !*rx.borrow() { break; }
                    }
                }
            }
            let mut status = self.status.write().await;
            if status.state == AgentState::Paused {
                status.state = previous;
                status.current_activity = None;
                self.bus.emit_status(status.clone());
            }
        }
        Ok(())
    }
}

pub type SharedHandle = Arc<AgentHandle>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn begin_rejects_concurrent_runs_and_finish_releases() {
        let h = AgentHandle::new();
        let run = AgentRun::new("u", RunKind::JobHunt, true);
        let token = h.begin(&run).await.unwrap();
        assert!(h.begin(&run).await.is_err());
        h.transition(AgentState::Discovering, &run.id)
            .await
            .unwrap();
        assert!(h.transition(AgentState::Applying, &run.id).await.is_err());
        assert!(h.request_stop().await);
        assert!(token.is_cancelled());
        h.finish(AgentState::Completed, None, &run.id).await;
        assert_eq!(h.status().await.state, AgentState::Completed);
        assert!(h.begin(&run).await.is_ok());
    }

    #[tokio::test]
    async fn pause_blocks_checkpoint_until_resumed() {
        let h = Arc::new(AgentHandle::new());
        let run = AgentRun::new("u", RunKind::JobHunt, true);
        let token = h.begin(&run).await.unwrap();
        h.transition(AgentState::Discovering, &run.id)
            .await
            .unwrap();
        h.pause().await.unwrap();
        let h2 = h.clone();
        let t = token.clone();
        let waiter = tokio::spawn(async move { h2.checkpoint(&t).await });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(h.status().await.state, AgentState::Paused);
        h.resume().await.unwrap();
        waiter.await.unwrap().unwrap();
        assert_eq!(h.status().await.state, AgentState::Discovering);
    }
}
