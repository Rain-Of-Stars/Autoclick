# -*- coding: utf-8 -*-
"""
WGC实时预览对话框
提供实时窗口捕获预览功能，方便用户验证WGC配置效果
"""

import time
import threading
from pathlib import Path
import numpy as np
import cv2
from PySide6 import QtWidgets, QtCore, QtGui
from typing import Optional


class WGCPreviewDialog(QtWidgets.QDialog):
    """WGC实时预览对话框"""
    _startup_progress_signal = QtCore.Signal(int, str)
    _startup_result_signal = QtCore.Signal(int, bool, str, float)
    
    def __init__(self, hwnd: int, parent=None, *, fps: int = 15, include_cursor: bool = False, border_required: bool = False):
        super().__init__(parent)
        self.hwnd = hwnd
        self.capture_manager = None
        self.timer = None
        self.is_capturing = False
        # 预览配置（从外部传入，确保与设置面板一致）
        self.preview_fps = max(1, min(int(fps), 60))
        self.include_cursor = bool(include_cursor)
        self.border_required = bool(border_required)

        # 共享帧缓存用户ID
        self.user_id = f"wgc_preview_{hwnd}_{int(time.time())}"
        
        self.setWindowTitle(f"WGC实时预览 - HWND: {hwnd}")
        self.resize(800, 600)
        self.setModal(True)
        
        self._setup_ui()
        self._setup_capture()

        # 启动链路状态
        self._is_starting = False
        self._start_request_seq = 0
        self._active_start_request_id: Optional[int] = None
        self._startup_elapsed_ms = 0.0
        self._waiting_first_frame = False
        self._first_frame_timeout_ms = 900
        self._first_frame_timeout_timer = QtCore.QTimer(self)
        self._first_frame_timeout_timer.setSingleShot(True)
        self._first_frame_timeout_timer.timeout.connect(self._on_first_frame_timeout)

        # 启动后台线程与主线程通信
        self._startup_progress_signal.connect(self._on_startup_progress)
        self._startup_result_signal.connect(self._on_startup_result)
        
    def _setup_ui(self):
        """设置UI界面"""
        layout = QtWidgets.QVBoxLayout(self)
        
        # 顶部信息栏
        info_layout = QtWidgets.QHBoxLayout()
        
        self.info_label = QtWidgets.QLabel(f"目标窗口: HWND {self.hwnd}")
        self.info_label.setStyleSheet("font-weight: bold; color: #2F80ED;")
        info_layout.addWidget(self.info_label)
        
        info_layout.addStretch()
        
        self.fps_label = QtWidgets.QLabel("FPS: --")
        info_layout.addWidget(self.fps_label)
        
        self.size_label = QtWidgets.QLabel("尺寸: --")
        info_layout.addWidget(self.size_label)
        
        layout.addLayout(info_layout)
        
        # 预览区域
        self.preview_label = QtWidgets.QLabel()
        self.preview_label.setAlignment(QtCore.Qt.AlignCenter)
        self.preview_label.setMinimumSize(400, 300)
        self.preview_label.setStyleSheet("""
            QLabel {
                border: 2px solid #E0E0E0;
                background-color: #F5F5F5;
                border-radius: 4px;
            }
        """)
        self.preview_label.setText("正在初始化预览...")
        
        # 将预览标签放在滚动区域中
        scroll_area = QtWidgets.QScrollArea()
        scroll_area.setWidget(self.preview_label)
        scroll_area.setWidgetResizable(True)
        scroll_area.setAlignment(QtCore.Qt.AlignCenter)
        
        layout.addWidget(scroll_area, 1)
        
        # 控制按钮
        control_layout = QtWidgets.QHBoxLayout()
        
        self.start_btn = QtWidgets.QPushButton("开始预览")
        self.start_btn.clicked.connect(self._start_preview)
        control_layout.addWidget(self.start_btn)
        
        self.stop_btn = QtWidgets.QPushButton("停止预览")
        self.stop_btn.clicked.connect(self._stop_preview)
        self.stop_btn.setEnabled(False)
        control_layout.addWidget(self.stop_btn)
        
        control_layout.addStretch()
        
        self.save_btn = QtWidgets.QPushButton("保存当前帧")
        self.save_btn.clicked.connect(self._save_current_frame)
        self.save_btn.setEnabled(False)
        control_layout.addWidget(self.save_btn)
        
        close_btn = QtWidgets.QPushButton("关闭")
        close_btn.clicked.connect(self.accept)
        control_layout.addWidget(close_btn)
        
        layout.addLayout(control_layout)
        
        # 状态栏
        self.status_label = QtWidgets.QLabel("就绪")
        self.status_label.setStyleSheet("color: #666; font-size: 12px;")
        layout.addWidget(self.status_label)
        
        # 用于FPS计算
        self.frame_count = 0
        self.last_fps_time = time.time()
        self.current_frame = None
        self._capture_fail_count = 0
        self._exception_count = 0
        # 记录当前异步保存任务的文件名，供回调线程安全读取
        self._pending_save_filename = None
        
    def _setup_capture(self):
        """设置捕获管理器"""
        try:
            from capture import CaptureManager
            self.capture_manager = CaptureManager()
            
            # 配置参数：使用外部传入的光标/边框开关，确保与设置勾选一致
            self.capture_manager.configure(
                fps=self.preview_fps,  # 预览使用较低帧率
                include_cursor=self.include_cursor,
                border_required=self.border_required,
                restore_minimized=True
            )
            
            self.status_label.setText("捕获管理器初始化成功")
            
        except Exception as e:
            self.status_label.setText(f"初始化失败: {e}")
            QtWidgets.QMessageBox.critical(self, "错误", f"初始化捕获管理器失败: {e}")
    
    def _start_preview(self):
        """开始预览"""
        if not self.capture_manager:
            return

        if self.is_capturing or self._is_starting:
            self.status_label.setText("预览启动中，请稍候...")
            return

        # 新启动请求编号（用于过滤过期回调）
        self._start_request_seq += 1
        request_id = self._start_request_seq
        self._active_start_request_id = request_id
        self._is_starting = True
        self._waiting_first_frame = False
        self._capture_fail_count = 0
        self._exception_count = 0
        self.current_frame = None

        # 禁用开始按钮，防止重复点击
        self.start_btn.setEnabled(False)
        self.stop_btn.setEnabled(False)
        self.save_btn.setEnabled(False)
        self.preview_label.setText("正在启动预览...")
        self.status_label.setText(f"启动中[{request_id}]：已提交后台任务")

        # 使用后台线程执行open_window，避免主线程阻塞
        worker = threading.Thread(
            target=self._open_window_async_task,
            args=(request_id,),
            daemon=True,
            name=f"wgc_preview_start_{request_id}"
        )
        worker.start()

    def _open_window_async_task(self, request_id: int):
        """后台线程：执行open_window，完成后通过信号回主线程。"""
        start_time = time.time()
        self._startup_progress_signal.emit(request_id, f"启动中[{request_id}]：后台正在初始化WGC会话")
        try:
            success = self.capture_manager.open_window(self.hwnd, async_init=True, timeout=3.0)
            elapsed_ms = (time.time() - start_time) * 1000
            detail = f"启动中[{request_id}]：后台初始化完成，耗时{elapsed_ms:.0f}ms"
            self._startup_result_signal.emit(request_id, success, detail, elapsed_ms)
        except Exception as e:
            elapsed_ms = (time.time() - start_time) * 1000
            detail = f"启动中[{request_id}]：后台异常，耗时{elapsed_ms:.0f}ms"
            self._startup_result_signal.emit(request_id, False, f"{detail}，原因: {e}", elapsed_ms)

    def _on_startup_progress(self, request_id: int, status_text: str):
        """主线程：接收后台启动状态。"""
        if request_id != self._active_start_request_id or not self._is_starting:
            return
        self.status_label.setText(status_text)

    def _on_startup_result(self, request_id: int, success: bool, detail: str, elapsed_ms: float):
        """主线程：处理后台启动结果，并安全启动定时器。"""
        # 忽略已过期的回调，但要确保资源不泄漏
        if request_id != self._active_start_request_id:
            # 仅在当前没有新的启动或预览会话时再关闭，避免误杀新请求
            if success and self.capture_manager and not self._is_starting and not self.is_capturing:
                self.capture_manager.close()
            return

        self._is_starting = False
        self._startup_elapsed_ms = elapsed_ms
        self._active_start_request_id = None

        if not success:
            self.status_label.setText("启动失败")
            self.start_btn.setEnabled(True)
            self.stop_btn.setEnabled(False)
            self.save_btn.setEnabled(False)
            QtWidgets.QMessageBox.warning(
                self,
                "失败",
                f"无法启动窗口捕获，请检查窗口是否有效\n\n诊断信息: {detail}"
            )
            return

        # 主线程安全创建并启动定时器
        if self.timer:
            self.timer.stop()
            self.timer = None
        self.timer = QtCore.QTimer(self)
        self.timer.timeout.connect(self._capture_frame)
        interval = max(16, int(1000 / self.preview_fps))  # 最少16ms (约60FPS)
        self.timer.start(interval)

        self.is_capturing = True
        self.stop_btn.setEnabled(True)
        self.save_btn.setEnabled(False)
        self._waiting_first_frame = True
        self._first_frame_timeout_timer.start(self._first_frame_timeout_ms)

        self.status_label.setText(f"启动成功：耗时{elapsed_ms:.0f}ms，等待首帧...")
        # 立即请求一次抓帧，缩短首帧到达时间
        QtCore.QTimer.singleShot(0, self._capture_frame)

    def _on_first_frame_timeout(self):
        """首帧超时提示（仅提示，不中断重试）。"""
        if not self.is_capturing or not self._waiting_first_frame:
            return
        self.status_label.setText(
            f"首帧等待超时(>{self._first_frame_timeout_ms}ms)，仍在重试..."
        )
    
    def _stop_preview(self):
        """停止预览"""
        was_starting = self._is_starting
        self._is_starting = False
        self._active_start_request_id = None
        self._waiting_first_frame = False
        self._first_frame_timeout_timer.stop()

        if self.timer:
            self.timer.stop()
            self.timer = None

        if self.capture_manager:
            # 释放共享帧缓存引用
            self.capture_manager.release_shared_frame(self.user_id)
            self.capture_manager.close()

        self.is_capturing = False
        self.start_btn.setEnabled(True)
        self.stop_btn.setEnabled(False)
        self.save_btn.setEnabled(False)

        self.status_label.setText("已取消启动" if was_starting else "已停止")
        self.preview_label.setText("预览已停止")
    
    def _capture_frame(self):
        """捕获并显示一帧（使用共享帧缓存）"""
        if not self.capture_manager or not self.is_capturing:
            return

        try:
            # 添加超时保护，避免长时间阻塞
            start_time = time.time()

            # 优先使用共享帧缓存
            frame = self.capture_manager.get_shared_frame(self.user_id, "preview")
            if frame is None:
                # 如果共享缓存没有，尝试传统捕获（带超时）
                frame = self.capture_manager.capture_frame()
                if frame is None:
                    self._capture_fail_count += 1

                    # 首帧阶段：更快反馈，缩短失败感知延迟
                    if self._waiting_first_frame:
                        if self._capture_fail_count in (3, 6):
                            self.status_label.setText(
                                f"等待首帧中... 已重试{self._capture_fail_count}次"
                            )
                        return

                    # 非首帧阶段：降低提示频率，避免状态栏抖动
                    if self._capture_fail_count > 15:
                        self.status_label.setText("捕获失败，请检查窗口状态")
                        self._capture_fail_count = 0
                    return

            # 重置失败计数
            self._capture_fail_count = 0
            self._exception_count = 0

            # 首帧到达：停止首帧超时提示并更新状态
            if self._waiting_first_frame:
                self._waiting_first_frame = False
                self._first_frame_timeout_timer.stop()
                self.status_label.setText(
                    f"预览中... 首帧已到达（启动耗时{self._startup_elapsed_ms:.0f}ms）"
                )

            # 检查捕获耗时
            capture_time = (time.time() - start_time) * 1000
            if capture_time > 100:  # 超过100ms
                self.status_label.setText(f"捕获较慢: {capture_time:.1f}ms")

            self.current_frame = frame
            h, w = frame.shape[:2]

            # 更新尺寸信息
            self.size_label.setText(f"尺寸: {w}×{h}")

            # 计算FPS
            self.frame_count += 1
            current_time = time.time()
            if current_time - self.last_fps_time >= 1.0:
                fps = self.frame_count / (current_time - self.last_fps_time)
                self.fps_label.setText(f"FPS: {fps:.1f}")
                self.frame_count = 0
                self.last_fps_time = current_time

            # 转换为Qt图像并显示
            self._display_frame(frame)

        except Exception as e:
            self.status_label.setText(f"捕获错误: {e}")
            # 连续异常计数
            self._exception_count += 1

            # 如果连续异常超过10次，停止预览
            if self._exception_count > 10:
                self.status_label.setText("捕获异常过多，已停止预览")
                self._stop_preview()
    
    def _display_frame(self, frame):
        """显示帧到预览标签"""
        try:
            h, w = frame.shape[:2]
            
            # 确保图像数据连续
            if not frame.flags['C_CONTIGUOUS']:
                frame = np.ascontiguousarray(frame)
            
            # BGR转RGB
            rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            
            # 创建QImage
            bytes_per_line = rgb.strides[0] if rgb.strides else w * 3
            qimg = QtGui.QImage(rgb.data, w, h, bytes_per_line, QtGui.QImage.Format_RGB888)
            
            if qimg.isNull():
                return
            
            # 创建像素图并缩放到合适大小
            pixmap = QtGui.QPixmap.fromImage(qimg)
            if pixmap.isNull():
                return
            
            # 缩放到预览标签大小，保持宽高比
            label_size = self.preview_label.size()
            scaled_pixmap = pixmap.scaled(
                label_size, 
                QtCore.Qt.KeepAspectRatio, 
                QtCore.Qt.SmoothTransformation
            )
            
            self.preview_label.setPixmap(scaled_pixmap)
            
        except Exception as e:
            self.status_label.setText(f"显示错误: {e}")
    
    def _save_current_frame(self):
        """保存当前帧 - 使用非阻塞I/O"""
        if self.current_frame is None:
            QtWidgets.QMessageBox.warning(self, "提示", "没有可保存的帧")
            return
        
        # 生成默认名称（不再写入磁盘，仅用于数据库键名）
        timestamp = int(time.time())
        default_filename = f"wgc_preview_capture_{timestamp}.png"
        filename = default_filename
        self._pending_save_filename = filename
        
        # 禁用保存按钮，防止重复点击
        self.save_btn.setEnabled(False)
        self.status_label.setText("正在保存图像...")
        QtWidgets.QApplication.processEvents()
        
        try:
            # 使用非阻塞I/O线程池保存图像（仅数据库）
            from workers.io_tasks import submit_image_save
            
            # 获取“扩展名”来确定质量设置（仅影响编码质量）
            file_extension = Path(filename).suffix.lower()
            quality = 95 if file_extension in ['.jpg', '.jpeg'] else 100
            
            def on_save_success(task_id, result):
                """保存成功的回调"""
                # 在主线程中更新UI
                QtCore.QTimer.singleShot(0, lambda: self._on_save_complete(True, result))
                
            def on_save_error(task_id, error_message, exception):
                """保存失败的回调"""  
                # 在主线程中更新UI
                QtCore.QTimer.singleShot(0, lambda: self._on_save_complete(False, error_message))
            
            # 提交非阻塞保存任务（保存到SQLite的images表）
            self.save_task_id = submit_image_save(
                self.current_frame,
                filename,
                quality,
                on_save_success,
                on_save_error
            )
            
        except Exception as e:
            # 回退：直接提示失败（不再进行磁盘写入）
            self._finish_save_ui("保存失败")
            self._pending_save_filename = None
            QtWidgets.QMessageBox.critical(self, "错误", f"保存失败: {e}")

    def _finish_save_ui(self, status_text: str):
        """统一恢复保存按钮与状态文本，避免异常路径遗漏。"""
        self.save_btn.setEnabled(self.is_capturing and self.current_frame is not None)
        self.status_label.setText(status_text)
    
    def _on_save_complete(self, success, result):
        """保存完成后的UI更新"""
        save_name = Path(self._pending_save_filename).name if self._pending_save_filename else "当前帧"
        try:
            if success:
                file_info = result
                file_size_mb = file_info['file_size'] / (1024 * 1024)
                dimensions = file_info['dimensions']
                self._finish_save_ui(f"保存成功: {save_name}")
                QtWidgets.QMessageBox.information(
                    self,
                    "保存成功",
                    f"图像已保存到数据库:\n{file_info['file_path']}\n\n"
                    f"尺寸: {dimensions[0]}×{dimensions[1]}\n"
                    f"大小: {file_size_mb:.2f} MB\n"
                    f"质量: {file_info['quality']}\n"
                    f"image_id: {file_info.get('image_id')}"
                )
            else:
                self._finish_save_ui("保存失败")
                QtWidgets.QMessageBox.critical(self, "保存失败", f"保存图像时出错:\n{result}")
        finally:
            self._pending_save_filename = None
    
    # 取消磁盘回退保存逻辑
    
    def closeEvent(self, event):
        """关闭事件"""
        self._stop_preview()
        # 确保释放共享帧缓存引用
        if self.capture_manager:
            self.capture_manager.release_shared_frame(self.user_id)
        super().closeEvent(event)
    
    def resizeEvent(self, event):
        """窗口大小改变事件"""
        super().resizeEvent(event)
        # 如果有当前帧，重新显示以适应新尺寸
        if self.current_frame is not None:
            self._display_frame(self.current_frame)
