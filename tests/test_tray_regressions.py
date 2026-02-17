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
from types import SimpleNamespace

import pytest

# 无头环境下运行Qt测试，避免交互依赖
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

from PySide6 import QtCore, QtGui, QtWidgets

import main_auto_approve_refactored as tray_module
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
