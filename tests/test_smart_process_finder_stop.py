# -*- coding: utf-8 -*-
"""
回归测试：SmartProcessFinder 停止路径不应被长间隔sleep阻塞。
"""

from __future__ import annotations

import time


def test_stop_smart_search_can_interrupt_long_interval():
    """停止应能快速打断长等待，避免join超时后遗留后台线程。"""
    from auto_approve.smart_process_finder import SmartProcessFinder

    finder = SmartProcessFinder()
    finder._event_hook_enabled = False
    finder._adaptive_interval = 8.0
    finder._base_interval = 8.0

    finder.start_smart_search()

    # 给后台线程一个极短启动窗口
    deadline = time.monotonic() + 0.3
    while time.monotonic() < deadline:
        t = finder._search_thread
        if t and t.is_alive():
            break
        time.sleep(0.01)

    start = time.monotonic()
    finder.stop_smart_search()
    elapsed = time.monotonic() - start

    assert elapsed < 1.0
    assert finder._search_thread is None or not finder._search_thread.is_alive()
