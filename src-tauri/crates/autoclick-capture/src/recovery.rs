use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    CaptureError,
    session::{CaptureSession, CaptureSessionConfig},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryReason {
    ItemClosed,
    TargetSwitched,
    SizeChanged,
    BackendFault,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPolicy {
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_backoff_ms: 250,
            max_backoff_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryAction {
    pub attempt: u32,
    pub should_retry: bool,
    pub wait_for_ms: u64,
    pub reason: RecoveryReason,
}

#[derive(Debug, Default)]
pub struct RecoveryState {
    attempts: u32,
}

impl RecoveryState {
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
    }

    pub fn register_failure(
        &mut self,
        reason: RecoveryReason,
        policy: &RecoveryPolicy,
    ) -> RecoveryAction {
        self.attempts += 1;
        let should_retry = self.attempts <= policy.max_attempts;
        let exponent = self.attempts.saturating_sub(1);
        let backoff = if should_retry {
            policy
                .base_backoff_ms
                .saturating_mul(2u64.saturating_pow(exponent))
                .min(policy.max_backoff_ms)
        } else {
            0
        };

        RecoveryAction {
            attempt: self.attempts,
            should_retry,
            wait_for_ms: backoff,
            reason,
        }
    }
}

pub fn restart_session_with_backoff(
    session: &mut CaptureSession,
    config: CaptureSessionConfig,
    state: &mut RecoveryState,
    policy: &RecoveryPolicy,
    reason: RecoveryReason,
) -> Result<RecoveryAction, CaptureError> {
    let action = state.register_failure(reason, policy);
    if action.should_retry {
        restart_session(session, config, Duration::from_millis(action.wait_for_ms))?;
    }
    Ok(action)
}

pub fn restart_session(
    session: &mut CaptureSession,
    config: CaptureSessionConfig,
    backoff: Duration,
) -> Result<(), CaptureError> {
    session.stop()?;
    if !backoff.is_zero() {
        std::thread::sleep(backoff);
    }
    session.start(config)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        CaptureError,
        backend::{CaptureFactory, CaptureSharedState, RunningCapture, WgcCaptureOptions},
        session::{CaptureSession, CaptureSessionConfig, CaptureTarget},
    };

    use super::{RecoveryPolicy, RecoveryReason, RecoveryState, restart_session_with_backoff};

    struct FakeRunner;

    impl RunningCapture for FakeRunner {
        fn stop(&mut self) -> Result<(), CaptureError> {
            Ok(())
        }

        fn is_finished(&self) -> bool {
            false
        }
    }

    struct FakeFactory;

    impl CaptureFactory for FakeFactory {
        fn start_window(
            &self,
            _hwnd: isize,
            _options: &WgcCaptureOptions,
            _shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            Ok(Box::new(FakeRunner))
        }

        fn start_monitor(
            &self,
            _monitor_handle: Option<isize>,
            _options: &WgcCaptureOptions,
            _shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            Ok(Box::new(FakeRunner))
        }
    }

    #[test]
    fn recovery_backoff_grows_and_clamps() {
        let policy = RecoveryPolicy {
            max_attempts: 5,
            base_backoff_ms: 100,
            max_backoff_ms: 300,
        };
        let mut state = RecoveryState::default();
        assert_eq!(
            state
                .register_failure(RecoveryReason::BackendFault, &policy)
                .wait_for_ms,
            100
        );
        assert_eq!(
            state
                .register_failure(RecoveryReason::BackendFault, &policy)
                .wait_for_ms,
            200
        );
        assert_eq!(
            state
                .register_failure(RecoveryReason::BackendFault, &policy)
                .wait_for_ms,
            300
        );
    }

    #[test]
    fn recovery_restart_restarts_session() {
        let mut session = CaptureSession::with_factory(Arc::new(FakeFactory));
        session
            .start(CaptureSessionConfig {
                target: CaptureTarget::Window { hwnd: 100 },
                options: WgcCaptureOptions::default(),
            })
            .expect("start");

        let action = restart_session_with_backoff(
            &mut session,
            CaptureSessionConfig {
                target: CaptureTarget::Monitor { handle: None },
                options: WgcCaptureOptions::default(),
            },
            &mut RecoveryState::default(),
            &RecoveryPolicy {
                max_attempts: 2,
                base_backoff_ms: 0,
                max_backoff_ms: 0,
            },
            RecoveryReason::TargetSwitched,
        )
        .expect("restart");

        assert!(action.should_retry);
        assert!(session.is_running());
    }
}
