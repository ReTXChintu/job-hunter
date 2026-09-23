use crate::domain::AgentState;

/// Allowed state transitions of the agent.
pub fn can_transition(from: AgentState, to: AgentState) -> bool {
    use AgentState as S;
    if from == to {
        return true;
    }
    match from {
        S::Idle => matches!(to, S::Initializing),
        S::Initializing => matches!(
            to,
            S::Discovering
                | S::Analyzing
                | S::PreparingApplications
                | S::Applying
                | S::Failed
                | S::Stopping
                | S::Completed
        ),
        S::Discovering => matches!(
            to,
            S::Extracting | S::Deduplicating | S::Paused | S::Stopping | S::Failed | S::Completed
        ),
        S::Extracting => matches!(
            to,
            S::Deduplicating | S::Paused | S::Stopping | S::Failed | S::Completed
        ),
        S::Deduplicating => matches!(to, S::Analyzing | S::Completed | S::Stopping | S::Failed),
        S::Analyzing => matches!(
            to,
            S::PreparingApplications | S::Completed | S::Paused | S::Stopping | S::Failed
        ),
        S::PreparingApplications => matches!(
            to,
            S::WaitingForApproval | S::Completed | S::Paused | S::Stopping | S::Failed
        ),
        S::WaitingForApproval => {
            matches!(to, S::Applying | S::Completed | S::Idle | S::Initializing)
        }
        S::Applying => matches!(
            to,
            S::Completed | S::ManualActionRequired | S::WaitingForUser | S::Failed | S::Stopping
        ),
        S::Paused => matches!(
            to,
            S::Discovering
                | S::Extracting
                | S::Analyzing
                | S::PreparingApplications
                | S::Stopping
                | S::Completed
                | S::Failed
        ),
        S::Stopping => matches!(to, S::Completed | S::Failed),
        S::Completed | S::Failed | S::ManualActionRequired | S::WaitingForUser => {
            matches!(to, S::Idle | S::Initializing | S::Applying)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AgentState as S;

    #[test]
    fn happy_path_is_allowed() {
        let path = [
            S::Idle,
            S::Initializing,
            S::Discovering,
            S::Extracting,
            S::Deduplicating,
            S::Analyzing,
            S::PreparingApplications,
            S::WaitingForApproval,
            S::Applying,
            S::Completed,
        ];
        for w in path.windows(2) {
            assert!(can_transition(w[0], w[1]), "{:?} -> {:?}", w[0], w[1]);
        }
    }

    #[test]
    fn cannot_skip_to_applying_from_discovery() {
        assert!(!can_transition(S::Discovering, S::Applying));
        assert!(!can_transition(S::Idle, S::Applying));
        assert!(!can_transition(S::Analyzing, S::Applying));
    }

    #[test]
    fn pause_and_resume() {
        assert!(can_transition(S::Discovering, S::Paused));
        assert!(can_transition(S::Paused, S::Discovering));
        assert!(!can_transition(S::Paused, S::Applying));
    }

    #[test]
    fn terminal_states_restart() {
        for s in [
            S::Completed,
            S::Failed,
            S::ManualActionRequired,
            S::WaitingForUser,
        ] {
            assert!(can_transition(s, S::Initializing));
            assert!(s.is_terminal());
        }
    }
}
