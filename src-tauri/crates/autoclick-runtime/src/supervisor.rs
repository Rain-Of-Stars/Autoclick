use autoclick_capture::{
    recovery::{
        RecoveryAction, RecoveryPolicy, RecoveryReason, RecoveryState, restart_session,
        restart_session_with_backoff,
    },
    session::{CaptureSession, CaptureSessionConfig},
};
use autoclick_domain::config::TargetProfile;
use autoclick_platform_win::locator::{LocatorCandidate, locate_target_window};
use serde::{Deserialize, Serialize};

use crate::{RuntimeError, metrics::RuntimeMetrics};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorReport {
    pub located: Option<LocatorCandidate>,
    pub recovery: Option<RecoveryAction>,
}

#[derive(Debug)]
pub struct RuntimeSupervisor {
    recovery_policy: RecoveryPolicy,
    recovery_state: RecoveryState,
}

impl RuntimeSupervisor {
    pub fn new(recovery_policy: RecoveryPolicy) -> Self {
        Self {
            recovery_policy,
            recovery_state: RecoveryState::default(),
        }
    }

    pub fn locate_target(
        &self,
        target: &TargetProfile,
    ) -> Result<Option<LocatorCandidate>, RuntimeError> {
        locate_target_window(target).map_err(|err| RuntimeError::State(err.to_string()))
    }

    pub fn recovery_attempts(&self) -> u32 {
        self.recovery_state.attempts()
    }

    pub fn mark_healthy(&mut self) {
        self.recovery_state.reset();
    }

    pub fn plan_recovery(&mut self, reason: RecoveryReason) -> RecoveryAction {
        self.recovery_state
            .register_failure(reason, &self.recovery_policy)
    }

    pub fn restart_capture(
        &mut self,
        session: &mut CaptureSession,
        config: CaptureSessionConfig,
        action: &RecoveryAction,
    ) -> Result<(), RuntimeError> {
        if !action.should_retry {
            return Ok(());
        }

        restart_session(
            session,
            config,
            std::time::Duration::from_millis(action.wait_for_ms),
        )
        .map_err(|err| RuntimeError::Capture(err.to_string()))
    }

    pub fn recover_capture(
        &mut self,
        session: &mut CaptureSession,
        config: CaptureSessionConfig,
        reason: RecoveryReason,
    ) -> Result<RecoveryAction, RuntimeError> {
        restart_session_with_backoff(
            session,
            config,
            &mut self.recovery_state,
            &self.recovery_policy,
            reason,
        )
        .map_err(|err| RuntimeError::Capture(err.to_string()))
    }

    pub fn handle_capture_failure(
        &mut self,
        session: &mut CaptureSession,
        config: CaptureSessionConfig,
        reason: RecoveryReason,
        metrics: &mut RuntimeMetrics,
    ) -> Result<RecoveryAction, RuntimeError> {
        let action = self.recover_capture(session, config, reason.clone())?;
        metrics.record_recovery(
            self.recovery_state.attempts(),
            Some(format!("{reason:?}")),
            Some(action.wait_for_ms),
        );
        Ok(action)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use autoclick_capture::{
        CaptureError, CaptureFactory, CaptureSharedState, RunningCapture, WgcCaptureOptions,
        recovery::{RecoveryPolicy, RecoveryReason},
        session::{CaptureSession, CaptureSessionConfig, CaptureTarget},
    };

    use super::RuntimeSupervisor;
    use crate::metrics::RuntimeMetrics;

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
    fn supervisor_restarts_session_and_updates_metrics() {
        let mut session = CaptureSession::with_factory(Arc::new(FakeFactory));
        session
            .start(CaptureSessionConfig {
                target: CaptureTarget::Window { hwnd: 100 },
                options: WgcCaptureOptions::default(),
            })
            .expect("start");
        let mut supervisor = RuntimeSupervisor::new(RecoveryPolicy {
            max_attempts: 2,
            base_backoff_ms: 0,
            max_backoff_ms: 0,
        });
        let mut metrics = RuntimeMetrics::default();
        let action = supervisor
            .handle_capture_failure(
                &mut session,
                CaptureSessionConfig {
                    target: CaptureTarget::Monitor { handle: None },
                    options: WgcCaptureOptions::default(),
                },
                RecoveryReason::BackendFault,
                &mut metrics,
            )
            .expect("recovery");
        assert!(action.should_retry);
        assert_eq!(metrics.snapshot().recovery_count, 1);
    }
}
