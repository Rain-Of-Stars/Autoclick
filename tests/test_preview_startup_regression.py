# -*- coding: utf-8 -*-
"""
预览启动链路回归测试。

覆盖目标：
- 单帧预览会话参数不再使用过长更新间隔；
- 首帧等待在 start() 阻塞时仍能按超时快速返回；
- 预览对话框启动流程改为后台线程执行；
- 智能捕获管理器预览路径与标准监视器路径可正常工作。
"""
from __future__ import annotations

import sys
import time
import types

import numpy as np
from PySide6 import QtCore

from auto_approve.smart_capture_test_manager import SmartCaptureTestManager
from auto_approve.wgc_preview_dialog import WGCPreviewDialog
from capture.NonBlockingCaptureManager import NonBlockingCaptureManager
from capture.capture_manager import CaptureManager
from capture.single_frame_preview import SingleFramePreview


class _DummyLogger:
    """简化日志对象，避免测试依赖真实日志器。"""

    def debug(self, *_args, **_kwargs):
        return None

    def info(self, *_args, **_kwargs):
        return None

    def warning(self, *_args, **_kwargs):
        return None

    def error(self, *_args, **_kwargs):
        return None


class _DummyButton:
    """按钮桩对象。"""

    def __init__(self, enabled: bool = True):
        self.enabled = enabled

    def setEnabled(self, value: bool):
        self.enabled = bool(value)


class _DummyLabel:
    """文本桩对象。"""

    def __init__(self):
        self.text = ""

    def setText(self, value: str):
        self.text = str(value)


def test_create_minimal_session_use_low_interval(monkeypatch):
    """单帧预览应使用低更新间隔，避免首帧被1s节流。"""
    captured_kwargs = {}

    class _FakeWindowsCapture:
        def __init__(self, **kwargs):
            captured_kwargs.update(kwargs)

    monkeypatch.setitem(
        sys.modules,
        "windows_capture",
        types.SimpleNamespace(WindowsCapture=_FakeWindowsCapture),
    )

    previewer = SingleFramePreview.__new__(SingleFramePreview)
    previewer._logger = _DummyLogger()
    previewer._get_window_title = lambda _hwnd: "demo-window"

    session = SingleFramePreview._create_minimal_session(previewer, 123)

    assert isinstance(session, _FakeWindowsCapture)
    assert captured_kwargs["minimum_update_interval"] == 16


def test_wait_for_first_frame_timeout_not_blocked_by_start():
    """当start阻塞时，等待逻辑应在timeout后尽快返回。"""

    class _FakeSession:
        def __init__(self):
            self.frame_handler = None
            self.closed_handler = None
            self.stop_count = 0

        def start(self):
            time.sleep(0.25)

        def stop(self):
            self.stop_count += 1

    previewer = SingleFramePreview.__new__(SingleFramePreview)
    previewer._logger = _DummyLogger()
    previewer._session = _FakeSession()
    previewer._extract_frame = lambda _frame: None

    start = time.monotonic()
    frame = SingleFramePreview._wait_for_first_frame(previewer, 0.05)
    elapsed = time.monotonic() - start

    assert frame is None
    assert elapsed < 0.15
    assert previewer._session.stop_count >= 1


def test_preview_dialog_start_runs_open_window_in_background(monkeypatch):
    """开始预览时应提交后台线程，而不是在主线程直接open_window。"""
    thread_calls = {"started": False}

    class _DummyCaptureManager:
        def __init__(self):
            self.open_called = False

        def open_window(self, *_args, **_kwargs):
            self.open_called = True
            return True

    class _FakeThread:
        def __init__(self, target, args=(), daemon=None, name=None):
            thread_calls["target"] = target
            thread_calls["args"] = args
            thread_calls["daemon"] = daemon
            thread_calls["name"] = name

        def start(self):
            thread_calls["started"] = True

    monkeypatch.setattr("auto_approve.wgc_preview_dialog.threading.Thread", _FakeThread)

    dialog = WGCPreviewDialog.__new__(WGCPreviewDialog)
    dialog.capture_manager = _DummyCaptureManager()
    dialog.hwnd = 9527
    dialog.is_capturing = False
    dialog._is_starting = False
    dialog._start_request_seq = 0
    dialog._active_start_request_id = None
    dialog._waiting_first_frame = False
    dialog._capture_fail_count = 0
    dialog._exception_count = 0
    dialog.current_frame = object()
    dialog.start_btn = _DummyButton(True)
    dialog.stop_btn = _DummyButton(True)
    dialog.save_btn = _DummyButton(True)
    dialog.preview_label = _DummyLabel()
    dialog.status_label = _DummyLabel()

    WGCPreviewDialog._start_preview(dialog)

    assert thread_calls["started"] is True
    assert thread_calls["args"] == (1,)
    assert dialog.capture_manager.open_called is False
    assert dialog._is_starting is True
    assert dialog._active_start_request_id == 1
    assert dialog.start_btn.enabled is False
    assert "已提交后台任务" in dialog.status_label.text


def test_preview_dialog_ignore_stale_callback_without_closing_new_start():
    """旧请求回调到达时，不应关闭正在启动中的新会话。"""

    class _DummyCaptureManager:
        def __init__(self):
            self.close_count = 0

        def close(self):
            self.close_count += 1

    dialog = WGCPreviewDialog.__new__(WGCPreviewDialog)
    dialog.capture_manager = _DummyCaptureManager()
    dialog._active_start_request_id = 2
    dialog._is_starting = True
    dialog.is_capturing = False

    WGCPreviewDialog._on_startup_result(dialog, 1, True, "stale", 12.0)

    assert dialog.capture_manager.close_count == 0


def test_smart_manager_preview_only_window_emits_runtime_result(monkeypatch):
    """preview_only窗口路径应走运行时单帧实现并成功回传结果。"""
    manager = SmartCaptureTestManager()
    completed = []
    manager.test_completed.connect(completed.append)

    class _FakePreviewer:
        def capture_single_frame(self, hwnd: int, timeout_sec: float):
            assert hwnd == 101
            assert timeout_sec == 0.8
            return np.zeros((2, 3, 3), dtype=np.uint8)

    def _fake_submit_io(task, on_success=None, on_error=None, on_progress=None):
        _ = on_progress
        result = task.execute()
        on_success(task.task_id, result)
        return task.task_id

    monkeypatch.setattr(
        "capture.single_frame_preview.get_single_frame_preview",
        lambda: _FakePreviewer(),
    )
    monkeypatch.setattr("workers.io_tasks.submit_io", _fake_submit_io)

    manager.start_smart_window_test(101, use_fast=False, preview_only=True)

    assert len(completed) == 1
    assert completed[0].success is True
    assert completed[0].metadata["preview_mode"] == "single_frame_runtime"
    assert manager.is_test_active() is False


def test_smart_manager_monitor_standard_branch_has_non_blocking_import(monkeypatch):
    """标准监视器路径应能正确导入并调用NonBlockingCaptureTest。"""
    manager = SmartCaptureTestManager()
    captured = {}

    class _FakeNonBlockingCaptureTest(QtCore.QObject):
        progress_updated = QtCore.Signal(int, str)
        test_completed = QtCore.Signal(object)
        test_failed = QtCore.Signal(str)

        def __init__(self, parent=None):
            super().__init__(parent)

        def test_monitor_capture_async(self, monitor_index: int, timeout_sec: float):
            captured["monitor_index"] = monitor_index
            captured["timeout_sec"] = timeout_sec

    monkeypatch.setitem(
        sys.modules,
        "tests.test_non_blocking_capture",
        types.SimpleNamespace(NonBlockingCaptureTest=_FakeNonBlockingCaptureTest),
    )

    manager.start_smart_monitor_test(2, use_fast=False, preview_only=False)

    assert isinstance(manager._current_tester, _FakeNonBlockingCaptureTest)
    assert captured["monitor_index"] == 2
    assert captured["timeout_sec"] == 1.5


def test_nonblocking_start_window_capture_emit_start_request():
    """非阻塞管理器应通过请求信号启动会话，避免主线程直接执行。"""
    emitted = []

    class _Emitter:
        def emit(self, *args):
            emitted.append(args)

    manager = types.SimpleNamespace(
        _is_capturing=False,
        _stats={},
        _request_start_capture=_Emitter(),
        logger=_DummyLogger(),
        stop_capture=lambda: None,
    )

    ok = NonBlockingCaptureManager.start_window_capture(manager, 9527, partial_match=False)

    assert ok is True
    assert emitted == [(9527, False)]


def test_nonblocking_request_frame_emit_frame_request():
    """请求帧时应发送排队信号，而不是直接在UI线程抓帧。"""
    emitted = []
    manager = types.SimpleNamespace(
        _is_capturing=True,
        _request_capture_frame=types.SimpleNamespace(emit=lambda: emitted.append(True)),
    )

    NonBlockingCaptureManager._request_frame(manager)

    assert emitted == [True]


def test_open_window_limits_close_timeout_for_restart(monkeypatch):
    """窗口重启时应缩短旧会话等待，降低预览重启延迟。"""
    manager = CaptureManager.__new__(CaptureManager)
    manager._logger = _DummyLogger()
    manager._session = object()
    manager._target_hwnd = None
    manager._target_hmonitor = None
    manager._target_fps = 30
    manager._include_cursor = False
    manager._border_required = False
    manager._resolve_window_target = lambda _target, _partial: 100
    manager._get_window_title = lambda _hwnd: "demo"

    close_timeouts = []

    def _fake_close(join_timeout: float = 1.0):
        close_timeouts.append(join_timeout)
        manager._session = None

    manager.close = _fake_close

    fake_session = types.SimpleNamespace(
        start=lambda **_kwargs: True,
        wait_for_frame=lambda _timeout: None,
    )

    monkeypatch.setattr(
        "capture.capture_manager.user32",
        types.SimpleNamespace(IsWindow=lambda _hwnd: True),
    )
    monkeypatch.setattr(
        "capture.capture_manager.WGCCaptureSession",
        types.SimpleNamespace(from_hwnd=lambda _hwnd: fake_session),
    )

    ok = CaptureManager.open_window(manager, 100, async_init=True, timeout=0.8)

    assert ok is True
    assert len(close_timeouts) == 1
    assert 0.05 <= close_timeouts[0] <= 0.2


def test_capture_manager_close_support_legacy_and_new_signature():
    """关闭会话时应透传join_timeout，并兼容旧版close签名。"""
    manager = CaptureManager.__new__(CaptureManager)
    manager._logger = _DummyLogger()
    manager._restore_window_state = lambda: None
    manager._target_hmonitor = None
    manager._was_minimized = False

    calls = {}

    class _NewSession:
        def close(self, join_timeout=1.0):
            calls["join_timeout"] = join_timeout

    manager._session = _NewSession()
    manager._target_hwnd = 123

    CaptureManager.close(manager, join_timeout=0.12)

    assert calls["join_timeout"] == 0.12
    assert manager._session is None
    assert manager._target_hwnd is None
    assert manager._target_hmonitor is None

    class _LegacySession:
        def __init__(self):
            self.closed = False

        def close(self):
            self.closed = True

    legacy = _LegacySession()
    manager._session = legacy
    manager._target_hwnd = 456
    manager._target_hmonitor = None
    manager._was_minimized = False

    CaptureManager.close(manager, join_timeout=0.12)

    assert legacy.closed is True
    assert manager._session is None
