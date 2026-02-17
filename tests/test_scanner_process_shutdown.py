# -*- coding: utf-8 -*-
"""
回归测试：验证扫描进程管理器的同步关停链路。
"""

from __future__ import annotations


class _FakeQueue:
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


class _GracefulProcess:
    """模拟可通过 join 正常退出的子进程。"""

    def __init__(self):
        self.alive = True
        self.terminate_calls = 0
        self.kill_calls = 0
        self.join_calls = 0

    def is_alive(self):
        return self.alive

    def join(self, timeout=None):
        self.join_calls += 1
        self.alive = False

    def terminate(self):
        self.terminate_calls += 1

    def kill(self):
        self.kill_calls += 1
        self.alive = False


class _StuckProcess:
    """模拟无论 join 多少次都不退出，直到 kill 才结束的子进程。"""

    def __init__(self):
        self.alive = True
        self.terminate_calls = 0
        self.kill_calls = 0
        self.join_calls = 0

    def is_alive(self):
        return self.alive

    def join(self, timeout=None):
        self.join_calls += 1

    def terminate(self):
        self.terminate_calls += 1

    def kill(self):
        self.kill_calls += 1
        self.alive = False


def _build_manager_with_ipc(process_obj):
    from workers.scanner_process import ScannerProcessManager

    mgr = ScannerProcessManager()
    mgr._running = True
    mgr._session_token = 11
    mgr._process = process_obj

    cmd_q = _FakeQueue()
    status_q = _FakeQueue()
    hit_q = _FakeQueue()
    log_q = _FakeQueue()

    mgr._command_queue = cmd_q
    mgr._status_queue = status_q
    mgr._hit_queue = hit_q
    mgr._log_queue = log_q

    return mgr, cmd_q, status_q, hit_q, log_q


def test_shutdown_graceful_exit_and_cleanup(monkeypatch):
    from workers import scanner_process as scanner_process_module

    mgr, cmd_q, status_q, hit_q, log_q = _build_manager_with_ipc(_GracefulProcess())
    monkeypatch.setattr(scanner_process_module.time, "sleep", lambda _s: None)

    assert mgr.shutdown(timeout_s=2.0) is True

    assert [cmd.command for cmd in cmd_q.items] == ["stop", "exit"]
    assert cmd_q.closed and cmd_q.joined
    assert status_q.closed and status_q.joined
    assert hit_q.closed and hit_q.joined
    assert log_q.closed and log_q.joined
    assert mgr._running is False
    assert mgr._process is None
    assert mgr._command_queue is None


def test_shutdown_force_kill_when_process_stuck(monkeypatch):
    from workers import scanner_process as scanner_process_module

    stuck_proc = _StuckProcess()
    mgr, cmd_q, status_q, hit_q, log_q = _build_manager_with_ipc(stuck_proc)
    monkeypatch.setattr(scanner_process_module.time, "sleep", lambda _s: None)

    assert mgr.shutdown(timeout_s=0.6) is True

    assert [cmd.command for cmd in cmd_q.items] == ["stop", "exit"]
    assert stuck_proc.terminate_calls >= 1
    assert stuck_proc.kill_calls >= 1
    assert cmd_q.closed and cmd_q.joined
    assert status_q.closed and status_q.joined
    assert hit_q.closed and hit_q.joined
    assert log_q.closed and log_q.joined
