use std::{
    panic::{self, AssertUnwindSafe},
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use autoclick_capture::{
    CaptureError, WgcCaptureOptions,
    frame::FramePacket,
    recovery::{RecoveryPolicy as CaptureRecoveryPolicy, RecoveryReason},
    session::{CaptureSession, CaptureSessionConfig, CaptureTarget},
};
use autoclick_detect::{
    hit_policy::{HitDecision, HitPolicyConfig},
    r#match::MatchResult,
    template_store::{LoadedTemplate, TemplateStore},
};
use autoclick_domain::{
    config::{AppConfig, CaptureSource},
    paths::AppPaths,
    runtime_snapshot::RuntimeSnapshot,
    template::TemplateRef,
    types::RuntimeStatus,
};
use autoclick_input::post_message::ClickReport;
use autoclick_platform_win::locator::LocatorCandidate;
use autoclick_runtime::{
    metrics::RuntimeMetricsSnapshot,
    preview_bus::{PreviewBusConfig, PreviewMessage},
    scanner_engine::{
        PolicyClickExecutor, PreviewIteration, ScanIteration, ScannerEngine, ScannerEngineConfig,
    },
    shutdown::ShutdownSignal,
    state_machine::{RuntimeStateMachine, StateEvent},
    supervisor::RuntimeSupervisor,
};
use autoclick_storage::{repo_run::RunRepository, repo_template::TemplateRepository};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::capture_window::{ensure_window_capture_ready, finish_window_capture};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeControllerSnapshot {
    pub status: RuntimeStatus,
    pub metrics: RuntimeMetricsSnapshot,
    pub preview: Option<PreviewMessage>,
    pub active_target: Option<LocatorCandidate>,
    pub best_match: Option<MatchResult>,
    pub decision: Option<HitDecision>,
    pub last_click: Option<ClickReport>,
    pub last_error: Option<String>,
}

impl Default for RuntimeControllerSnapshot {
    fn default() -> Self {
        let mut metrics = RuntimeMetricsSnapshot {
            runtime: RuntimeSnapshot::default(),
            recovery_count: 0,
            buffer_drops: 0,
            memory_bytes_estimate: 0,
        };
        metrics.runtime.status = RuntimeStatus::Idle;
        Self {
            status: RuntimeStatus::Idle,
            metrics,
            preview: None,
            active_target: None,
            best_match: None,
            decision: None,
            last_click: None,
            last_error: None,
        }
    }
}

pub struct RuntimeController {
    shared: Arc<RwLock<RuntimeControllerSnapshot>>,
    template_store: Arc<TemplateStore>,
    inner: Mutex<RuntimeControllerState>,
}

impl Default for RuntimeController {
    fn default() -> Self {
        Self {
            shared: Arc::new(RwLock::new(RuntimeControllerSnapshot::default())),
            template_store: Arc::new(TemplateStore::new()),
            inner: Mutex::new(RuntimeControllerState::default()),
        }
    }
}

#[derive(Default)]
struct RuntimeControllerState {
    machine: RuntimeStateMachine,
    worker: Option<ScannerWorkerHandle>,
}

struct ScannerWorkerHandle {
    shutdown: ShutdownSignal,
    join: JoinHandle<()>,
}

impl RuntimeController {
    pub fn snapshot(&self) -> RuntimeControllerSnapshot {
        let mut inner = self.inner.lock();
        self.cleanup_finished_worker_locked(&mut inner);
        self.shared.read().clone()
    }

    pub fn start(
        &self,
        app_paths: AppPaths,
        config: AppConfig,
        prefetched_target: Option<LocatorCandidate>,
    ) -> Result<RuntimeControllerSnapshot, String> {
        let mut inner = self.inner.lock();
        self.cleanup_finished_worker_locked(&mut inner);
        if inner.worker.is_some() {
            return Err("扫描会话已在运行".to_string());
        }
        validate_runtime_config(&config)?;

        inner.machine = RuntimeStateMachine::default();
        inner
            .machine
            .apply(StateEvent::RequestStart)
            .map_err(|err| err.to_string())?;
        set_status(&self.shared, RuntimeStatus::Starting);
        clear_error(&self.shared);

        let shutdown = ShutdownSignal::default();
        let shared = self.shared.clone();
        let worker_shutdown = shutdown.clone();
        let template_store = self.template_store.clone();
        let join = thread::spawn(move || {
            run_scanner_worker(
                shared,
                worker_shutdown,
                template_store,
                app_paths,
                config,
                prefetched_target,
            )
        });
        inner.worker = Some(ScannerWorkerHandle { shutdown, join });
        Ok(self.shared.read().clone())
    }

    pub fn stop(&self) -> Result<RuntimeControllerSnapshot, String> {
        self.stop_inner(Duration::from_millis(500))
    }

    /// 发送停止信号并等待工作线程退出（带超时）。
    fn stop_inner(&self, join_timeout: Duration) -> Result<RuntimeControllerSnapshot, String> {
        {
            let mut inner = self.inner.lock();
            self.cleanup_finished_worker_locked(&mut inner);

            if inner.worker.is_none() {
                inner.machine = RuntimeStateMachine::default();
                set_status(&self.shared, RuntimeStatus::Idle);
                return Ok(self.shared.read().clone());
            }

            if inner.machine.state() != RuntimeStatus::Stopping {
                let _ = inner.machine.apply(StateEvent::RequestStop);
            }
            set_status(&self.shared, RuntimeStatus::Stopping);
            if let Some(worker) = inner.worker.as_ref() {
                worker.shutdown.request();
            }
            // 释放 inner 锁后再等待
        }

        // 等待工作线程退出，轮询检查以避免持锁阻塞
        let deadline = Instant::now() + join_timeout;
        while Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
            let mut inner = self.inner.lock();
            self.cleanup_finished_worker_locked(&mut inner);
            if inner.worker.is_none() {
                return Ok(self.shared.read().clone());
            }
        }

        Ok(self.shared.read().clone())
    }

    pub fn restart(
        &self,
        app_paths: AppPaths,
        config: AppConfig,
        prefetched_target: Option<LocatorCandidate>,
    ) -> Result<RuntimeControllerSnapshot, String> {
        // 先等旧工作线程退出（更长的超时）
        self.stop_inner(Duration::from_millis(2000))?;
        {
            let inner = self.inner.lock();
            if inner.worker.is_some() {
                return Err("停止超时：旧扫描线程未能在规定时间内退出".to_string());
            }
        }
        self.start(app_paths, config, prefetched_target)
    }

    pub fn invalidate_template_cache(&self, hash: &str) {
        self.template_store.invalidate(hash);
    }

    fn cleanup_finished_worker_locked(&self, inner: &mut RuntimeControllerState) {
        if inner
            .worker
            .as_ref()
            .is_some_and(|worker| worker.join.is_finished())
        {
            if let Some(worker) = inner.worker.take() {
                let _ = worker.join.join();
            }
            inner.machine = RuntimeStateMachine::default();
            let faulted = self.shared.read().status == RuntimeStatus::Faulted;
            if !faulted {
                set_status(&self.shared, RuntimeStatus::Idle);
            }
        }
    }
}

fn run_scanner_worker(
    shared: Arc<RwLock<RuntimeControllerSnapshot>>,
    shutdown: ShutdownSignal,
    template_store: Arc<TemplateStore>,
    app_paths: AppPaths,
    config: AppConfig,
    prefetched_target: Option<LocatorCandidate>,
) {
    let run_repository = RunRepository::new(&app_paths.db_path);
    let run_id = run_repository.start_run("starting").ok();
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        run_scanner_loop(
            &shared,
            &shutdown,
            &template_store,
            &app_paths,
            config,
            prefetched_target,
        )
    }))
    .map_err(panic_payload_to_string)
    .and_then(|result| result);

    match result {
        Ok(WorkerExit::Stopped) => {
            if let Some(run_id) = run_id {
                let _ = run_repository.finish_run(run_id, "stopped");
            }
            set_status(&shared, RuntimeStatus::Idle);
        }
        Err(err) => {
            error!("扫描运行时故障: {err}");
            if let Some(run_id) = run_id {
                let _ = run_repository.finish_run(run_id, "faulted");
            }
            set_faulted(&shared, err);
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return format!("扫描线程发生 panic: {message}");
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return format!("扫描线程发生 panic: {message}");
    }
    "扫描线程发生 panic".to_string()
}

enum WorkerExit {
    Stopped,
}

const FRAME_WAIT_SLICE: Duration = Duration::from_millis(120);

fn read_next_frame_cancelable(
    session: &CaptureSession,
    last_frame_id: u64,
    timeout: Duration,
    shutdown: &ShutdownSignal,
) -> Result<Option<FramePacket>, CaptureError> {
    let deadline = Instant::now() + timeout;

    loop {
        if shutdown.is_requested() {
            return Ok(None);
        }

        let now = Instant::now();
        if now >= deadline {
            return Err(CaptureError::Timeout);
        }

        let remaining = deadline.saturating_duration_since(now);
        let wait_step = remaining.min(FRAME_WAIT_SLICE);
        match session.read_next_frame(last_frame_id, wait_step) {
            Ok(frame) => return Ok(Some(frame)),
            Err(CaptureError::Timeout) => continue,
            Err(CaptureError::ItemClosed) if shutdown.is_requested() => return Ok(None),
            Err(err) => return Err(err),
        }
    }
}

fn map_capture_recovery_reason(error: &CaptureError) -> RecoveryReason {
    match error {
        CaptureError::ItemClosed => RecoveryReason::ItemClosed,
        _ => RecoveryReason::BackendFault,
    }
}

fn run_scanner_loop(
    shared: &Arc<RwLock<RuntimeControllerSnapshot>>,
    shutdown: &ShutdownSignal,
    template_store: &Arc<TemplateStore>,
    app_paths: &AppPaths,
    config: AppConfig,
    prefetched_target: Option<LocatorCandidate>,
) -> Result<WorkerExit, String> {
    let mut supervisor = RuntimeSupervisor::new(map_recovery_policy(&config));
    let located = resolve_located_target(&mut supervisor, &config, prefetched_target)?;
    update_target(shared, Some(located.clone()));
    let mut restored_from_minimized = if matches!(config.capture.source, CaptureSource::Window) {
        ensure_window_capture_ready(&config.capture, located.window.hwnd)?
    } else {
        false
    };

    let run_result = {
        let capture_config = build_capture_session_config(&config, located.window.hwnd);
        let scanner_config = build_scanner_config(&config, &located);
        let mut session = CaptureSession::new();
        session
            .start(capture_config.clone())
            .map_err(|err| err.to_string())?;
        let templates = match resolve_startup_templates(app_paths, &config) {
            Ok(templates) => templates,
            Err(err) => {
                let _ = session.stop();
                return Err(err);
            }
        };
        let template_loader_templates = templates;
        let template_loader_store = template_store.clone();
        let mut template_loader = Some(thread::spawn(move || {
            template_loader_store
                .load_all(&template_loader_templates)
                .map_err(|err| err.to_string())
        }));
        let mut loaded_templates = None;
        let mut preview_primed = false;

        let mut engine = ScannerEngine::new(
            HitPolicyConfig {
                threshold: config.detection.threshold,
                min_detections: config.detection.min_detections,
                cooldown_ms: config.detection.cooldown_ms,
            },
            build_preview_config(&config),
            PolicyClickExecutor,
        );
        let timeout = Duration::from_millis(config.capture.timeout_ms.clamp(50, 1_000));
        let mut last_frame_id = 0u64;

        loop {
            if shutdown.is_requested() {
                let _ = session.stop();
                break Ok(WorkerExit::Stopped);
            }

            let frame = match read_next_frame_cancelable(&session, last_frame_id, timeout, shutdown)
            {
                Ok(Some(frame)) => frame,
                Ok(None) => {
                    let _ = session.stop();
                    break Ok(WorkerExit::Stopped);
                }
                Err(err) => {
                    if shutdown.is_requested() {
                        let _ = session.stop();
                        break Ok(WorkerExit::Stopped);
                    }

                    if !config.recovery.enable_auto_recovery {
                        let _ = session.stop();
                        break Err(format!("读取捕获帧失败: {err}"));
                    }

                    set_status(shared, RuntimeStatus::Recovering);
                    let action = supervisor.plan_recovery(map_capture_recovery_reason(&err));
                    engine.record_recovery(
                        supervisor.recovery_attempts(),
                        Some(format!("{:?}", action.reason)),
                        Some(action.wait_for_ms),
                    );
                    update_metrics_only(
                        shared,
                        RuntimeStatus::Recovering,
                        engine.metrics_snapshot(),
                        engine.latest_preview(),
                    );

                    if !action.should_retry {
                        let _ = session.stop();
                        break Err("自动恢复次数已耗尽".to_string());
                    }

                    if shutdown.sleep_cancelable(Duration::from_millis(action.wait_for_ms)) {
                        let _ = session.stop();
                        break Ok(WorkerExit::Stopped);
                    }

                    if matches!(config.capture.source, CaptureSource::Window) {
                        restored_from_minimized |=
                            ensure_window_capture_ready(&config.capture, located.window.hwnd)?;
                    }

                    supervisor
                        .restart_capture(&mut session, capture_config.clone(), &action)
                        .map_err(|err| err.to_string())?;
                    set_status(shared, RuntimeStatus::Running);
                    continue;
                }
            };

            last_frame_id = frame.frame_id;
            let stats = session.snapshot().stats;
            if loaded_templates.is_none() {
                loaded_templates = try_collect_loaded_templates(&mut template_loader)?;
            }
            if !preview_primed {
                let preview_iteration = engine
                    .process_preview_frame(&frame, stats)
                    .map_err(|err| err.to_string())?;
                apply_starting_preview(shared, &located, preview_iteration);
                preview_primed = true;
            }
            if loaded_templates.is_none() {
                continue;
            }
            let iteration = engine
                .process_frame(
                    &frame,
                    loaded_templates.as_deref().unwrap_or(&[]),
                    &scanner_config,
                    stats,
                )
                .map_err(|err| err.to_string())?;
            supervisor.mark_healthy();
            if shutdown.is_requested() {
                let _ = session.stop();
                break Ok(WorkerExit::Stopped);
            }
            apply_iteration(shared, &located, iteration);
        }
    };
    let cleanup_result = if matches!(config.capture.source, CaptureSource::Window) {
        finish_window_capture(
            &config.capture,
            located.window.hwnd,
            restored_from_minimized,
        )
    } else {
        Ok(())
    };
    if let Err(err) = cleanup_result {
        if run_result.is_ok() {
            return Err(err);
        }
    }
    run_result
}

fn try_collect_loaded_templates(
    template_loader: &mut Option<JoinHandle<Result<Vec<Arc<LoadedTemplate>>, String>>>,
) -> Result<Option<Vec<Arc<LoadedTemplate>>>, String> {
    let Some(loader) = template_loader.as_ref() else {
        return Ok(None);
    };
    if !loader.is_finished() {
        return Ok(None);
    }

    let loader = template_loader
        .take()
        .ok_or_else(|| "模板加载线程状态异常".to_string())?;
    loader.join().map_err(panic_payload_to_string)?.map(Some)
}

fn resolve_startup_templates(
    app_paths: &AppPaths,
    config: &AppConfig,
) -> Result<Vec<TemplateRef>, String> {
    if !config.templates.is_empty() {
        return Ok(config.templates.clone());
    }

    let template_repository = TemplateRepository::new(&app_paths.db_path);
    template_repository.list().map_err(|err| err.to_string())
}

fn resolve_located_target(
    supervisor: &mut RuntimeSupervisor,
    config: &AppConfig,
    prefetched_target: Option<LocatorCandidate>,
) -> Result<LocatorCandidate, String> {
    if let Some(target) = prefetched_target {
        return Ok(target);
    }

    supervisor
        .locate_target(&config.target)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "未找到匹配的目标窗口".to_string())
}

fn validate_runtime_config(config: &AppConfig) -> Result<(), String> {
    if matches!(config.capture.source, CaptureSource::Window) {
        match config.target.hwnd {
            Some(hwnd) if hwnd > 0 => {}
            _ => {
                return Err("开始扫描前请先在“目标窗口”中选择一个可捕获窗口".to_string());
            }
        }
    }
    Ok(())
}

fn build_capture_session_config(config: &AppConfig, hwnd: isize) -> CaptureSessionConfig {
    CaptureSessionConfig {
        target: match config.capture.source {
            CaptureSource::Window => CaptureTarget::Window { hwnd },
            CaptureSource::Monitor => CaptureTarget::Monitor { handle: None },
        },
        options: WgcCaptureOptions {
            target_fps: config.capture.target_fps,
            include_cursor: config.capture.include_cursor,
            draw_border: match config.capture.source {
                CaptureSource::Window => config.capture.window_border_required,
                CaptureSource::Monitor => config.capture.screen_border_required,
            },
            include_secondary_windows: false,
            remove_title_bar: false,
            dirty_region_enabled: false,
        },
    }
}

fn build_scanner_config(config: &AppConfig, located: &LocatorCandidate) -> ScannerEngineConfig {
    ScannerEngineConfig {
        roi: config.capture.roi.clone(),
        scales: config.detection.scales.clone(),
        multi_scale: config.detection.multi_scale,
        threshold: config.detection.threshold,
        early_exit: config.detection.early_exit,
        input_policy: config.input.clone(),
        target_hwnd: located.window.hwnd,
        window_rect: located.window.rect.clone(),
        preview: build_preview_config(config),
    }
}

fn build_preview_config(config: &AppConfig) -> PreviewBusConfig {
    PreviewBusConfig {
        enabled: true,
        throttle_ms: if config.ui.debug_mode { 120 } else { 250 },
        ..PreviewBusConfig::default()
    }
}

fn map_recovery_policy(config: &AppConfig) -> CaptureRecoveryPolicy {
    let base_backoff_ms = (config.recovery.recovery_cooldown_secs.max(0.1) * 1000.0) as u64;
    CaptureRecoveryPolicy {
        max_attempts: config.recovery.max_recovery_attempts.max(1),
        base_backoff_ms,
        max_backoff_ms: base_backoff_ms.saturating_mul(4).max(base_backoff_ms),
    }
}

fn apply_iteration(
    shared: &Arc<RwLock<RuntimeControllerSnapshot>>,
    located: &LocatorCandidate,
    iteration: ScanIteration,
) {
    let status = if matches!(iteration.decision, HitDecision::CoolingDown(_)) {
        RuntimeStatus::CoolingDown
    } else {
        RuntimeStatus::Running
    };

    let mut snapshot = shared.write();
    snapshot.status = status;
    snapshot.metrics = iteration.metrics.clone();
    snapshot.metrics.runtime.status = status;
    snapshot.preview = iteration
        .preview
        .clone()
        .or_else(|| snapshot.preview.clone());
    snapshot.active_target = Some(located.clone());
    snapshot.best_match = iteration.pipeline.best_match.clone();
    snapshot.decision = Some(iteration.decision);
    snapshot.last_click = iteration.click_report;
    snapshot.last_error = None;
}

fn apply_starting_preview(
    shared: &Arc<RwLock<RuntimeControllerSnapshot>>,
    located: &LocatorCandidate,
    iteration: PreviewIteration,
) {
    let mut snapshot = shared.write();
    snapshot.status = RuntimeStatus::Starting;
    snapshot.metrics = iteration.metrics;
    snapshot.metrics.runtime.status = RuntimeStatus::Starting;
    snapshot.preview = iteration.preview.or_else(|| snapshot.preview.clone());
    snapshot.active_target = Some(located.clone());
    snapshot.best_match = None;
    snapshot.decision = None;
    snapshot.last_click = None;
    snapshot.last_error = None;
}

fn update_metrics_only(
    shared: &Arc<RwLock<RuntimeControllerSnapshot>>,
    status: RuntimeStatus,
    metrics: RuntimeMetricsSnapshot,
    preview: Option<PreviewMessage>,
) {
    let mut snapshot = shared.write();
    snapshot.status = status;
    snapshot.metrics = metrics;
    snapshot.metrics.runtime.status = status;
    snapshot.preview = preview.or_else(|| snapshot.preview.clone());
}

fn update_target(
    shared: &Arc<RwLock<RuntimeControllerSnapshot>>,
    target: Option<LocatorCandidate>,
) {
    shared.write().active_target = target;
}

fn clear_error(shared: &Arc<RwLock<RuntimeControllerSnapshot>>) {
    shared.write().last_error = None;
}

fn set_status(shared: &Arc<RwLock<RuntimeControllerSnapshot>>, status: RuntimeStatus) {
    let mut snapshot = shared.write();
    snapshot.status = status;
    snapshot.metrics.runtime.status = status;
    if status != RuntimeStatus::Faulted {
        snapshot.last_error = None;
    }
}

fn set_faulted(shared: &Arc<RwLock<RuntimeControllerSnapshot>>, error: String) {
    let mut snapshot = shared.write();
    snapshot.status = RuntimeStatus::Faulted;
    snapshot.metrics.runtime.status = RuntimeStatus::Faulted;
    snapshot.metrics.runtime.last_error = Some(error.clone());
    snapshot.last_error = Some(error);
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use autoclick_domain::{
        config::AppConfig, paths::AppPaths, template::TemplateRef, types::RuntimeStatus,
    };
    use autoclick_platform_win::window::{WindowInfo, WindowRect};
    use autoclick_runtime::{shutdown::ShutdownSignal, state_machine::StateEvent};

    use super::{
        RuntimeController, ScannerWorkerHandle, resolve_located_target, resolve_startup_templates,
        set_status,
    };

    #[test]
    fn start_rejects_missing_window_target() {
        let controller = RuntimeController::default();
        let error = controller
            .start(
                AppPaths::from_base_dir("test-data/autoclick"),
                AppConfig::default(),
                None,
            )
            .expect_err("missing target should be rejected");

        assert!(error.contains("目标窗口"));
        assert_eq!(controller.snapshot().status, RuntimeStatus::Idle);
    }

    #[test]
    fn stop_waits_for_worker_and_returns_idle() {
        let controller = RuntimeController::default();
        let shutdown = ShutdownSignal::default();
        let worker_shutdown = shutdown.clone();
        let join = thread::spawn(move || {
            while !worker_shutdown.is_requested() {
                thread::sleep(Duration::from_millis(5));
            }
            // 模拟退出前的短暂清理
            thread::sleep(Duration::from_millis(50));
        });

        {
            let mut inner = controller.inner.lock();
            inner
                .machine
                .apply(StateEvent::RequestStart)
                .expect("start");
            inner
                .machine
                .apply(StateEvent::CaptureReady)
                .expect("ready");
            inner.worker = Some(ScannerWorkerHandle { shutdown, join });
        }
        set_status(&controller.shared, RuntimeStatus::Running);

        let started_at = Instant::now();
        let snapshot = controller.stop().expect("stop should succeed");

        // stop 现在会等待线程退出（500ms 超时），短线程应直接返回 Idle
        assert_eq!(snapshot.status, RuntimeStatus::Idle);
        assert!(started_at.elapsed() < Duration::from_millis(400));
    }

    #[test]
    fn stop_returns_stopping_when_worker_slow() {
        let controller = RuntimeController::default();
        let shutdown = ShutdownSignal::default();
        let worker_shutdown = shutdown.clone();
        let join = thread::spawn(move || {
            while !worker_shutdown.is_requested() {
                thread::sleep(Duration::from_millis(5));
            }
            // 模拟超长清理
            thread::sleep(Duration::from_millis(2000));
        });

        {
            let mut inner = controller.inner.lock();
            inner
                .machine
                .apply(StateEvent::RequestStart)
                .expect("start");
            inner
                .machine
                .apply(StateEvent::CaptureReady)
                .expect("ready");
            inner.worker = Some(ScannerWorkerHandle { shutdown, join });
        }
        set_status(&controller.shared, RuntimeStatus::Running);

        let started_at = Instant::now();
        let snapshot = controller.stop().expect("stop should succeed");

        // 线程在 500ms 超时内未退出，返回 Stopping
        assert_eq!(snapshot.status, RuntimeStatus::Stopping);
        assert!(started_at.elapsed() < Duration::from_millis(800));
    }

    #[test]
    fn startup_prefers_templates_carried_in_config() {
        let app_paths = AppPaths::from_base_dir("test-data/autoclick-config-templates");
        let mut config = AppConfig::default();
        let mut template = TemplateRef::new("配置模板");
        template.hash = "config-template-hash".to_string();
        config.templates = vec![template.clone()];

        let resolved = resolve_startup_templates(&app_paths, &config).expect("resolve templates");

        assert_eq!(resolved, vec![template]);
    }

    #[test]
    fn startup_reuses_prefetched_target() {
        let target = autoclick_platform_win::locator::LocatorCandidate {
            window: WindowInfo {
                hwnd: 42,
                title: "目标窗口".to_string(),
                class_name: "Chrome_WidgetWin_1".to_string(),
                pid: 100,
                exe_path: Some("apps/Target/Target.exe".to_string()),
                is_minimized: false,
                is_visible: true,
                rect: WindowRect {
                    left: 0,
                    top: 0,
                    right: 1280,
                    bottom: 720,
                },
            },
            reliability: 88,
            reason: "预定位命中".to_string(),
        };

        let resolved = resolve_located_target(
            &mut autoclick_runtime::supervisor::RuntimeSupervisor::new(super::map_recovery_policy(
                &AppConfig::default(),
            )),
            &AppConfig::default(),
            Some(target.clone()),
        )
        .expect("resolve target");

        assert_eq!(resolved, target);
    }
}
