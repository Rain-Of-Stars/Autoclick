# -*- coding: utf-8 -*-
"""
IO线程池配置回归测试：验证PySide6线程池隔离与基础参数稳定。
"""

from __future__ import annotations

from PySide6.QtCore import QThreadPool


def test_io_pool_isolated_from_qt_global_pool():
    """IO线程池应使用专用实例，避免影响Qt全局线程池。"""
    from workers import io_tasks

    io_tasks.cleanup_thread_pool()
    pool = io_tasks.get_global_thread_pool()

    assert pool is not QThreadPool.globalInstance()
    assert pool.maxThreadCount() >= 1

    io_tasks.cleanup_thread_pool()


def test_optimize_thread_pool_preserves_safe_runtime_defaults():
    """线程池优化后应保持可回收策略和有效线程数。"""
    from workers import io_tasks

    io_tasks.cleanup_thread_pool()

    result = io_tasks.optimize_thread_pool(cpu_intensive_ratio=0.2, gui_priority=True)
    stats = io_tasks.get_thread_pool_stats()

    assert result["max_threads"] == stats["max_thread_count"]
    assert stats["expiry_timeout"] == 15000
    assert stats["max_thread_count"] >= 2

    io_tasks.cleanup_thread_pool()
