# -*- coding: utf-8 -*-
"""
回归测试：全局缓存管理器在cleanup_all后不应重启清理定时器。
"""

from __future__ import annotations


def test_cleanup_all_blocks_timer_reschedule(monkeypatch):
    from capture import cache_manager as cache_module

    created_timers = []

    class _FakeTimer:
        def __init__(self, _interval, callback):
            self.callback = callback
            self.daemon = False
            self.cancelled = False
            self.started = False
            created_timers.append(self)

        def start(self):
            self.started = True

        def cancel(self):
            self.cancelled = True

    monkeypatch.setattr(cache_module.threading, "Timer", _FakeTimer)

    manager = cache_module.GlobalCacheManager()
    assert manager._cleanup_timer is created_timers[-1]

    # 模拟清理线程执行中触发退出清理：若无修复，finally会再次创建新定时器。
    monkeypatch.setattr(manager, "cleanup_expired_sessions", lambda: manager.cleanup_all())
    created_timers[0].callback()

    assert manager._shutdown is True
    assert manager._cleanup_timer is None
    assert len(created_timers) == 1
