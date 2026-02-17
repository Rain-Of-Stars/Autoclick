# -*- coding: utf-8 -*-
"""
回归测试：性能监控线程在stop_monitoring后应快速退出，避免残留后台线程。
"""

from __future__ import annotations

import time


def test_performance_collector_stop_is_prompt(monkeypatch):
    from auto_approve.performance_monitor import PerformanceCollector

    collector = PerformanceCollector()
    monkeypatch.setattr(collector, "_collect_metrics", lambda: None)

    collector.start_monitoring()
    time.sleep(0.05)

    start = time.monotonic()
    collector.stop_monitoring()
    elapsed = time.monotonic() - start

    assert elapsed < 0.8
    assert collector._thread is None or not collector._thread.is_alive()
