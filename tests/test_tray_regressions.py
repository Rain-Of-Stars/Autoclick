# -*- coding: utf-8 -*-
"""
托盘回归测试：
1) 托盘状态短格式可更新（含“已停止”）
2) stop_scanning 会停止启动相关定时器
3) _handle_menu_update 能落地更新关键字段
4) 菜单包含“屏幕列表…”入口
5) 图标回退路径可得到非空图标
"""

import os
import threading
import time
from types import SimpleNamespace

import pytest

# 无头环境下运行Qt测试，避免交互依赖
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

from PySide6 import QtCore, QtGui, QtWidgets

import main_auto_approve_refactored as tray_module
from auto_approve.auto_hwnd_updater import AutoHWNDUpdater
from auto_approve.gui_responsiveness_manager import UIUpdateRequest


class _DummyLogger:
    """测试用空日志器，避免真实日志副作用。"""

    def debug(self, *_args, **_kwargs):
        pass

    def info(self, *_args, **_kwargs):
        pass

    def warning(self, *_args, **_kwargs):
        pass

    def error(self, *_args, **_kwargs):
        pass


class _TrayHarness(tray_module.RefactoredTrayApp):
    """轻量托盘实例：仅初始化本测试用到的字段。"""

    def __init__(self, app: QtWidgets.QApplication):
        QtWidgets.QSystemTrayIcon.__init__(self, QtGui.QIcon())
        self.app = app
        self.logger = _DummyLogger()
        self.state = SimpleNamespace(enable_logging=False)
        self.worker = None
        self.settings_dlg = None

        self._cached_status = ""
        self._cached_backend = ""
        self._cached_detail = ""
        self._last_tooltip_update = 0.0
        self._tooltip_update_interval = 0.0
        self._last_action_update = 0.0
        self._action_update_interval = 0.0
        self._notification_icon_cache = {}
        self._cpu_manager_started = False
        self.async_loop = None

        self._progress_counter = 0
        self._start_timeout_timer = QtCore.QTimer(self)
        self._start_timeout_timer.setSingleShot(True)
        self._progress_timer = QtCore.QTimer(self)
        self._quit_in_progress = False
        self._quit_cleanup_done = False
        self._quit_watchdog_ms = 300
        self._quit_cleanup_thread = None


@pytest.fixture(scope="module")
def qapp():
    app = QtWidgets.QApplication.instance()
    if app is None:
        app = QtWidgets.QApplication([])
    return app


@pytest.fixture
def tray_app(qapp, monkeypatch):
    monkeypatch.setattr(
        "auto_approve.gui_responsiveness_manager.schedule_ui_update",
        lambda *_args, **_kwargs: None,
    )
    monkeypatch.setattr(tray_module, "record_ui_update", lambda: None)

    app = _TrayHarness(qapp)
    app._create_menu()
    return app


@pytest.mark.parametrize("short_status", ["stopped", "已停止"])
def test_tray_status_short_format_updates_stopped(tray_app, short_status):
    request = UIUpdateRequest(
        widget_id="scanner",
        update_type="status",
        data={"status": short_status},
    )

    tray_app._handle_status_update(request)

    assert tray_app.act_status.text() == "状态: 已停止"
    assert tray_app.act_backend.text() == "后端: -"


def test_stop_scanning_stops_startup_related_timers(tray_app):
    class _FakeWorker:
        def __init__(self):
            self._running = True
            self.stop_called = False
            self.cleanup_called = False
            self.terminate_called = False

        def isRunning(self):
            return self._running

        def stop(self):
            self.stop_called = True
            self._running = False

        def wait(self, _ms):
            return None

        def terminate(self):
            self.terminate_called = True
            self._running = False

        def cleanup(self):
            self.cleanup_called = True

    worker = _FakeWorker()
    tray_app.worker = worker
    tray_app._progress_counter = 7

    tray_app._start_timeout_timer.start(10000)
    tray_app._progress_timer.start(10000)
    assert tray_app._start_timeout_timer.isActive()
    assert tray_app._progress_timer.isActive()

    tray_app.stop_scanning()

    assert not tray_app._start_timeout_timer.isActive()
    assert not tray_app._progress_timer.isActive()
    assert tray_app._progress_counter == 0
    assert worker.stop_called is True
    assert worker.cleanup_called is True
    assert worker.terminate_called is False
    assert tray_app.worker is None


def test_handle_menu_update_applies_critical_fields(tray_app, monkeypatch):
    updates = []
    monkeypatch.setattr(tray_module, "record_ui_update", lambda: updates.append("ok"))

    request = UIUpdateRequest(
        widget_id="tray_menu",
        update_type="menu",
        data={
            "status": "running | 后端: 线程 | 详情: 捕获中",
            "logging_checked": True,
            "running": True,
            "tooltip": "Autoclick - 运行中",
        },
    )

    tray_app._handle_menu_update(request)

    assert tray_app.act_status.text() == "状态: 运行中"
    assert tray_app.act_backend.text() == "后端: 线程"
    assert tray_app.act_detail.text() == "捕获中"
    assert tray_app.act_logging.isChecked() is True
    assert tray_app.act_start.isChecked() is True
    assert tray_app.act_stop.isChecked() is False
    assert tray_app.toolTip() == "Autoclick - 运行中"
    assert updates


def test_menu_contains_screen_list_entry(tray_app):
    action_texts = [action.text() for action in tray_app.menu.actions()]
    assert "屏幕列表…" in action_texts
    assert tray_app.act_screen_list.text() == "屏幕列表…"


def test_apply_menu_icons_fallback_returns_non_null_icon(tray_app, monkeypatch):
    called_types = []

    def _fake_create_menu_icon(icon_type: str, size: int = 16):
        called_types.append(icon_type)
        pixmap = QtGui.QPixmap(size, size)
        pixmap.fill(QtGui.QColor("#2f95ff"))
        return QtGui.QIcon(pixmap)

    monkeypatch.setattr(tray_module, "create_menu_icon", _fake_create_menu_icon)

    tray_app._apply_menu_icons({})

    expected_types = {"play", "stop", "settings", "log", "screen", "quit"}
    assert expected_types.issubset(set(called_types))
    assert not tray_app.act_screen_list.icon().isNull()
    assert not tray_app.act_start.icon().isNull()


def test_cleanup_threading_resources_uses_blocking_scanner_cleanup(tray_app, monkeypatch):
    calls = {"io": 0, "scanner": []}

    class _FakeScannerManager:
        def cleanup(self, *args, **kwargs):
            calls["scanner"].append((args, kwargs))

    fake_manager = _FakeScannerManager()

    monkeypatch.setattr(
        "workers.scanner_process.get_global_scanner_manager",
        lambda: fake_manager,
    )
    monkeypatch.setattr(
        "workers.io_tasks.cleanup_thread_pool",
        lambda: calls.__setitem__("io", calls["io"] + 1),
    )

    tray_app._cleanup_threading_resources()

    assert calls["io"] == 1
    assert len(calls["scanner"]) == 1
    args, kwargs = calls["scanner"][0]
    assert args == ()
    assert kwargs == {"blocking": True, "timeout_s": 8.0}


def test_quit_returns_quickly_and_runs_cleanup_in_background_thread(tray_app, monkeypatch):
    cleanup_started = threading.Event()
    cleanup_release = threading.Event()
    cleanup_done = threading.Event()
    cleanup_thread_ids = []

    def _fake_cleanup():
        cleanup_thread_ids.append(threading.get_ident())
        cleanup_started.set()
        cleanup_release.wait(timeout=0.6)
        cleanup_done.set()

    tray_app.auto_hwnd_updater = SimpleNamespace(is_running=lambda: False)
    tray_app.worker = None
    monkeypatch.setattr(tray_app, "_request_app_quit", lambda: None)
    monkeypatch.setattr(tray_app, "_cleanup_threading_resources", _fake_cleanup)

    main_thread_id = threading.get_ident()
    start = time.perf_counter()
    tray_app.quit()
    elapsed = time.perf_counter() - start

    assert elapsed < 0.2
    assert cleanup_started.wait(timeout=0.3)
    assert cleanup_thread_ids and cleanup_thread_ids[0] != main_thread_id

    cleanup_release.set()
    assert cleanup_done.wait(timeout=0.3)


def test_quit_still_calls_scanner_cleanup_with_expected_parameters(tray_app, monkeypatch):
    scanner_cleanup_called = threading.Event()
    scanner_calls = []

    class _FakeScannerManager:
        def cleanup(self, *args, **kwargs):
            scanner_calls.append((args, kwargs, threading.get_ident()))
            scanner_cleanup_called.set()

    tray_app.auto_hwnd_updater = SimpleNamespace(is_running=lambda: False)
    tray_app.worker = None
    monkeypatch.setattr(tray_app, "_request_app_quit", lambda: None)

    monkeypatch.setattr(
        "workers.scanner_process.get_global_scanner_manager",
        lambda: _FakeScannerManager(),
    )
    monkeypatch.setattr("workers.io_tasks.cleanup_thread_pool", lambda: None)

    main_thread_id = threading.get_ident()
    start = time.perf_counter()
    tray_app.quit()
    elapsed = time.perf_counter() - start

    assert elapsed < 0.2
    assert scanner_cleanup_called.wait(timeout=0.6)

    args, kwargs, cleanup_thread_id = scanner_calls[0]
    assert args == ()
    assert kwargs == {"blocking": True, "timeout_s": 8.0}
    assert cleanup_thread_id != main_thread_id


def test_auto_hwnd_updater_stop_calls_smart_finder_outside_lock():
    updater = AutoHWNDUpdater()
    updater._lock = threading.Lock()

    state = {"lock_free": None, "timer_cancelled": False}

    class _FakeSmartFinder:
        def stop_smart_search(self):
            # 若这里能拿到锁，说明stop_smart_search在锁外执行
            acquired = updater._lock.acquire(blocking=False)
            state["lock_free"] = acquired
            if acquired:
                updater._lock.release()

    class _FakeTimer:
        def cancel(self):
            state["timer_cancelled"] = True

    updater._smart_finder = _FakeSmartFinder()
    updater._update_timer = _FakeTimer()
    updater._running = True

    updater.stop()

    assert state["lock_free"] is True
    assert state["timer_cancelled"] is True
    assert updater.is_running() is False
