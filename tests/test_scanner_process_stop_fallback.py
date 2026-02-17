# -*- coding: utf-8 -*-
"""
回归测试：扫描进程在无Qt事件循环场景下仍能可靠停止并释放资源。
"""

from __future__ import annotations

import types


def test_stop_scanning_fallback_timer_can_cleanup(monkeypatch):
    """singleShot不触发时，应由线程兜底定时器完成stop->exit->cleanup链路。"""
    from PySide6 import QtCore
    from workers.scanner_process import ScannerProcessManager

    class FakeQueue:
        def __init__(self):
            self.items = []
            self.closed = False
            self.joined = False

        def put(self, item):
            self.items.append(item)

        def close(self):
            self.closed = True

        def join_thread(self):
            self.joined = True

    class ImmediateTimer:
        """同步执行的假定时器，模拟兜底回调立即触发。"""

        def __init__(self, _delay, callback):
            self._callback = callback
            self._cancelled = False
            self.daemon = True

        def start(self):
            if not self._cancelled:
                self._callback()

        def cancel(self):
            self._cancelled = True

    mgr = ScannerProcessManager()
    mgr._running = True
    mgr._session_token = 9

    command_queue = FakeQueue()
    status_queue = FakeQueue()
    hit_queue = FakeQueue()
    log_queue = FakeQueue()

    mgr._command_queue = command_queue
    mgr._status_queue = status_queue
    mgr._hit_queue = hit_queue
    mgr._log_queue = log_queue
    mgr._process = types.SimpleNamespace(is_alive=lambda: False)

    # 模拟无事件循环：Qt定时器回调永不执行
    monkeypatch.setattr(QtCore.QTimer, "singleShot", staticmethod(lambda _ms, _fn: None))
    # 线程兜底定时器立即执行，确保测试快速稳定
    monkeypatch.setattr("workers.scanner_process.threading.Timer", ImmediateTimer)

    assert mgr.stop_scanning() is True

    assert mgr._running is False
    assert mgr._command_queue is None
    assert mgr._status_queue is None
    assert mgr._hit_queue is None
    assert mgr._log_queue is None

    assert [cmd.command for cmd in command_queue.items] == ["stop", "exit"]
    assert command_queue.closed and command_queue.joined
    assert status_queue.closed and status_queue.joined
    assert hit_queue.closed and hit_queue.joined
    assert log_queue.closed and log_queue.joined


def test_send_exit_command_skip_stale_session(monkeypatch):
    """过期会话的延迟回调不应影响当前会话。"""
    from workers.scanner_process import ScannerProcessManager

    class FakeQueue:
        def __init__(self):
            self.items = []

        def put(self, item):
            self.items.append(item)

    mgr = ScannerProcessManager()
    mgr._running = True
    mgr._session_token = 3
    command_queue = FakeQueue()
    mgr._command_queue = command_queue

    scheduled = {"count": 0}

    def _fake_schedule(_delay_ms, _callback, _task_name):
        scheduled["count"] += 1

    monkeypatch.setattr(mgr, "_schedule_with_fallback", _fake_schedule)

    mgr._send_exit_command(expected_token=2)

    assert command_queue.items == []
    assert scheduled["count"] == 0
