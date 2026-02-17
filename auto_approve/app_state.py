# -*- coding: utf-8 -*-
"""
应用共享状态中心：
- 统一维护“启用日志”状态，提供Qt信号用于跨模块实时同步；
- 集中处理文件日志启用与配置持久化，避免不同入口重复实现。

使用方式：
from auto_approve.app_state import get_app_state
state = get_app_state()
state.set_enable_logging(True)  # 将立即启用文件日志，并异步合并持久化到配置
state.loggingChanged.connect(slot)  # 订阅状态变化
"""
from __future__ import annotations

import threading
import time

from PySide6 import QtCore

from auto_approve.logger_manager import enable_file_logging
from storage import init_db, get_config_json, set_config_enable_logging


class AppState(QtCore.QObject):
    """应用级共享状态对象（单例）。"""

    # 日志开关变化信号
    loggingChanged = QtCore.Signal(bool)

    def __init__(self):
        super().__init__()
        # 线程锁：保护共享状态和异步持久化调度变量
        self._lock = threading.RLock()

        # 异步持久化状态（懒启动后台线程）
        self._persist_cv = threading.Condition(self._lock)
        self._persist_thread: threading.Thread | None = None
        self._persist_debounce_s = 0.15  # 合并窗口：快速连点时仅落盘最后一次
        self._persist_version = 0
        self._persist_flushed_version = 0
        self._persist_target = False

        # 初始状态直接从存储层读取，避免构建完整配置对象
        try:
            init_db()
            cfg = get_config_json() or {}
            self._enable_logging = bool(cfg.get("enable_logging", False)) if isinstance(cfg, dict) else False
            # 确保启动时文件日志状态与配置一致
            enable_file_logging(self._enable_logging)
        except Exception:
            self._enable_logging = False

        self._persist_target = self._enable_logging

    # 只读属性，便于外部快速读取当前状态
    @property
    def enable_logging(self) -> bool:
        with self._lock:
            return bool(self._enable_logging)

    def _start_persist_worker_locked(self) -> None:
        """按需启动异步持久化线程（调用方需持有 self._lock）。"""
        if self._persist_thread is not None and self._persist_thread.is_alive():
            return
        self._persist_thread = threading.Thread(
            target=self._persist_worker,
            name="app-state-persist",
            daemon=True,
        )
        self._persist_thread.start()

    def _schedule_persist(self, value: bool) -> None:
        """异步调度落盘；连续更新会在短窗口内自动合并。"""
        with self._persist_cv:
            self._persist_target = bool(value)
            self._persist_version += 1
            self._start_persist_worker_locked()
            self._persist_cv.notify()

    def _persist_worker(self) -> None:
        """后台持久化线程：去抖动+合并更新，避免UI线程阻塞。"""
        while True:
            with self._persist_cv:
                # 等待新的持久化请求
                while self._persist_version == self._persist_flushed_version:
                    self._persist_cv.wait()

                target_version = self._persist_version
                # 合并窗口：在静默期结束后再落盘，避免快速连点频繁写盘
                deadline = time.monotonic() + self._persist_debounce_s
                while True:
                    wait_s = deadline - time.monotonic()
                    if wait_s <= 0:
                        break
                    self._persist_cv.wait(timeout=wait_s)
                    if self._persist_version != target_version:
                        target_version = self._persist_version
                        deadline = time.monotonic() + self._persist_debounce_s

                target_value = bool(self._persist_target)

            # 配置IO放到锁外执行，避免阻塞调用线程
            try:
                # 仅更新enable_logging字段，避免全量load/save链路开销
                set_config_enable_logging(target_value)
            except Exception:
                pass

            with self._persist_cv:
                # 若写入期间没有新请求，则标记为已落盘；否则继续下一轮合并
                if self._persist_version == target_version:
                    self._persist_flushed_version = target_version

    def set_enable_logging(self, value: bool, *, persist: bool = True, emit_signal: bool = True) -> None:
        """设置“启用日志”状态。
        参数：
        - value: 目标布尔值
        - persist: 是否写入配置文件（默认True）
        - emit_signal: 是否发出变化信号（默认True）
        说明：
        - 无论是否持久化，都会立即调用 enable_file_logging 以使日志开关即时生效。
        - 当 persist=True 时，配置写入会异步执行并自动合并连续请求，避免频繁阻塞UI线程。
        - 若新旧状态一致且无需强制持久化/发信号，则仅确保 enable_file_logging 一致。
        """
        value = bool(value)
        with self._lock:
            changed = (value != self._enable_logging)
            self._enable_logging = value
            # 立即应用文件日志开关，确保并发下日志状态与内存状态一致
            try:
                enable_file_logging(value)
            except Exception:
                pass

        # 可选持久化到配置
        if persist:
            try:
                # 改为异步+可合并落盘，避免UI线程频繁阻塞
                self._schedule_persist(value)
            except Exception:
                pass

        # 仅在需要时发信号（避免不必要的环路）
        if emit_signal and changed:
            try:
                self.loggingChanged.emit(value)
            except Exception:
                pass


# 单例持有者
__app_state_singleton: AppState | None = None


def get_app_state() -> AppState:
    """获取全局共享状态单例。"""
    global __app_state_singleton
    if __app_state_singleton is None:
        __app_state_singleton = AppState()
    return __app_state_singleton
