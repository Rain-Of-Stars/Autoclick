# -*- coding: utf-8 -*-
"""
智能捕获测试管理器
提供更智能的捕获测试功能，防止UI卡顿
"""
from __future__ import annotations
import time
from dataclasses import dataclass, field
from typing import Optional, Callable, Any
from PySide6 import QtCore

from auto_approve.logger_manager import get_logger


@dataclass
class CaptureTestResult:
    """捕获测试结果（运行时版本，避免生产路径依赖tests模块）"""
    success: bool
    image: Any = None
    duration_ms: int = 0
    error_message: str = ""
    metadata: dict = field(default_factory=dict)


class SmartCaptureTestManager(QtCore.QObject):
    """智能捕获测试管理器
    
    主要优化：
    1. 自适应超时时间
    2. 智能重试机制
    3. 资源使用监控
    4. 更好的错误恢复
    """
    
    # 信号定义
    test_started = QtCore.Signal(str)  # 测试类型
    progress_updated = QtCore.Signal(int, str)  # 进度, 消息
    test_completed = QtCore.Signal(object)  # CaptureTestResult
    test_failed = QtCore.Signal(str)  # 错误消息
    test_finished = QtCore.Signal()  # 测试完成
    # 预览任务跨线程结果信号（确保在主线程统一收敛）
    _preview_completed = QtCore.Signal(object)
    _preview_failed = QtCore.Signal(str)
    
    def __init__(self, parent=None):
        super().__init__(parent)
        self.logger = get_logger()
        self._current_tester = None
        self._test_active = False
        self._start_time = 0
        self._adaptive_timeout = 2.0  # 自适应超时时间
        self._preview_request_seq = 0
        self._active_preview_request_id: Optional[int] = None
        self._preview_completed.connect(self._on_test_completed)
        self._preview_failed.connect(self._on_test_failed)

    def _next_preview_request_id(self) -> int:
        """生成预览请求ID，用于取消与过期回调过滤。"""
        self._preview_request_seq += 1
        return self._preview_request_seq

    def _is_preview_request_active(self, request_id: int) -> bool:
        """判断预览请求是否仍为当前有效任务。"""
        return self._test_active and self._active_preview_request_id == request_id

    def _start_preview_window_task(self, hwnd: int, timeout_sec: float):
        """异步执行窗口单帧预览（生产实现，不依赖tests模块）。"""
        from workers.io_tasks import IOTaskBase, submit_io

        request_id = self._next_preview_request_id()
        self._active_preview_request_id = request_id
        self.progress_updated.emit(15, "正在初始化单帧窗口预览...")

        class _PreviewWindowTask(IOTaskBase):
            def __init__(self, target_hwnd: int, timeout_value: float, req_id: int):
                super().__init__(f"preview_window_{req_id}")
                self.target_hwnd = int(target_hwnd)
                self.timeout_value = float(timeout_value)

            def execute(self):
                start = time.monotonic()
                from capture.single_frame_preview import get_single_frame_preview
                image = get_single_frame_preview().capture_single_frame(
                    self.target_hwnd, timeout_sec=self.timeout_value
                )
                duration_ms = int((time.monotonic() - start) * 1000)
                if image is None:
                    return {
                        "success": False,
                        "error": "单帧预览未获取到图像",
                        "duration_ms": duration_ms,
                    }
                return {
                    "success": True,
                    "image": image,
                    "duration_ms": duration_ms,
                    "metadata": {
                        "hwnd": self.target_hwnd,
                        "preview_mode": "single_frame_runtime",
                        "image_shape": image.shape,
                    },
                }

        def _on_success(_task_id: str, result: dict):
            if not self._is_preview_request_active(request_id):
                return
            if result.get("success"):
                self._preview_completed.emit(
                    CaptureTestResult(
                        success=True,
                        image=result.get("image"),
                        duration_ms=int(result.get("duration_ms", 0)),
                        metadata=result.get("metadata") or {},
                    )
                )
            else:
                self._preview_failed.emit(str(result.get("error") or "单帧预览失败"))

        def _on_error(_task_id: str, error_message: str, _exception):
            if not self._is_preview_request_active(request_id):
                return
            self._preview_failed.emit(f"单帧预览异常: {error_message}")

        submit_io(_PreviewWindowTask(hwnd, timeout_sec, request_id), _on_success, _on_error)

    def _start_preview_monitor_task(self, monitor_index: int, timeout_sec: float):
        """异步执行屏幕单帧预览（生产实现，不依赖tests模块）。"""
        from workers.io_tasks import IOTaskBase, submit_io

        request_id = self._next_preview_request_id()
        self._active_preview_request_id = request_id
        self.progress_updated.emit(15, "正在初始化单帧屏幕预览...")

        class _PreviewMonitorTask(IOTaskBase):
            def __init__(self, monitor_idx: int, timeout_value: float, req_id: int):
                super().__init__(f"preview_monitor_{req_id}")
                self.monitor_idx = int(monitor_idx)
                self.timeout_value = max(0.1, float(timeout_value))

            def execute(self):
                from capture.capture_manager import CaptureManager

                start = time.monotonic()
                capture_manager = CaptureManager()
                try:
                    capture_manager.configure(
                        fps=15,
                        include_cursor=False,
                        border_required=False,
                        restore_minimized=False,
                    )
                    if not capture_manager.open_monitor(self.monitor_idx):
                        duration_ms = int((time.monotonic() - start) * 1000)
                        return {
                            "success": False,
                            "error": f"无法打开显示器 {self.monitor_idx}",
                            "duration_ms": duration_ms,
                        }

                    deadline = time.monotonic() + self.timeout_value
                    image = None
                    while time.monotonic() < deadline:
                        image = capture_manager.capture_frame_fast()
                        if image is not None:
                            break
                        time.sleep(0.01)

                    if image is None:
                        image = capture_manager.capture_frame()

                    duration_ms = int((time.monotonic() - start) * 1000)
                    if image is None:
                        return {
                            "success": False,
                            "error": "单帧屏幕预览未获取到图像",
                            "duration_ms": duration_ms,
                        }

                    return {
                        "success": True,
                        "image": image,
                        "duration_ms": duration_ms,
                        "metadata": {
                            "monitor_index": self.monitor_idx,
                            "preview_mode": "single_frame_monitor_runtime",
                            "image_shape": image.shape,
                        },
                    }
                finally:
                    try:
                        capture_manager.close()
                    except Exception:
                        pass

        def _on_success(_task_id: str, result: dict):
            if not self._is_preview_request_active(request_id):
                return
            if result.get("success"):
                self._preview_completed.emit(
                    CaptureTestResult(
                        success=True,
                        image=result.get("image"),
                        duration_ms=int(result.get("duration_ms", 0)),
                        metadata=result.get("metadata") or {},
                    )
                )
            else:
                self._preview_failed.emit(str(result.get("error") or "单帧预览失败"))

        def _on_error(_task_id: str, error_message: str, _exception):
            if not self._is_preview_request_active(request_id):
                return
            self._preview_failed.emit(f"单帧屏幕预览异常: {error_message}")

        submit_io(_PreviewMonitorTask(monitor_index, timeout_sec, request_id), _on_success, _on_error)
        
    def start_smart_window_test(self, hwnd: int, callback: Optional[Callable] = None, 
                            use_fast: bool = True, preview_only: bool = False):
        """开始智能窗口捕获测试
        
        Args:
            hwnd: 窗口句柄
            callback: 回调函数
            use_fast: 是否使用快速模式
            preview_only: 是否仅预览（单帧）
        """
        if self._test_active:
            self.logger.warning("已有测试在进行中，忽略新请求")
            return
            
        self._test_active = True
        self._start_time = time.time()
        self.test_started.emit("window")
        
        # 根据参数选择测试器类型
        if preview_only:
            self._current_tester = None
            
            # 单帧预览使用极短超时
            preview_timeout = 0.8  # 预留初始化预算，避免首帧前超时
            self.logger.info(f"使用单帧预览模式，超时时间: {preview_timeout:.1f}秒")
            
            # 启动预览
            self._start_preview_window_task(hwnd, preview_timeout)
            
        elif use_fast:
            # 使用极速测试器避免UI卡顿
            from tests.test_fast_capture import FastCaptureTest
            self._current_tester = FastCaptureTest(self)
            
            # 连接信号
            self._current_tester.test_completed.connect(self._on_test_completed)
            self._current_tester.test_failed.connect(self._on_test_failed)
            
            # 快速测试使用固定短超时
            fast_timeout = 1.0  # 1秒快速测试
            self.logger.info(f"使用快速测试模式，超时时间: {fast_timeout:.1f}秒")
            
            # 启动快速测试
            self._current_tester.test_window_fast(hwnd, timeout_sec=fast_timeout)
        else:
            # 使用标准测试器
            from tests.test_non_blocking_capture import NonBlockingCaptureTest
            self._current_tester = NonBlockingCaptureTest(self)
            
            # 连接信号
            self._current_tester.progress_updated.connect(self._on_test_progress)
            self._current_tester.test_completed.connect(self._on_test_completed)
            self._current_tester.test_failed.connect(self._on_test_failed)
            
            # 使用自适应超时
            adaptive_timeout = self._calculate_adaptive_timeout(hwnd)
            self.logger.info(f"使用标准测试模式，自适应超时: {adaptive_timeout:.1f}秒")
            
            # 启动标准测试
            self._current_tester.test_window_capture_async(hwnd, timeout_sec=adaptive_timeout)
        
    def start_smart_monitor_test(self, monitor_index: int, callback: Optional[Callable] = None, 
                             use_fast: bool = True, preview_only: bool = False):
        """开始智能屏幕捕获测试
        
        Args:
            monitor_index: 显示器索引
            callback: 回调函数
            use_fast: 是否使用快速模式
            preview_only: 是否仅预览（单帧）
        """
        if self._test_active:
            self.logger.warning("已有测试在进行中，忽略新请求")
            return
            
        self._test_active = True
        self._start_time = time.time()
        self.test_started.emit("monitor")
        
        # 根据参数选择测试器类型
        if preview_only:
            self._current_tester = None
            
            # 单帧预览使用极短超时
            preview_timeout = 0.8  # 预留初始化预算，避免首帧前超时
            self.logger.info(f"使用单帧预览模式，超时时间: {preview_timeout:.1f}秒")
            
            # 启动预览
            self._start_preview_monitor_task(monitor_index, preview_timeout)
            
        elif use_fast:
            # 使用极速测试器避免UI卡顿
            from tests.test_fast_capture import FastCaptureTest
            self._current_tester = FastCaptureTest(self)
            
            # 连接信号
            self._current_tester.test_completed.connect(self._on_test_completed)
            self._current_tester.test_failed.connect(self._on_test_failed)
            
            # 快速测试使用固定短超时
            fast_timeout = 1.0  # 1秒快速测试
            self.logger.info(f"使用快速测试模式，超时时间: {fast_timeout:.1f}秒")
            
            # 启动快速测试
            self._current_tester.test_monitor_fast(monitor_index, timeout_sec=fast_timeout)
        else:
            # 使用标准测试器
            from tests.test_non_blocking_capture import NonBlockingCaptureTest
            self._current_tester = NonBlockingCaptureTest(self)
            
            # 连接信号
            self._current_tester.progress_updated.connect(self._on_test_progress)
            self._current_tester.test_completed.connect(self._on_test_completed)
            self._current_tester.test_failed.connect(self._on_test_failed)
            
            # 屏幕捕获通常更快，使用较短超时
            adaptive_timeout = min(self._adaptive_timeout, 1.5)
            self.logger.info(f"使用标准测试模式，超时时间: {adaptive_timeout:.1f}秒")
            
            # 启动标准测试
            self._current_tester.test_monitor_capture_async(monitor_index, timeout_sec=adaptive_timeout)
        
    def cancel_current_test(self):
        """取消当前测试"""
        if self._active_preview_request_id is not None and self._test_active:
            self.logger.info("取消当前单帧预览测试")
            self._active_preview_request_id = None
            self._test_active = False
            self.test_finished.emit()
            return

        if self._current_tester and self._test_active:
            self.logger.info("取消当前捕获测试")
            self._current_tester.cancel_test()
            
    def _calculate_adaptive_timeout(self, hwnd: int) -> float:
        """计算自适应超时时间"""
        # 基于历史性能数据调整超时时间
        # 这里可以添加更复杂的逻辑，比如：
        # - 记录历史测试时间
        # - 基于窗口类型调整
        # - 基于系统负载调整
        
        # 简单实现：根据时间动态调整
        current_hour = time.localtime().tm_hour
        
        # 在工作时间（9-18点）使用较短超时
        if 9 <= current_hour <= 18:
            return min(self._adaptive_timeout, 1.5)
        else:
            return self._adaptive_timeout
            
    def _on_test_progress(self, progress: int, message: str):
        """处理测试进度更新"""
        # 添加智能进度反馈
        elapsed_time = time.time() - self._start_time
        
        # 如果测试时间过长，记录警告
        if elapsed_time > self._adaptive_timeout * 0.8:
            self.logger.warning(f"捕获测试耗时较长: {elapsed_time:.1f}秒")
        
        # 发送进度信号
        self.progress_updated.emit(progress, message)
        
    def _on_test_completed(self, result: CaptureTestResult):
        """处理测试完成"""
        self._test_active = False
        self._active_preview_request_id = None
        self._current_tester = None
        
        # 记录测试结果
        elapsed_time = time.time() - self._start_time
        self.logger.info(f"捕获测试完成，耗时: {elapsed_time:.1f}秒，结果: {result.success}")
        
        # 更新自适应超时基于实际性能
        if result.success:
            # 如果测试成功且时间合理，稍微减少超时时间
            if result.duration_ms < 1000:  # 小于1秒
                self._adaptive_timeout = max(1.0, self._adaptive_timeout * 0.9)
            elif result.duration_ms > 3000:  # 大于3秒
                self._adaptive_timeout = min(3.0, self._adaptive_timeout * 1.1)
        
        # 发送完成信号
        self.test_completed.emit(result)
        self.test_finished.emit()
        
    def _on_test_failed(self, error_message: str):
        """处理测试失败"""
        self._test_active = False
        self._active_preview_request_id = None
        self._current_tester = None
        
        # 记录失败
        elapsed_time = time.time() - self._start_time
        self.logger.warning(f"捕获测试失败，耗时: {elapsed_time:.1f}秒，错误: {error_message}")
        
        # 如果失败，增加超时时间
        self._adaptive_timeout = min(3.0, self._adaptive_timeout * 1.2)
        
        # 发送失败信号
        self.test_failed.emit(error_message)
        self.test_finished.emit()
        
    def is_test_active(self) -> bool:
        """检查是否有测试正在进行"""
        return self._test_active
        
    def get_adaptive_timeout(self) -> float:
        """获取当前自适应超时时间"""
        return self._adaptive_timeout


# 全局实例
_global_smart_capture_manager: Optional[SmartCaptureTestManager] = None


def get_smart_capture_manager() -> SmartCaptureTestManager:
    """获取全局智能捕获测试管理器"""
    global _global_smart_capture_manager
    if _global_smart_capture_manager is None:
        _global_smart_capture_manager = SmartCaptureTestManager()
    return _global_smart_capture_manager
