use autoclick_domain::types::RuntimeStatus;
use serde::{Deserialize, Serialize};

use crate::RuntimeError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StateEvent {
    RequestStart,
    CaptureReady,
    EnterCooldown,
    CooldownElapsed,
    RequestRecover,
    RecoverSuccess,
    RecoverFailure(String),
    RequestStop,
    Stopped,
    ResetFault,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StateTransition {
    pub from: RuntimeStatus,
    pub event: StateEvent,
    pub to: RuntimeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStateMachine {
    state: RuntimeStatus,
}

impl Default for RuntimeStateMachine {
    fn default() -> Self {
        Self {
            state: RuntimeStatus::Idle,
        }
    }
}

impl RuntimeStateMachine {
    pub fn state(&self) -> RuntimeStatus {
        self.state
    }

    pub fn apply(&mut self, event: StateEvent) -> Result<StateTransition, RuntimeError> {
        let from = self.state;
        let to = transition(from, &event)?;
        self.state = to;
        Ok(StateTransition { from, event, to })
    }
}

pub fn transition(state: RuntimeStatus, event: &StateEvent) -> Result<RuntimeStatus, RuntimeError> {
    use RuntimeStatus as S;

    let next = match (state, event) {
        (S::Idle, StateEvent::RequestStart) => S::Starting,
        (S::Starting, StateEvent::CaptureReady) => S::Running,
        (S::Running, StateEvent::EnterCooldown) => S::CoolingDown,
        (S::CoolingDown, StateEvent::CooldownElapsed) => S::Running,
        (S::Running, StateEvent::RequestRecover) => S::Recovering,
        (S::CoolingDown, StateEvent::RequestRecover) => S::Recovering,
        (S::Recovering, StateEvent::RecoverSuccess) => S::Running,
        (S::Recovering, StateEvent::RecoverFailure(_)) => S::Faulted,
        (S::Starting, StateEvent::RequestStop) => S::Stopping,
        (S::Running, StateEvent::RequestStop) => S::Stopping,
        (S::CoolingDown, StateEvent::RequestStop) => S::Stopping,
        (S::Recovering, StateEvent::RequestStop) => S::Stopping,
        (S::Stopping, StateEvent::Stopped) => S::Idle,
        (S::Faulted, StateEvent::ResetFault) => S::Idle,
        _ => {
            return Err(RuntimeError::State(format!(
                "非法状态迁移: {:?} + {:?}",
                state, event
            )));
        }
    };
    Ok(next)
}

#[cfg(test)]
mod tests {
    use autoclick_domain::types::RuntimeStatus;

    use super::{RuntimeStateMachine, StateEvent};

    #[test]
    fn state_machine_follows_expected_path() {
        let mut machine = RuntimeStateMachine::default();
        assert_eq!(machine.state(), RuntimeStatus::Idle);
        machine.apply(StateEvent::RequestStart).expect("start");
        machine.apply(StateEvent::CaptureReady).expect("ready");
        machine.apply(StateEvent::EnterCooldown).expect("cooldown");
        machine
            .apply(StateEvent::CooldownElapsed)
            .expect("cooldown elapsed");
        machine.apply(StateEvent::RequestStop).expect("stop");
        machine.apply(StateEvent::Stopped).expect("stopped");
        assert_eq!(machine.state(), RuntimeStatus::Idle);
    }

    #[test]
    fn state_machine_rejects_invalid_transition() {
        let mut machine = RuntimeStateMachine::default();
        assert!(machine.apply(StateEvent::Stopped).is_err());
    }
}
