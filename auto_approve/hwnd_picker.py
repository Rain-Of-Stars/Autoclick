# -*- coding: utf-8 -*-
"""
HWND 获取工具（集成到设置中使用）：
- 列出可见窗口并搜索过滤；
- 支持拖拽十字准星到目标窗口获取 HWND；
- 显示窗口详细信息；
- 支持“测试捕获”调用窗口级截屏后端验证。
"""
from __future__ import annotations
import ctypes
from ctypes import wintypes
from typing import Optional, Tuple

from PySide6 import QtWidgets, QtCore, QtGui
import os

from capture.monitor_utils import find_window_by_process, is_electron_process


# Windows API
user32 = ctypes.WinDLL('user32', use_last_error=True)
kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
psapi = ctypes.WinDLL('psapi', use_last_error=True)

user32.GetCursorPos.restype = wintypes.BOOL
user32.GetCursorPos.argtypes = [ctypes.POINTER(ctypes.wintypes.POINT)]

user32.WindowFromPoint.restype = wintypes.HWND
user32.WindowFromPoint.argtypes = [ctypes.wintypes.POINT]

user32.GetWindowTextW.restype = ctypes.c_int
user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]

user32.GetWindowTextLengthW.restype = ctypes.c_int
user32.GetWindowTextLengthW.argtypes = [wintypes.HWND]

user32.GetClassNameW.restype = ctypes.c_int
user32.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]

user32.IsWindowVisible.restype = wintypes.BOOL
user32.IsWindowVisible.argtypes = [wintypes.HWND]

user32.GetWindowRect.restype = wintypes.BOOL
user32.GetWindowRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]

user32.GetWindowThreadProcessId.restype = wintypes.DWORD
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]

# 获取祖先窗口（顶层窗口）相关定义
GA_PARENT = 1
GA_ROOT = 2
GA_ROOTOWNER = 3
user32.GetAncestor.restype = wintypes.HWND
user32.GetAncestor.argtypes = [wintypes.HWND, wintypes.UINT]

kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]

psapi.GetModuleFileNameExW.restype = wintypes.DWORD
psapi.GetModuleFileNameExW.argtypes = [wintypes.HANDLE, wintypes.HMODULE, wintypes.LPWSTR, wintypes.DWORD]

kernel32.CloseHandle.restype = wintypes.BOOL
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]


class WindowInfo:
    """窗口信息封装。"""
    def __init__(self, hwnd: int):
        self.hwnd = hwnd
        self.title = self._get_window_title()
        self.class_name = self._get_class_name()
        self.rect = self._get_window_rect()
        self.process_name = self._get_process_name()
        self.process_id = self._get_process_id()
        self.is_electron = is_electron_process(hwnd)
        self.is_visible = bool(user32.IsWindowVisible(hwnd))

    def _get_window_title(self) -> str:
        try:
            length = user32.GetWindowTextLengthW(self.hwnd)
            if length > 0:
                buf = ctypes.create_unicode_buffer(length + 1)
                user32.GetWindowTextW(self.hwnd, buf, length + 1)
                return buf.value
        except Exception:
            pass
        return ""

    def _get_class_name(self) -> str:
        try:
            buf = ctypes.create_unicode_buffer(256)
            n = user32.GetClassNameW(self.hwnd, buf, 256)
            if n > 0:
                return buf.value
        except Exception:
            pass
        return ""

    def _get_window_rect(self):
        try:
            rect = wintypes.RECT()
            if user32.GetWindowRect(self.hwnd, ctypes.byref(rect)):
                return {
                    'left': rect.left,
                    'top': rect.top,
                    'right': rect.right,
                    'bottom': rect.bottom,
                    'width': rect.right - rect.left,
                    'height': rect.bottom - rect.top,
                }
        except Exception:
            pass
        return None

    def _get_process_id(self) -> int:
        try:
            pid = wintypes.DWORD()
            user32.GetWindowThreadProcessId(self.hwnd, ctypes.byref(pid))
            return pid.value
        except Exception:
            return 0

    def _get_process_name(self) -> str:
        try:
            pid = self._get_process_id()
            if not pid:
                return ""
            h = kernel32.OpenProcess(0x0400, False, pid)
            if not h:
                return ""
            try:
                buf = ctypes.create_unicode_buffer(260)
                if psapi.GetModuleFileNameExW(h, None, buf, 260):
                    return buf.value
            finally:
                kernel32.CloseHandle(h)
        except Exception:
            pass
        return ""


class _WindowEnumWorker(QtCore.QObject):
    """后台枚举窗口，避免主线程在刷新时阻塞。"""
    batch_ready = QtCore.Signal(int, list)
    finished = QtCore.Signal(int)
    error = QtCore.Signal(int, str)

    def __init__(self, token: int, batch_size: int = 32):
        super().__init__()
        self._token = token
        self._batch_size = max(1, int(batch_size))
        self._canceled = False

    def cancel(self):
        self._canceled = True

    @QtCore.Slot()
    def run(self):
        rows: list[tuple[int, str, str]] = []
        enum_proc_ref = None
        try:
            @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
            def enum_proc(hwnd, _lp):
                if self._canceled:
                    return False
                if not user32.IsWindowVisible(hwnd):
                    return True
                ln = user32.GetWindowTextLengthW(hwnd)
                if ln <= 0:
                    return True
                buf = ctypes.create_unicode_buffer(ln + 1)
                user32.GetWindowTextW(hwnd, buf, ln + 1)
                title = buf.value
                try:
                    process_name = WindowInfo(hwnd).process_name
                except Exception:
                    process_name = ""
                rows.append((int(hwnd), title, process_name))
                if len(rows) >= self._batch_size:
                    self.batch_ready.emit(self._token, rows[:])
                    rows.clear()
                return True

            # 保留回调引用，确保枚举期间不会被回收。
            enum_proc_ref = enum_proc
            user32.EnumWindows(enum_proc_ref, 0)
            if rows and not self._canceled:
                self.batch_ready.emit(self._token, rows[:])
        except Exception as e:
            self.error.emit(self._token, str(e))
        finally:
            self.finished.emit(self._token)
            enum_proc_ref = None


class WindowListWidget(QtWidgets.QTableWidget):
    """窗口列表控件。"""
    window_selected = QtCore.Signal(int)

    def __init__(self, parent=None):
        super().__init__(0, 3, parent)
        self.setHorizontalHeaderLabels(["HWND", "标题", "进程"])
        self.horizontalHeader().setStretchLastSection(True)
        self.setSelectionBehavior(QtWidgets.QAbstractItemView.SelectRows)
        self.setEditTriggers(QtWidgets.QAbstractItemView.NoEditTriggers)
        self.setAlternatingRowColors(True)
        self.itemSelectionChanged.connect(self._on_sel)
        self._refresh_token = 0
        self._refresh_thread: Optional[QtCore.QThread] = None
        self._refresh_worker: Optional[_WindowEnumWorker] = None
        self._pending_rows: list[tuple[int, str, str]] = []
        self._flush_timer = QtCore.QTimer(self)
        self._flush_timer.setSingleShot(True)
        self._flush_timer.timeout.connect(self._flush_pending_rows)
        self._loading_row_visible = False
        self.refresh_window_list()

    def refresh_window_list(self):
        """异步枚举可见窗口，并分块填充表格。"""
        self._refresh_token += 1
        token = self._refresh_token
        self._cancel_refresh_worker()
        self._pending_rows.clear()
        self._flush_timer.stop()
        self.setUpdatesEnabled(False)
        self.setRowCount(0)
        self._insert_loading_row()
        self.setUpdatesEnabled(True)
        self.setSortingEnabled(False)

        thread = QtCore.QThread(self)
        worker = _WindowEnumWorker(token=token, batch_size=32)
        worker.moveToThread(thread)
        thread.started.connect(worker.run)
        worker.batch_ready.connect(self._on_batch_ready)
        worker.error.connect(self._on_refresh_error)
        worker.finished.connect(self._on_refresh_finished)
        worker.finished.connect(thread.quit)
        worker.finished.connect(worker.deleteLater)
        thread.finished.connect(thread.deleteLater)
        self._refresh_thread = thread
        self._refresh_worker = worker
        thread.start()

    def _cancel_refresh_worker(self):
        """取消上一轮刷新，避免重复枚举导致主线程来回重绘。"""
        try:
            if self._refresh_worker is not None:
                self._refresh_worker.cancel()
        except Exception:
            pass

    def _insert_loading_row(self):
        """显示轻量占位行，提示用户正在刷新。"""
        self.insertRow(0)
        item = QtWidgets.QTableWidgetItem("正在刷新...")
        item.setFlags(item.flags() & ~QtCore.Qt.ItemIsSelectable)
        self.setItem(0, 0, item)
        self.setItem(0, 1, QtWidgets.QTableWidgetItem(""))
        self.setItem(0, 2, QtWidgets.QTableWidgetItem(""))
        self._loading_row_visible = True

    def _remove_loading_row_if_needed(self):
        if self._loading_row_visible and self.rowCount() > 0:
            self.removeRow(0)
        self._loading_row_visible = False

    @QtCore.Slot(int, list)
    def _on_batch_ready(self, token: int, rows: list):
        if token != self._refresh_token:
            return
        if rows:
            self._pending_rows.extend(rows)
            if not self._flush_timer.isActive():
                self._flush_timer.start(0)

    def _flush_pending_rows(self):
        if not self._pending_rows:
            return
        # 分块写入，给事件循环留出响应窗口，避免一次性重绘卡顿。
        chunk_size = 24
        chunk = self._pending_rows[:chunk_size]
        del self._pending_rows[:chunk_size]
        self.setUpdatesEnabled(False)
        if chunk:
            self._remove_loading_row_if_needed()
        for hwnd, title, process_name in chunk:
            row = self.rowCount()
            self.insertRow(row)
            self.setItem(row, 0, QtWidgets.QTableWidgetItem(str(hwnd)))
            self.setItem(row, 1, QtWidgets.QTableWidgetItem(title))
            self.setItem(row, 2, QtWidgets.QTableWidgetItem(process_name))
        self.setUpdatesEnabled(True)
        if self._pending_rows:
            self._flush_timer.start(0)

    @QtCore.Slot(int, str)
    def _on_refresh_error(self, token: int, message: str):
        if token != self._refresh_token:
            return
        self._pending_rows.clear()
        self.setRowCount(0)
        self._loading_row_visible = False
        self.insertRow(0)
        err = QtWidgets.QTableWidgetItem(f"刷新失败: {message}")
        err.setFlags(err.flags() & ~QtCore.Qt.ItemIsSelectable)
        self.setItem(0, 0, err)
        self.setItem(0, 1, QtWidgets.QTableWidgetItem(""))
        self.setItem(0, 2, QtWidgets.QTableWidgetItem(""))

    @QtCore.Slot(int)
    def _on_refresh_finished(self, token: int):
        if token != self._refresh_token:
            return
        # 清空尾部待处理数据，确保结束时界面状态一致。
        while self._pending_rows:
            self._flush_pending_rows()
        self._remove_loading_row_if_needed()
        self._refresh_worker = None
        self._refresh_thread = None
        self.resizeColumnsToContents()
        self.setSortingEnabled(False)
        self.viewport().update()

    def _on_sel(self):
        rows = self.selectionModel().selectedRows()
        if rows:
            try:
                hwnd = int(self.item(rows[0].row(), 0).text())
            except Exception:
                return
            self.window_selected.emit(hwnd)


class CrosshairWidget(QtWidgets.QWidget):
    """十字准星：按下左键后拖拽到目标窗口释放即可选中。"""
    window_picked = QtCore.Signal(int)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(32, 32)
        self.setCursor(QtCore.Qt.CrossCursor)
        self._dragging = False
        self._timer = QtCore.QTimer(self)
        self._timer.timeout.connect(self._update_target)
        self._cur_hwnd = 0

    def paintEvent(self, _e):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.Antialiasing)
        p.setBrush(QtGui.QBrush(QtGui.QColor(47, 128, 237)))
        p.setPen(QtGui.QPen(QtGui.QColor(255, 255, 255), 2))
        p.drawEllipse(2, 2, 28, 28)
        p.setPen(QtGui.QPen(QtGui.QColor(255, 255, 255), 3))
        p.drawLine(16, 6, 16, 26)
        p.drawLine(6, 16, 26, 16)

    def mousePressEvent(self, e):
        if e.button() == QtCore.Qt.LeftButton:
            self._dragging = True
            self._timer.start(50)
            self.grabMouse()

    def mouseReleaseEvent(self, e):
        if e.button() == QtCore.Qt.LeftButton and self._dragging:
            self._dragging = False
            self._timer.stop()
            self.releaseMouse()
            if self._cur_hwnd:
                self.window_picked.emit(self._cur_hwnd)

    def _update_target(self):
        if not self._dragging:
            return
        pt = wintypes.POINT()
        if user32.GetCursorPos(ctypes.byref(pt)):
            hwnd = user32.WindowFromPoint(pt)
            # 将拾取到的窗口提升为顶层窗口，避免拾取到子控件
            try:
                root = user32.GetAncestor(hwnd, GA_ROOT)
                if root:
                    hwnd = root
            except Exception:
                pass
            if hwnd and hwnd != self._cur_hwnd:
                self._cur_hwnd = hwnd
                try:
                    # 更新拖拽提示：显示进程名而非窗口标题
                    info = WindowInfo(hwnd)
                    exe_name = os.path.basename(info.process_name) if info.process_name else ""
                    tip_proc = exe_name or info.process_name or "未知"
                    self.setToolTip(f"HWND: {hwnd}\n进程: {tip_proc}")
                except Exception:
                    self.setToolTip(f"HWND: {hwnd}")


class HWNDPickerDialog(QtWidgets.QDialog):
    """HWND 获取对话框。"""
    hwnd_selected = QtCore.Signal(int, str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("HWND获取工具")
        self.resize(820, 600)
        self._sel_hwnd = 0
        self._sel_title = ""
        self._init_ui()
        self._connect()
        self.list.refresh_window_list()

    def _init_ui(self):
        v = QtWidgets.QVBoxLayout(self)
        title = QtWidgets.QLabel("选择目标窗口以获取HWND")
        title.setStyleSheet("font-weight: bold; color: #4A9EFF;")
        v.addWidget(title)

        # 工具条
        bar = QtWidgets.QHBoxLayout()
        bar.addWidget(QtWidgets.QLabel("搜索:"))
        self.ed_search = QtWidgets.QLineEdit()
        self.ed_search.setPlaceholderText("输入窗口标题关键词…")
        bar.addWidget(self.ed_search, 1)
        self.btn_refresh = QtWidgets.QPushButton("刷新列表")
        self.btn_refresh.setIcon(self.style().standardIcon(QtWidgets.QStyle.SP_BrowserReload))
        bar.addWidget(self.btn_refresh)
        bar.addWidget(QtWidgets.QLabel("拖拽选择:"))
        self.cross = CrosshairWidget()
        bar.addWidget(self.cross)
        bar.addStretch(1)
        v.addLayout(bar)

        # 窗口列表
        self.list = WindowListWidget()
        v.addWidget(self.list, 1)

        # 详细信息
        grp = QtWidgets.QGroupBox("选中窗口信息")
        form = QtWidgets.QFormLayout(grp)
        self.info_hwnd = QtWidgets.QLineEdit(); self.info_hwnd.setReadOnly(True)
        # “标题”信息改为显示进程名
        self.info_title = QtWidgets.QLineEdit(); self.info_title.setReadOnly(True)
        self.info_class = QtWidgets.QLineEdit(); self.info_class.setReadOnly(True)
        self.info_proc = QtWidgets.QLineEdit(); self.info_proc.setReadOnly(True)
        self.info_size = QtWidgets.QLineEdit(); self.info_size.setReadOnly(True)
        self.chk_ele = QtWidgets.QCheckBox("Electron进程"); self.chk_ele.setEnabled(False)
        form.addRow("HWND:", self.info_hwnd)
        # 标签由“标题”改为“进程名”
        form.addRow("进程名:", self.info_title)
        form.addRow("类名:", self.info_class)
        form.addRow("进程:", self.info_proc)
        form.addRow("尺寸:", self.info_size)
        form.addRow("", self.chk_ele)
        v.addWidget(grp)

        # 按钮区
        h = QtWidgets.QHBoxLayout()
        self.btn_test = QtWidgets.QPushButton("测试捕获")
        self.btn_ok = QtWidgets.QPushButton("确定"); self.btn_ok.setObjectName("primary"); self.btn_ok.setEnabled(False)
        self.btn_cancel = QtWidgets.QPushButton("取消")
        h.addWidget(self.btn_test)
        h.addStretch(1)
        h.addWidget(self.btn_ok)
        h.addWidget(self.btn_cancel)
        v.addLayout(h)

    def _connect(self):
        self.btn_refresh.clicked.connect(self.list.refresh_window_list)
        self.ed_search.textChanged.connect(self._on_search)
        self.cross.window_picked.connect(self._on_pick)
        self.list.window_selected.connect(self._on_select)
        self.btn_ok.clicked.connect(self.accept)
        self.btn_cancel.clicked.connect(self.reject)
        self.btn_test.clicked.connect(self._on_test)

    def _on_search(self, text: str):
        for r in range(self.list.rowCount()):
            item = self.list.item(r, 1)
            ok = (text.lower() in item.text().lower()) if text else True
            self.list.setRowHidden(r, not ok)

    def _on_pick(self, hwnd: int):
        self._select(hwnd)

    def _on_select(self, hwnd: int):
        self._select(hwnd)

    def _select(self, hwnd: int):
        try:
            # 将传入的窗口句柄提升为顶层窗口，确保一致性
            try:
                root = user32.GetAncestor(hwnd, GA_ROOT)
                if root:
                    hwnd = root
            except Exception:
                pass

            info = WindowInfo(hwnd)
            self._sel_hwnd = hwnd
            # 进程“相对路径”指 exe 文件名，用于配置中的 target_process 更稳健匹配
            exe_name = os.path.basename(info.process_name) if info.process_name else ""
            # 将返回值的第二项调整为 exe 名称（相对进程路径）
            self._sel_title = exe_name or info.process_name
            self.info_hwnd.setText(str(hwnd))
            # 信息面板：“进程名”显示 exe 名称；“进程”显示完整路径
            self.info_title.setText(exe_name or info.process_name)
            self.info_class.setText(info.class_name)
            self.info_proc.setText(info.process_name)
            if info.rect:
                self.info_size.setText(f"{info.rect['width']} × {info.rect['height']}")
            else:
                self.info_size.setText("未知")
            self.chk_ele.setChecked(info.is_electron)
            self.btn_ok.setEnabled(True)
            # 实时向外发射信号，便于设置窗口同步回填
            try:
                self.hwnd_selected.emit(self._sel_hwnd, self._sel_title)
            except Exception:
                pass
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, "错误", f"获取窗口信息失败: {e}")

    def _on_test(self):
        """使用后台线程测试窗口捕获，彻底避免主线程卡顿。"""
        if not self._sel_hwnd:
            QtWidgets.QMessageBox.warning(self, "提示", "请先选择窗口")
            return
        try:
            from auto_approve.qt_workers import CaptureTestWorker, WindowCaptureParams
            from auto_approve.settings_dialog import ScreenshotPreviewDialog
            import cv2, time

            progress = QtWidgets.QProgressDialog("正在测试窗口捕获...", "取消", 0, 100, self)
            progress.setWindowModality(QtCore.Qt.WindowModal)
            progress.setAutoClose(False)
            progress.setAutoReset(False)
            progress.setValue(0)
            progress.show()

            thread = QtCore.QThread(self)
            worker = CaptureTestWorker()
            worker.moveToThread(thread)

            worker.sig_progress.connect(lambda v, t: (progress.setValue(v), progress.setLabelText(t)))

            def _on_finished(img, meta):
                try:
                    progress.setValue(100)
                    progress.close()
                    # 展示静态图像预览
                    h, w = img.shape[:2]
                    rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
                    qimg = QtGui.QImage(rgb.data, w, h, rgb.strides[0], QtGui.QImage.Format_RGB888)
                    pm = QtGui.QPixmap.fromImage(qimg)
                    dlg = ScreenshotPreviewDialog(pm, self, is_wgc_test=True)
                    dlg.setWindowTitle("WGC捕获测试结果")
                    if dlg.exec() == QtWidgets.QDialog.Accepted:
                        from storage import init_db, save_image_blob
                        name = f"wgc_test_capture_{int(time.time())}.png"
                        ok, buf = cv2.imencode('.png', img, [int(cv2.IMWRITE_PNG_COMPRESSION), 6])
                        if ok:
                            init_db()
                            save_image_blob(name, buf.tobytes(), category="export", size=(w, h))
                            QtWidgets.QMessageBox.information(self, "已保存", f"图片保存到数据库: db://export/{name}")
                        else:
                            QtWidgets.QMessageBox.warning(self, "保存失败", "无法编码图片数据")
                finally:
                    thread.quit()

            def _on_error(msg: str):
                try:
                    progress.close()
                    QtWidgets.QMessageBox.warning(self, "测试失败", msg)
                finally:
                    thread.quit()

            def _on_canceled():
                try:
                    progress.close()
                finally:
                    thread.quit()

            worker.sig_finished.connect(_on_finished)
            worker.sig_error.connect(_on_error)
            worker.sig_canceled.connect(_on_canceled)

            progress.canceled.connect(worker.cancel)

            params = WindowCaptureParams(
                hwnd=int(self._sel_hwnd),
                fps=10,
                include_cursor=False,
                border_required=False,
                restore_minimized=True,
                timeout_sec=2.0,
                stabilize_ms=500,
            )

            thread.started.connect(lambda: worker.run_window_test(params))
            thread.finished.connect(worker.deleteLater)
            thread.finished.connect(thread.deleteLater)
            thread.start()

        except Exception as e:
            QtWidgets.QMessageBox.critical(self, "错误", f"测试捕获出错: {e}")

    def get_selected_hwnd(self) -> Tuple[int, str]:
        return self._sel_hwnd, self._sel_title
