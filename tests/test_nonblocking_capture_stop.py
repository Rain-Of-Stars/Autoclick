# -*- coding: utf-8 -*-
"""
回归测试：NonBlockingCaptureManager 停止路径应覆盖“启动中”场景。
"""

from __future__ import annotations

import threading
import types

from capture.NonBlockingCaptureManager import NonBlockingCaptureManager


class _DummyLogger:
    def info(self, *_args, **_kwargs):
        return None


def test_stop_capture_should_handle_pending_start():
    """当会话仍处于启动中时，stop_capture也应投递停止请求。"""
    emitted = []

    manager = types.SimpleNamespace(
        _is_capturing=False,
        _start_pending=True,
        _request_timer=types.SimpleNamespace(isActive=lambda: False),
        _stop_ack_event=threading.Event(),
        _request_stop_capture=types.SimpleNamespace(emit=lambda: emitted.append("stop")),
        _current_target="demo",
        logger=_DummyLogger(),
    )

    NonBlockingCaptureManager.stop_capture(manager)

    assert emitted == ["stop"]
    assert manager._start_pending is False
    assert manager._current_target is None


def test_on_capture_stopped_should_release_state():
    """工作线程确认停止后，应释放状态并发出外部停止信号。"""
    emitted = []
    stop_event = threading.Event()

    manager = types.SimpleNamespace(
        _start_pending=True,
        _is_capturing=True,
        _current_target="demo",
        _request_timer=types.SimpleNamespace(isActive=lambda: False),
        _stop_ack_event=stop_event,
        capture_stopped=types.SimpleNamespace(emit=lambda: emitted.append("done")),
    )

    NonBlockingCaptureManager._on_capture_stopped(manager)

    assert manager._start_pending is False
    assert manager._is_capturing is False
    assert manager._current_target is None
    assert stop_event.is_set()
    assert emitted == ["done"]
