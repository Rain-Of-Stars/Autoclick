# -*- coding: utf-8 -*-
"""
日志管理模块：按配置开关将日志输出到 log.txt，格式包含时间戳。
提供动态开启/关闭文件日志的能力。
"""
import atexit
import logging
import os
import queue
import threading
from logging.handlers import QueueHandler, QueueListener
from typing import Optional

_LOGGER_NAME = "auto_approver"
_LOG_FILE = "log.txt"

_logger = logging.getLogger(_LOGGER_NAME)
_logger.setLevel(logging.INFO)

# 控制台仅在调试时使用，这里默认关闭以减少干扰
if not _logger.handlers:
    _logger.addHandler(logging.NullHandler())

_file_handler: Optional[logging.Handler] = None
_queue_handler: Optional[QueueHandler] = None
_queue_listener: Optional[QueueListener] = None
_log_queue: Optional[queue.SimpleQueue] = None

# 启停文件日志时使用同一把锁，保证并发场景下状态一致
_file_logging_lock = threading.RLock()


def _make_formatter() -> logging.Formatter:
    # 时间、级别、信息
    return logging.Formatter(fmt="%(asctime)s | %(levelname)s | %(message)s",
                             datefmt="%Y-%m-%d %H:%M:%S")


def _start_file_logging_locked(log_path: Optional[str]) -> None:
    """在已加锁前提下启动异步文件日志。"""
    global _file_handler, _queue_handler, _queue_listener, _log_queue

    if _queue_handler is not None:
        return

    path = os.path.abspath(log_path or _LOG_FILE)
    log_dir = os.path.dirname(path)
    if log_dir:
        os.makedirs(log_dir, exist_ok=True)

    # 后台线程独占文件写入，业务线程只负责入队，降低高频日志的磁盘阻塞
    file_handler = logging.FileHandler(path, mode="a", encoding="utf-8")
    file_handler.setFormatter(_make_formatter())

    log_queue = queue.SimpleQueue()
    queue_handler = QueueHandler(log_queue)
    queue_listener = QueueListener(log_queue, file_handler, respect_handler_level=True)

    try:
        queue_listener.start()
        _logger.addHandler(queue_handler)
    except Exception:
        # 启动失败时及时释放资源，避免句柄泄露
        try:
            queue_listener.stop()
        except Exception:
            pass
        try:
            file_handler.close()
        except Exception:
            pass
        raise

    _file_handler = file_handler
    _queue_handler = queue_handler
    _queue_listener = queue_listener
    _log_queue = log_queue


def _stop_file_logging_locked() -> None:
    """在已加锁前提下停止异步文件日志并释放资源。"""
    global _file_handler, _queue_handler, _queue_listener, _log_queue

    queue_handler = _queue_handler
    queue_listener = _queue_listener
    file_handler = _file_handler

    # 先从 logger 卸载入队 handler，避免停止过程继续接收新日志
    if queue_handler is not None:
        try:
            _logger.removeHandler(queue_handler)
        except Exception:
            pass

    # 停止监听线程并尽可能消费完队列中的剩余日志
    if queue_listener is not None:
        try:
            queue_listener.stop()
        except Exception:
            pass

    if queue_handler is not None:
        try:
            queue_handler.close()
        except Exception:
            pass

    if file_handler is not None:
        try:
            file_handler.close()
        except Exception:
            pass

    _file_handler = None
    _queue_handler = None
    _queue_listener = None
    _log_queue = None


def _shutdown_logging() -> None:
    """进程退出时兜底关闭文件日志线程与句柄。"""
    with _file_logging_lock:
        _stop_file_logging_locked()


atexit.register(_shutdown_logging)


def enable_file_logging(enable: bool, log_path: Optional[str] = None) -> None:
    """根据 enable 开关文件日志输出。"""
    with _file_logging_lock:
        if enable:
            if _queue_handler is None:
                _start_file_logging_locked(log_path)
            _logger.info("文件日志已开启")
        else:
            if _queue_handler is not None:
                _stop_file_logging_locked()
            _logger.info("文件日志已关闭")


def get_logger() -> logging.Logger:
    return _logger
