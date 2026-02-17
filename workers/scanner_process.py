# -*- coding: utf-8 -*-
"""
独立扫描进程模块

将扫描功能完全独立为进程，避免UI卡顿：
- 使用multiprocessing.Process实现完全独立的扫描进程
- 通过Queue进行进程间通信
- 支持配置动态更新
- 提供状态监控和错误处理
"""

import os
import sys
import time
import threading
import uuid
import pickle
import traceback
import multiprocessing as mp
from typing import Any, Dict, List, Tuple, Optional, Union, Callable
import ctypes
from ctypes import wintypes
from dataclasses import dataclass, asdict
from queue import Empty
import numpy as np
import cv2

from PySide6 import QtCore
from PySide6.QtCore import QObject, Signal, QTimer

from auto_approve.config_manager import AppConfig
from auto_approve.logger_manager import get_logger
from auto_approve.win_clicker import post_click_with_config, post_click_in_window_with_config
from capture.capture_manager import CaptureManager
from capture.monitor_utils import get_monitor_info
from utils.win_dpi import set_process_dpi_awareness, get_dpi_info_summary
from auto_approve.path_utils import get_app_base_dir


# 确保Windows平台使用spawn方式启动进程
if sys.platform.startswith('win'):
    mp.set_start_method('spawn', force=True)


@dataclass
class ScannerCommand:
    """扫描器命令"""
    command: str  # 'start', 'stop', 'update_config', 'get_status'
    data: Any = None
    timestamp: float = 0.0


@dataclass
class ScannerStatus:
    """扫描器状态"""
    running: bool = False
    status_text: str = ""
    backend: str = ""
    detail: str = ""
    scan_count: int = 0
    error_message: str = ""
    timestamp: float = 0.0


@dataclass
class ScannerHit:
    """扫描命中结果"""
    score: float
    x: int
    y: int
    timestamp: float


class ScannerProcessSignals(QObject):
    """扫描进程信号类"""
    # 状态更新信号
    status_updated = Signal(object)  # ScannerStatus
    # 命中信号
    hit_detected = Signal(object)   # ScannerHit
    # 日志信号
    log_message = Signal(str)
    # 错误信号
    error_occurred = Signal(str)
    # 进度更新信号
    progress_updated = Signal(str)


def _compute_window_open_plan(cfg: AppConfig) -> list:
    """根据配置生成窗口打开尝试顺序（纯逻辑函数，便于测试）。

    返回值示例：
    [
        ("hwnd", 123456),
        ("title", "Code"),
        ("process", "Code.exe"),
    ]
    """
    plan = []
    try:
        hwnd = int(getattr(cfg, 'target_hwnd', 0))
    except Exception:
        hwnd = 0
    title = getattr(cfg, 'target_window_title', '') or getattr(cfg, 'window_title', '')
    proc = getattr(cfg, 'target_process', '')

    # 先尝试HWND，失败时允许回退
    if hwnd > 0:
        plan.append(("hwnd", hwnd))
        # 若同时提供标题/进程名，作为回退项
        if title:
            plan.append(("title", title))
        if proc:
            plan.append(("process", proc))
    else:
        # 无有效HWND，直接走标题/进程名
        if title:
            plan.append(("title", title))
        if proc:
            plan.append(("process", proc))

    return plan


def _resolve_template_path(p: str) -> str:
    """解析模板路径为绝对路径（文件路径分支）。SQLite模式不再使用 assets/images 兜底。"""
    try:
        if not p:
            return ""
        p = os.path.normpath(p)
        # 绝对路径直接使用
        if os.path.isabs(p) and os.path.exists(p):
            return p

        base_dir = get_app_base_dir()

        # 基于应用目录解析相对路径
        candidate = os.path.join(base_dir, p)
        if os.path.exists(candidate):
            return candidate

        # 工作目录兜底
        wd_path = os.path.abspath(os.path.join(os.getcwd(), p))
        if os.path.exists(wd_path):
            return wd_path

        return p
    except Exception:
        return p


def _load_templates_from_paths(template_paths: List[str]) -> List[Tuple[np.ndarray, Tuple[int, int]]]:
    """加载模板图像 - 使用内存模板管理器避免磁盘IO"""
    try:
        # 导入内存模板管理器
        import sys
        import os
        sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        from utils.memory_template_manager import get_template_manager

        # 统一解析模板路径为绝对路径，避免相对路径在子进程中失效
        resolved_paths = []
        for p in template_paths:
            if isinstance(p, str) and p.startswith('db://'):
                # 数据库引用保持原样传递给模板管理器
                resolved_paths.append(p)
            else:
                rp = _resolve_template_path(p)
                resolved_paths.append(rp)

        # 获取模板管理器并加载模板（内存缓存）
        template_manager = get_template_manager()
        template_manager.load_templates(resolved_paths)

        # 从内存获取模板数据
        templates = template_manager.get_templates(resolved_paths)

        print(f"从内存加载了 {len(templates)} 个模板")
        return templates

    except Exception as e:
        print(f"内存模板管理器加载失败，回退到传统方式: {e}")

        # 回退到传统的磁盘加载方式
        templates = []
        for path in template_paths:
            try:
                if os.path.exists(path):
                    # 使用cv2.imdecode处理中文路径
                    img_data = np.fromfile(path, dtype=np.uint8)
                    template = cv2.imdecode(img_data, cv2.IMREAD_COLOR)
                    if template is not None:
                        h, w = template.shape[:2]
                        templates.append((template, (w, h)))
            except Exception as e:
                print(f"加载模板失败 {path}: {e}")
        return templates


def _template_matching(roi_img: np.ndarray, templates: List[Tuple[np.ndarray, Tuple[int, int]]], 
                      threshold: float, grayscale: bool) -> Tuple[float, int, int, int, int]:
    """模板匹配：返回分数、最佳位置以及模板宽高（用于中心点击）。"""
    best_score = 0.0
    best_x, best_y = 0, 0
    best_w, best_h = 0, 0
    
    # 转换为灰度图（如果需要）
    if grayscale and len(roi_img.shape) == 3:
        roi_gray = cv2.cvtColor(roi_img, cv2.COLOR_BGR2GRAY)
    else:
        roi_gray = roi_img
    
    for template, (tw, th) in templates:
        # 转换模板为灰度图（如果需要）
        if grayscale and len(template.shape) == 3:
            template_gray = cv2.cvtColor(template, cv2.COLOR_BGR2GRAY)
        else:
            template_gray = template
        
        # 模板匹配
        result = cv2.matchTemplate(roi_gray, template_gray, cv2.TM_CCOEFF_NORMED)
        _, max_val, _, max_loc = cv2.minMaxLoc(result)
        
        if max_val > best_score:
            best_score = max_val
            best_x, best_y = max_loc
            best_w, best_h = tw, th
    
    return best_score, best_x, best_y, best_w, best_h


def _scanner_worker_process(command_queue: mp.Queue, status_queue: mp.Queue,
                           hit_queue: mp.Queue, log_queue: mp.Queue):
    """扫描器工作进程"""
    logger = get_logger()
    logger.info("扫描器工作进程启动")

    try:
        # 设置 DPI 感知
        logger.info("设置DPI感知...")
        set_process_dpi_awareness()
        logger.info("DPI感知设置完成")

        # 进程状态
        running = False
        cfg: Optional[AppConfig] = None
        capture_manager: Optional[CaptureManager] = None
        templates: List[Tuple[np.ndarray, Tuple[int, int]]] = []
        scan_count = 0
        consecutive_clicks = 0
        next_click_allowed = 0.0

        logger.info("扫描器工作进程初始化完成")

    except Exception as e:
        logger.error(f"扫描器工作进程初始化失败: {e}")
        return
    
    def send_status(status_text: str = "", backend: str = "", detail: str = "", error: str = ""):
        """发送状态更新"""
        status = ScannerStatus(
            running=running,
            status_text=status_text,
            backend=backend,
            detail=detail,
            scan_count=scan_count,
            error_message=error,
            timestamp=time.time()
        )
        try:
            status_queue.put_nowait(status)
        except:
            pass
    
    def send_log(message: str):
        """发送日志消息"""
        try:
            log_queue.put_nowait(message)
        except:
            pass
    
    def send_hit(score: float, x: int, y: int):
        """发送命中结果"""
        hit = ScannerHit(score=score, x=x, y=y, timestamp=time.time())
        try:
            hit_queue.put_nowait(hit)
        except:
            pass
    
    def init_capture_manager():
        """初始化捕获管理器（适配新版 CaptureManager API）

        关键改进：
        1) 当配置包含失效的 HWND 时，自动回退到按标题/按进程名查找，避免仅因一个失效句柄而失败；
        2) 在关键阶段通过状态队列发送进度，让托盘不再长期停留在“正在创建扫描进程…”。
        """
        nonlocal capture_manager
        try:
            if cfg is None:
                send_log("配置为空，无法初始化捕获管理器")
                return False

            send_log("开始初始化捕获管理器...")
            # 提前投递一次状态，避免UI长时间无反馈
            send_status("启动中", "进程扫描", "正在初始化捕获管理器...")

            # 创建管理器并配置参数
            capture_manager = CaptureManager()
            send_log("捕获管理器对象创建成功")

            fps = int(getattr(cfg, 'fps_max', getattr(cfg, 'target_fps', 30)))
            include_cursor = bool(getattr(cfg, 'include_cursor', False))
            # 根据模式分别读取边框开关（兼容旧字段）
            use_monitor = bool(getattr(cfg, 'use_monitor', False))
            border_required = bool(
                getattr(cfg, 'screen_border_required' if use_monitor else 'window_border_required',
                        getattr(cfg, 'border_required', False))
            )
            restore_minimized = bool(getattr(cfg, 'restore_minimized_after_capture', False))

            send_log(f"配置参数: fps={fps}, cursor={include_cursor}, border={border_required}, monitor={use_monitor}")

            capture_manager.configure(
                fps=fps,
                include_cursor=include_cursor,
                border_required=border_required,
                restore_minimized=restore_minimized
            )
            send_log("捕获管理器配置完成")
            if use_monitor:
                # 打开显示器捕获（monitor_index 按 0 基准处理）
                monitor_index = int(getattr(cfg, 'monitor_index', 0))
                if not capture_manager.open_monitor(monitor_index):
                    send_log("显示器捕获初始化失败")
                    return False
            else:
                # 选择目标窗口：优先 hwnd，其次标题，再次进程名
                target_hwnd = int(getattr(cfg, 'target_hwnd', 0))
                target_title = getattr(cfg, 'target_window_title', '') or getattr(cfg, 'window_title', '')
                partial = bool(getattr(cfg, 'window_title_partial_match', True))
                target_proc = getattr(cfg, 'target_process', '')

                opened = False
                if target_hwnd > 0:
                    # 先按HWND尝试
                    send_status("启动中", "进程扫描", f"正在按HWND初始化: {target_hwnd}")
                    opened = capture_manager.open_window(target_hwnd, async_init=True)
                    # 若HWND失败且提供了标题/进程名，则自动回退
                    if (not opened) and (target_title or target_proc):
                        send_log("HWND初始化失败，尝试按标题/进程名回退")
                        send_status("启动中", "进程扫描", "HWND无效，回退按标题/进程名")
                        if target_title:
                            opened = capture_manager.open_window(target_title, partial_match=partial, async_init=True)
                        if (not opened) and target_proc:
                            opened = capture_manager.open_window(target_proc, partial_match=True, async_init=True)
                else:
                    # 没有有效HWND，直接尝试标题/进程名
                    if target_title:
                        send_status("启动中", "进程扫描", f"按标题查找: {target_title}")
                        opened = capture_manager.open_window(target_title, partial_match=partial, async_init=True)
                    if (not opened) and target_proc:
                        send_status("启动中", "进程扫描", f"按进程查找: {target_proc}")
                        opened = capture_manager.open_window(target_proc, partial_match=True, async_init=True)

                if not opened:
                    send_log("窗口捕获初始化失败：请检查 target_hwnd/target_window_title/target_process 配置")
                    send_status("启动失败", "进程扫描", "无法找到有效窗口", "初始化失败")
                    return False

            return True
        except Exception as e:
            send_log(f"捕获管理器初始化异常: {e}")
            # 将异常同步到状态，便于UI及时显示
            try:
                send_status("启动失败", "进程扫描", f"异常: {e}", "初始化失败")
            except Exception:
                pass
            return False

    def cleanup_capture_manager():
        """清理捕获管理器（新版使用 close）"""
        nonlocal capture_manager
        if capture_manager:
            try:
                capture_manager.close()
            except Exception as e:
                send_log(f"清理捕获管理器异常: {e}")
            finally:
                capture_manager = None

    def load_templates():
        """加载模板"""
        nonlocal templates
        if cfg is None:
            return
        
        # 读取多模板列表，若为空则回退到单模板字段，确保至少有一个模板
        template_paths = getattr(cfg, 'template_paths', [])
        if not template_paths:
            single_path = getattr(cfg, 'template_path', '')
            if single_path:
                template_paths = [single_path]
        
        templates = _load_templates_from_paths(template_paths)
        send_log(f"加载了 {len(templates)} 个模板")
    
    def apply_roi_to_image(img: np.ndarray) -> Tuple[np.ndarray, int, int]:
        """应用ROI到图像"""
        if cfg is None:
            return img, 0, 0
        
        roi = getattr(cfg, 'roi', None)
        h, w = img.shape[:2]

        # 兼容多种ROI格式：
        # - dataclass ROI(x,y,w,h)
        # - dict {left, top, right, bottom}
        # - 序列 [left, top, right, bottom]
        if roi is None:
            return img, 0, 0

        left = top = 0
        right = w
        bottom = h
        try:
            if hasattr(roi, 'x') and hasattr(roi, 'y') and hasattr(roi, 'w') and hasattr(roi, 'h'):
                # dataclass ROI
                left = int(getattr(roi, 'x', 0))
                top = int(getattr(roi, 'y', 0))
                rw = int(getattr(roi, 'w', 0))
                rh = int(getattr(roi, 'h', 0))
                right = left + (rw if rw > 0 else (w - left))
                bottom = top + (rh if rh > 0 else (h - top))
            elif isinstance(roi, dict):
                left = int(roi.get('left', 0))
                top = int(roi.get('top', 0))
                right = int(roi.get('right', w))
                bottom = int(roi.get('bottom', h))
            elif isinstance(roi, (list, tuple)) and len(roi) == 4:
                left, top, right, bottom = map(int, roi)
            else:
                # 未知格式，使用全图
                return img, 0, 0
        except Exception:
            # 解析失败，使用全图
            return img, 0, 0

        # 边界裁剪
        left = max(0, min(left, w))
        top = max(0, min(top, h))
        right = max(left, min(right, w))
        bottom = max(top, min(bottom, h))

        roi_img = img[top:bottom, left:right]
        return roi_img, left, top

    # --- 坐标换算辅助（WGC窗口内容像素 -> 客户端坐标） ---
    class _POINT(ctypes.Structure):
        _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]

    class _RECT(ctypes.Structure):
        _fields_ = [("left", wintypes.LONG), ("top", wintypes.LONG), ("right", wintypes.LONG), ("bottom", wintypes.LONG)]

    _user32 = ctypes.WinDLL('user32', use_last_error=True)
    _user32.GetClientRect.restype = wintypes.BOOL
    _user32.GetClientRect.argtypes = [wintypes.HWND, ctypes.POINTER(_RECT)]

    def _get_client_size(hwnd: int) -> Tuple[int, int]:
        """读取窗口客户区宽高，失败返回(0,0)。"""
        try:
            rc = _RECT()
            if _user32.GetClientRect(hwnd, ctypes.byref(rc)):
                return int(rc.right - rc.left), int(rc.bottom - rc.top)
        except Exception:
            pass
        return 0, 0

    def _scale_capture_to_client(x: float, y: float, stats: Dict[str, Any], hwnd: int) -> Tuple[int, int]:
        """将以WGC内容像素计的坐标缩放到窗口客户区坐标，避免缩放错位。"""
        try:
            content_size = stats.get('content_size')  # (w,h) 或 None
            if not content_size or not isinstance(content_size, (list, tuple)):
                return int(round(x)), int(round(y))
            cw, ch = int(content_size[0] or 0), int(content_size[1] or 0)
            if cw <= 0 or ch <= 0:
                return int(round(x)), int(round(y))
            gw, gh = _get_client_size(hwnd)
            if gw <= 0 or gh <= 0:
                return int(round(x)), int(round(y))
            sx = gw / cw
            sy = gh / ch
            return int(round(x * sx)), int(round(y * sy))
        except Exception:
            return int(round(x)), int(round(y))

    def scan_and_maybe_click() -> float:
        """执行扫描和点击"""
        nonlocal scan_count, consecutive_clicks, next_click_allowed
        
        if not templates or not capture_manager or cfg is None:
            return 0.0
        
        # 捕获帧（优先使用共享帧缓存）
        img = capture_manager.get_shared_frame("scanner_detection", "detection")
        if img is None:
            # 如果共享缓存没有，使用传统捕获
            restore_after = getattr(cfg, 'restore_minimized_after_capture', False)
            img = capture_manager.capture_frame(restore_after_capture=restore_after)

        if img is None:
            return 0.0
        
        # 应用ROI
        roi_img, roi_left, roi_top = apply_roi_to_image(img)
        
        # 模板匹配
        score, match_x, match_y, tpl_w, tpl_h = _template_matching(
            roi_img, templates, cfg.threshold, cfg.grayscale
        )
        
        scan_count += 1
        
        # 检查是否命中
        if score >= cfg.threshold:
            # 计算候选坐标（以ROI左上为基准，点选模板中心）
            raw_x = roi_left + match_x + (tpl_w // 2) + cfg.click_offset[0]
            raw_y = roi_top + match_y + (tpl_h // 2) + cfg.click_offset[1]

            # 判断捕获模式
            stats = capture_manager.get_stats() if capture_manager else {}
            target_hwnd = stats.get('target_hwnd')

            # 点击控制逻辑
            current_time = time.time()
            if current_time >= next_click_allowed:
                try:
                    if target_hwnd:
                        # 窗口捕获：根据内容尺寸与客户区尺寸做自适应缩放，确保精准点击
                        stats = capture_manager.get_stats() if capture_manager else {}
                        cx, cy = _scale_capture_to_client(raw_x, raw_y, stats, int(target_hwnd))
                        success = post_click_in_window_with_config(int(target_hwnd), int(cx), int(cy), cfg)
                        click_log_pos = f"client({cx},{cy}) hwnd={target_hwnd}"
                    else:
                        # 显示器捕获：raw_x/raw_y 是屏幕坐标
                        success = post_click_with_config(int(raw_x), int(raw_y), cfg)
                        click_log_pos = f"screen({raw_x},{raw_y})"

                    if success:
                        consecutive_clicks += 1
                        # 发送命中信号（统一传屏幕坐标更直观）
                        send_hit(score, int(raw_x), int(raw_y))

                        # 计算下次点击时间（容错：若无 click_delay_ms 则使用 cooldown_s）
                        base_delay = getattr(cfg, 'click_delay_ms', None)
                        if base_delay is None:
                            base_delay = getattr(cfg, 'cooldown_s', 1.0) * 1000.0
                        base_delay = float(base_delay) / 1000.0
                        adaptive_delay = base_delay * (1 + consecutive_clicks * 0.1)
                        next_click_allowed = current_time + adaptive_delay

                        send_log(f"点击成功 {click_log_pos}, 置信度: {score:.3f}")
                    else:
                        send_log(f"点击失败 {click_log_pos}, 置信度: {score:.3f}")

                except Exception as e:
                    send_log(f"点击失败: {e}")
            else:
                send_log(f"点击被限制，等待 {next_click_allowed - current_time:.1f}s")
        else:
            # 重置连续点击计数
            if consecutive_clicks > 0:
                consecutive_clicks = max(0, consecutive_clicks - 1)
        
        return score
    
    # 主循环
    try:
        # 状态上报节流：避免过于频繁地通过队列向主进程发送状态，造成UI线程负载过高
        last_status_emit = 0.0
        status_emit_interval = 0.2  # 至少间隔200ms上报一次状态
        last_backend_label = ""

        # 循环最小让步：配置 interval_ms=0 时也避免背靠背自旋
        min_loop_yield_s = 0.001
        # 下一次扫描时刻（monotonic），None 表示当前不在扫描态
        next_scan_at: Optional[float] = None

        def _get_scan_interval_s() -> float:
            """计算扫描间隔（秒），确保至少有最小让步。"""
            interval_s = 0.0
            if cfg is not None:
                interval_s = max(0.0, float(getattr(cfg, 'interval_ms', 0)) / 1000.0)
            return max(interval_s, min_loop_yield_s)

        def _handle_command(command: ScannerCommand) -> bool:
            """处理命令。返回 True 表示需要退出主循环。"""
            nonlocal running, cfg, scan_count, consecutive_clicks, next_click_allowed, next_scan_at

            if command.command == 'start':
                if not running:
                    cfg = command.data
                    if init_capture_manager():
                        load_templates()
                        running = True
                        scan_count = 0
                        consecutive_clicks = 0
                        next_click_allowed = 0.0
                        # 启动后立即允许首帧扫描，避免额外等待一个 interval
                        next_scan_at = time.monotonic()
                        send_status("运行中", "进程扫描", "正在初始化...")
                        send_log("扫描进程已启动")
                    else:
                        send_status("", "", "", "初始化失败")

            elif command.command == 'stop':
                if running:
                    running = False
                    next_scan_at = None
                    cleanup_capture_manager()
                    send_status("已停止", "", "")
                    send_log("扫描进程已停止")

            elif command.command == 'update_config':
                cfg = command.data
                if running:
                    cleanup_capture_manager()
                    if init_capture_manager():
                        load_templates()
                        # 配置更新后立即按新参数进入下一次扫描
                        next_scan_at = time.monotonic()
                        send_log("配置已更新")
                    else:
                        running = False
                        next_scan_at = None
                        send_status("", "", "", "配置更新失败")

            elif command.command == 'exit':
                return True

            return False

        while True:
            # 空闲态：完全阻塞等待命令，避免空转轮询
            if not running:
                try:
                    command: ScannerCommand = command_queue.get()
                    if _handle_command(command):
                        break
                except Exception as e:
                    send_log(f"命令处理异常: {e}")
                    time.sleep(min_loop_yield_s)
                continue

            # 运行态：等待“下一次扫描时刻”或“新命令”，两者谁先到就处理谁
            if next_scan_at is None:
                next_scan_at = time.monotonic()

            wait_s = max(0.0, next_scan_at - time.monotonic())
            try:
                command = command_queue.get(timeout=wait_s)
                if _handle_command(command):
                    break
                continue
            except Empty:
                # 超时说明到达扫描时刻，进入扫描流程
                pass
            except Exception as e:
                send_log(f"命令处理异常: {e}")
                time.sleep(min_loop_yield_s)
                continue

            # 执行扫描
            try:
                score = scan_and_maybe_click()

                # 更新状态（带节流）
                backend = "WGC 窗口捕获" if not getattr(cfg, 'use_monitor', False) else "WGC 显示器捕获"
                detail = f"上次匹配: {score:.3f}"

                now = time.time()
                # 重要状态变化或后端切换应即时上报，其余按节流间隔上报
                backend_changed = (backend != last_backend_label)
                if backend_changed or (now - last_status_emit >= status_emit_interval):
                    send_status("运行中", backend, detail)
                    last_status_emit = now
                    last_backend_label = backend

            except Exception as e:
                send_log(f"扫描异常: {e}")
                logger.exception("扫描异常")

            # 仅在仍处于运行态时更新下一次扫描时刻
            if running:
                next_scan_at = time.monotonic() + _get_scan_interval_s()
                
    except KeyboardInterrupt:
        send_log("扫描进程被中断")
    except Exception as e:
        send_log(f"扫描进程异常: {e}")
        logger.exception("扫描进程异常")
    finally:
        cleanup_capture_manager()
        send_log("扫描进程退出")


class ScannerProcessManager(QObject):
    """扫描进程管理器"""
    
    def __init__(self):
        super().__init__()
        self._logger = get_logger()
        self.signals = ScannerProcessSignals()
        
        # 进程和队列
        self._process: Optional[mp.Process] = None
        self._command_queue: Optional[mp.Queue] = None
        self._status_queue: Optional[mp.Queue] = None
        self._hit_queue: Optional[mp.Queue] = None
        self._log_queue: Optional[mp.Queue] = None
        
        # 状态轮询定时器 - 自适应轮询机制
        self._poll_timer = QTimer()
        self._poll_timer.timeout.connect(self._poll_queues)

        # 自适应轮询配置
        self._base_poll_interval = 100   # 基础轮询间隔100ms
        self._min_poll_interval = 50     # 最小轮询间隔50ms（活跃时）
        self._max_poll_interval = 1200   # 最大轮询间隔1200ms（深度空闲时）
        self._current_poll_interval = self._base_poll_interval
        self._poll_timer.setInterval(self._current_poll_interval)

        # 轮询自适应统计
        self._poll_stats = {
            'empty_polls': 0,                 # 连续空轮询次数
            'active_polls': 0,                # 连续活跃轮询次数
            'last_activity': time.time()      # 上次活动时间
        }

        # 轮询预算参数（常量前置，减少 _poll_queues 中的重复构造开销）
        self._max_status_per_tick = 5
        self._max_hit_per_tick = 10
        self._max_log_per_tick = 20
        self._max_tick_budget_s = 0.008
        # 各队列空转退避：通过跳帧减少连续空取，避免空轮询异常开销
        self._queue_backoff = {
            'status': {'empty_streak': 0, 'skip_ticks': 0, 'max_skip': 1},
            'hit': {'empty_streak': 0, 'skip_ticks': 0, 'max_skip': 1},
            'log': {'empty_streak': 0, 'skip_ticks': 0, 'max_skip': 2},
        }

        # 当前状态
        self._running = False
        self._current_config: Optional[AppConfig] = None

        # 启动稳健性：就绪握手与看门狗
        # _ready 表示是否已从子进程收到“running=True”的状态，作为就绪握手
        self._ready: bool = False
        # 启动看门狗定时器，到期未就绪则判定为“启动卡死/超时”并尝试重启
        self._startup_watchdog = QTimer()
        self._startup_watchdog.setSingleShot(True)
        self._startup_watchdog.timeout.connect(self._on_startup_timeout)
        # 看门狗初始超时（毫秒）和最大重试次数
        self._startup_timeout_ms: int = 8000
        self._startup_attempt: int = 0
        self._max_startup_attempts: int = 3
        # 指数回退上限（毫秒）
        self._max_backoff_ms: int = 12000

        # 生命周期与并发控制
        self._session_token: int = 0
        self._state_lock = threading.Lock()
        self._cleanup_lock = threading.Lock()
        # 无事件循环时使用线程定时器兜底，避免回调永久不执行
        self._fallback_timers: List[threading.Timer] = []
        self._fallback_timers_lock = threading.Lock()
        
        self._logger.info("扫描进程管理器初始化完成")

    def start_scanning(self, cfg: AppConfig) -> bool:
        """启动扫描进程 - 使用线程化启动避免阻塞GUI"""
        # 修复：当 stop 后短时间内再次 start 时，可能出现 _running=True 但进程已不在/已被清理，
        # 导致上层始终停留在“正在创建扫描进程…”。这里改为更稳健的判断：
        # 1) 若实际仍在运行（进程活着），直接返回；
        # 2) 若仅有残留标志（_running=True 但进程对象为空或已退出），执行一次强制清理后继续启动。
        if self._running:
            if self._process is not None and self._process.is_alive():
                self._logger.warning("扫描进程已在运行")
                return True
            else:
                self._logger.warning("检测到残留运行状态但进程未存活，执行强制清理并重新启动")
                # 确保彻底清理残留资源，避免新的启动卡住
                try:
                    self._cleanup_process()
                except Exception as e:
                    self._logger.debug(f"残留清理异常: {e}")

        try:
            self._logger.info("开始创建扫描进程...")

            # 进入新会话：失效掉旧会话的延迟回调，避免误清理新进程。
            with self._state_lock:
                self._session_token += 1
            self._cancel_fallback_timers()
            
            # 发送进度更新
            try:
                self.signals.progress_updated.emit("正在创建扫描进程...")
            except Exception:
                pass

            # 创建队列
            self._command_queue = mp.Queue()
            self._status_queue = mp.Queue()
            self._hit_queue = mp.Queue()
            self._log_queue = mp.Queue()
            self._logger.info("队列创建完成")
            
            # 发送进度更新
            try:
                self.signals.progress_updated.emit("队列创建完成，正在创建进程对象...")
            except Exception:
                pass

            # 创建进程
            self._process = mp.Process(
                target=_scanner_worker_process,
                args=(self._command_queue, self._status_queue, self._hit_queue, self._log_queue),
                daemon=True
            )
            self._logger.info("进程对象创建完成")
            
            # 发送进度更新
            try:
                self.signals.progress_updated.emit("进程对象创建完成，正在启动进程...")
            except Exception:
                pass

            # 使用线程启动进程，完全避免阻塞主线程
            from workers.io_tasks import submit_io, IOTaskBase

            class ProcessStartTask(IOTaskBase):
                def __init__(self, manager, cfg):
                    super().__init__("process_start")
                    self.manager = manager
                    self.cfg = cfg

                def execute(self):
                    return self.manager._threaded_start_process(self.cfg)

            task = ProcessStartTask(self, cfg)
            submit_io(task,
                     on_success=self._on_process_started,
                     on_error=self._on_process_start_error)

            self._logger.info("进程启动任务已提交到IO线程池")
            
            # 发送进度更新
            try:
                self.signals.progress_updated.emit("进程启动任务已提交，正在等待进程启动...")
            except Exception:
                pass

            # 重置就绪握手与看门狗并启动
            self._ready = False
            self._startup_attempt += 1  # 记录尝试次数
            # 每次尝试按指数回退扩大超时窗口，避免慢机/首次加载过早判定失败
            dynamic_timeout = min(
                int(self._startup_timeout_ms * (1.6 ** (self._startup_attempt - 1))),
                self._max_backoff_ms
            )
            try:
                self._startup_watchdog.start(dynamic_timeout)
                self._logger.info(f"启动看门狗已开启，超时: {dynamic_timeout}ms，尝试次数: {self._startup_attempt}")
            except Exception as e:
                self._logger.debug(f"启动看门狗启动失败: {e}")

            return True

        except Exception as e:
            self._logger.error(f"启动扫描进程失败: {e}")
            self._cleanup_process()
            return False

    def _threaded_start_process(self, cfg: AppConfig):
        """在IO线程中启动进程"""
        try:
            self._logger.info("IO线程中开始启动进程...")

            # 启动进程（在IO线程中执行，不阻塞GUI）
            import time
            start_time = time.time()

            self._process.start()
            startup_time = time.time() - start_time

            pid = self._process.pid
            self._logger.info(f"进程启动成功，PID: {pid}，耗时: {startup_time:.3f}秒")

            return {"success": True, "pid": pid, "cfg": cfg, "startup_time": startup_time}

        except Exception as e:
            self._logger.error(f"IO线程中启动进程失败: {e}")
            import traceback
            self._logger.debug(f"详细错误: {traceback.format_exc()}")
            return {"success": False, "error": str(e)}

    def _on_process_started(self, task_id: str, result):
        """进程启动成功回调（在主线程中执行）"""
        if result.get("success"):
            pid = result.get("pid")
            cfg = result.get("cfg")
            startup_time = result.get("startup_time", 0)

            self._logger.info(f"扫描进程已启动 (PID: {pid}, 启动耗时: {startup_time:.3f}秒)")
            
            # 发送进度更新
            try:
                self.signals.progress_updated.emit(f"进程已启动 (PID: {pid})，正在初始化轮询...")
            except Exception:
                pass

            # 启动轮询前重置节奏，避免沿用上次会话的空闲大间隔
            self._reset_polling_state()
            self._poll_timer.start()
            self._logger.info("状态轮询已启动")
            
            # 发送进度更新
            try:
                self.signals.progress_updated.emit("状态轮询已启动，正在准备发送启动命令...")
            except Exception:
                pass

            # 立即投递一次"启动中"状态，避免UI长时间停留在"正在创建扫描进程..."
            try:
                bootstrap_status = ScannerStatus(
                    running=True,
                    status_text="启动中",
                    backend="进程扫描",
                    detail="进程已创建，准备发送启动命令",
                    scan_count=0,
                    error_message="",
                    timestamp=time.time()
                )
                self.signals.status_updated.emit(bootstrap_status)
            except Exception:
                pass

            # 发送启动命令
            try:
                command = ScannerCommand(command='start', data=cfg, timestamp=time.time())
                self._command_queue.put(command)
                self._logger.info("启动命令已发送到进程")
                
                # 发送进度更新
                try:
                    self.signals.progress_updated.emit("启动命令已发送，等待扫描器初始化...")
                except Exception:
                    pass
            except Exception as e:
                self._logger.error(f"发送启动命令失败: {e}")
                self._cleanup_process()
                return

            self._running = True
            self._current_config = cfg
            self._logger.info("扫描进程启动完成")
            
            # 发送最终进度更新
            try:
                self.signals.progress_updated.emit("扫描进程启动完成，开始运行...")
            except Exception:
                pass
            
            # 注意：真正"就绪"以收到子进程上报的 running=True 状态为准；
            # 这里不直接标记 _ready，等待 _poll_queues 中的状态握手。
        else:
            error = result.get("error", "未知错误")
            self._logger.error(f"进程启动失败: {error}")
            
            # 发送错误进度更新
            try:
                self.signals.progress_updated.emit(f"进程启动失败: {error}")
            except Exception:
                pass
                
            self._cleanup_process()

    def _on_process_start_error(self, task_id: str, error):
        """进程启动错误回调（在主线程中执行）"""
        self._logger.error(f"线程化启动扫描进程失败: {error}")
        
        # 发送错误进度更新
        try:
            self.signals.progress_updated.emit(f"进程启动错误: {error}")
        except Exception:
            pass
            
        self._cleanup_process()

    def _on_startup_timeout(self):
        """启动看门狗超时回调

        触发条件：在设定时间内未从子进程收到任何 running=True 的状态上报，
        可能是子进程卡死/初始化异常/目标窗口解析过慢。此时执行以下策略：
        1) 输出明确的进度与日志，便于用户理解当前卡在"启动中"；
        2) 清理当前进程资源；
        3) 若剩余重试次数>0，则按指数回退延迟后自动重试；
        4) 若用尽重试，发射错误信号并保持安全停机状态。
        """
        try:
            if self._ready:
                # 已就绪则忽略（可能是晚到的超时）
                self._logger.debug("看门狗触发但已就绪，忽略")
                return

            self._logger.warning("扫描进程启动超时，未收到就绪握手")
            try:
                self.signals.progress_updated.emit("启动超时，正在尝试自动重启...")
            except Exception:
                pass

            # 清理当前进程
            self._cleanup_process()

            # 判断是否可重试
            if self._startup_attempt < self._max_startup_attempts and self._current_config is not None:
                # 计算回退延迟（毫秒）
                delay_ms = min(int(800 * (1.6 ** (self._startup_attempt - 1))), self._max_backoff_ms)
                self._logger.info(f"计划在 {delay_ms}ms 后重试第 {self._startup_attempt + 1} 次启动")
                self._schedule_with_fallback(
                    delay_ms,
                    lambda: self.start_scanning(self._current_config),
                    "启动看门狗自动重试",
                )
            else:
                # 用尽重试，报错并停机
                self._logger.error("扫描进程启动多次失败，已放弃重试")
                try:
                    self.signals.error_occurred.emit("扫描进程启动失败，请检查配置或系统环境")
                except Exception:
                    pass
        except Exception as e:
            # 看门狗自身不应影响主流程，任何异常都要被吞掉并记录
            try:
                self._logger.error(f"启动看门狗处理异常: {e}")
            except Exception:
                pass

    def _is_session_stale(self, expected_token: Optional[int]) -> bool:
        """判断延迟回调是否属于过期会话。"""
        if expected_token is None:
            return False
        with self._state_lock:
            return expected_token != self._session_token

    def _schedule_with_fallback(self, delay_ms: int, callback: Callable[[], None], task_name: str):
        """优先使用Qt定时器，并用线程定时器兜底，避免无事件循环导致卡死。"""
        run_lock = threading.Lock()
        has_run = {"value": False}

        def _run_once():
            with run_lock:
                if has_run["value"]:
                    return
                has_run["value"] = True
            callback()

        try:
            QtCore.QTimer.singleShot(delay_ms, _run_once)
        except Exception as e:
            self._logger.debug(f"Qt定时器调度[{task_name}]失败: {e}")

        timer_ref: Dict[str, Optional[threading.Timer]] = {"timer": None}

        def _fallback_runner():
            try:
                _run_once()
            finally:
                timer = timer_ref["timer"]
                with self._fallback_timers_lock:
                    if timer in self._fallback_timers:
                        self._fallback_timers.remove(timer)

        # 略微延后于Qt定时器触发，正常GUI场景下不会抢先执行。
        delay_s = max(delay_ms / 1000.0 + 0.15, 0.05)
        timer = threading.Timer(delay_s, _fallback_runner)
        timer.daemon = True
        timer_ref["timer"] = timer
        with self._fallback_timers_lock:
            self._fallback_timers.append(timer)
        timer.start()

    def _cancel_fallback_timers(self):
        """取消并清空兜底定时器，避免旧回调影响新会话。"""
        with self._fallback_timers_lock:
            timers = list(self._fallback_timers)
            self._fallback_timers.clear()

        for timer in timers:
            try:
                timer.cancel()
            except Exception:
                pass

    def _close_ipc_queue(self, queue_obj: Optional[mp.Queue], queue_name: str):
        """关闭并回收IPC队列资源，避免句柄泄漏。"""
        if queue_obj is None:
            return

        try:
            close_fn = getattr(queue_obj, "close", None)
            if callable(close_fn):
                close_fn()
        except Exception as e:
            self._logger.debug(f"关闭{queue_name}队列失败: {e}")

        try:
            join_fn = getattr(queue_obj, "join_thread", None)
            if callable(join_fn):
                join_fn()
        except Exception as e:
            self._logger.debug(f"回收{queue_name}队列线程失败: {e}")

    def _force_stop_process(self, process_obj: Optional[mp.Process], join_timeout_s: float = 3.0,
                            kill_timeout_s: float = 1.0) -> bool:
        """尽最大努力停止子进程，支持同步/异步场景复用。"""
        if process_obj is None:
            return True

        try:
            if not process_obj.is_alive():
                return True
        except Exception:
            return True

        safe_join_timeout = max(0.0, float(join_timeout_s))
        safe_kill_timeout = max(0.0, float(kill_timeout_s))

        try:
            process_obj.terminate()
            process_obj.join(timeout=safe_join_timeout)
        except Exception as e:
            self._logger.debug(f"terminate扫描进程失败: {e}")

        try:
            if process_obj.is_alive():
                process_obj.kill()
                process_obj.join(timeout=safe_kill_timeout)
        except Exception as e:
            self._logger.debug(f"kill扫描进程失败: {e}")

        try:
            return not process_obj.is_alive()
        except Exception:
            return True

    def stop_scanning(self) -> bool:
        """停止扫描进程"""
        process_alive = bool(self._process and self._process.is_alive())
        if not self._running and not process_alive:
            self._logger.warning("扫描进程未在运行")
            return True

        try:
            with self._state_lock:
                stop_token = self._session_token

            # 发送停止命令
            if self._command_queue:
                command = ScannerCommand(command='stop', timestamp=time.time())
                self._command_queue.put(command)

                # 等待一段时间后发送退出命令
                self._schedule_with_fallback(
                    1000,
                    lambda token=stop_token: self._send_exit_command(token),
                    "发送退出命令",
                )
            else:
                # 无命令队列时直接进入清理，避免悬挂状态卡住重启。
                self._schedule_with_fallback(
                    0,
                    lambda token=stop_token: self._cleanup_process(token),
                    "直接清理扫描进程",
                )

            return True

        except Exception as e:
            self._logger.error(f"停止扫描进程失败: {e}")
            return False

    def update_config(self, cfg: AppConfig):
        """更新配置"""
        self._current_config = cfg

        if self._running and self._command_queue:
            try:
                command = ScannerCommand(command='update_config', data=cfg, timestamp=time.time())
                self._command_queue.put(command)
                self._logger.info("配置更新命令已发送")
            except Exception as e:
                self._logger.error(f"发送配置更新命令失败: {e}")

    def is_running(self) -> bool:
        """检查是否在运行"""
        return self._running and self._process and self._process.is_alive()

    def _send_exit_command(self, expected_token: Optional[int] = None):
        """发送退出命令"""
        if self._is_session_stale(expected_token):
            self._logger.debug("跳过过期会话的退出命令")
            return

        if self._command_queue:
            try:
                command = ScannerCommand(command='exit', timestamp=time.time())
                self._command_queue.put(command)
            except:
                pass

        # 延迟清理
        self._schedule_with_fallback(
            2000,
            lambda token=expected_token: self._cleanup_process(token),
            "延迟清理扫描进程",
        )

    def _cleanup_process(self, expected_token: Optional[int] = None, terminate_in_background: bool = True):
        """清理进程资源"""
        if self._is_session_stale(expected_token):
            self._logger.debug("跳过过期会话的清理请求")
            return

        with self._cleanup_lock:
            if self._is_session_stale(expected_token):
                self._logger.debug("清理阶段检测到过期会话，已跳过")
                return

            self._running = False
            self._ready = False

            # 关闭看门狗，避免错误触发
            if self._startup_watchdog.isActive():
                try:
                    self._startup_watchdog.stop()
                except Exception:
                    pass

            # 停止轮询
            if self._poll_timer.isActive():
                self._poll_timer.stop()

            # 终止进程
            if self._process and self._process.is_alive():
                old_process = self._process
                if terminate_in_background:
                    # 非阻塞路径：在后台线程完成进程终止，避免阻塞UI线程。
                    threading.Thread(
                        target=self._force_stop_process,
                        args=(old_process, 3.0, 1.0),
                        daemon=True
                    ).start()
                    self._process = None  # 立即清除引用
                else:
                    self._force_stop_process(old_process, 3.0, 1.0)
                    self._process = None

            # 清理并关闭IPC队列（先关资源，再断引用）
            self._close_ipc_queue(self._command_queue, "命令")
            self._close_ipc_queue(self._status_queue, "状态")
            self._close_ipc_queue(self._hit_queue, "命中")
            self._close_ipc_queue(self._log_queue, "日志")
            self._command_queue = None
            self._status_queue = None
            self._hit_queue = None
            self._log_queue = None
            self._process = None

            # 清理当前会话遗留的兜底定时器
            self._cancel_fallback_timers()

            self._logger.info("扫描进程资源已清理")

    def shutdown(self, timeout_s: float = 6.0) -> bool:
        """同步关闭扫描进程并回收资源（用于应用退出链路）。"""
        safe_timeout = max(0.5, float(timeout_s))
        deadline = time.monotonic() + safe_timeout

        with self._state_lock:
            token = self._session_token

        # 退出链路中要避免旧定时回调与同步清理互相干扰
        self._cancel_fallback_timers()

        proc = self._process
        command_queue = self._command_queue

        if command_queue:
            try:
                command_queue.put(ScannerCommand(command='stop', timestamp=time.time()))
            except Exception as e:
                self._logger.debug(f"发送stop命令失败: {e}")

        # 给子进程短暂窗口执行 stop -> cleanup_capture_manager()，优先优雅释放WGC会话
        graceful_wait_s = min(1.2, max(0.0, deadline - time.monotonic()))
        while proc and proc.is_alive() and graceful_wait_s > 0:
            sleep_step = min(0.05, graceful_wait_s)
            time.sleep(sleep_step)
            graceful_wait_s -= sleep_step

        if proc and proc.is_alive() and command_queue:
            try:
                command_queue.put(ScannerCommand(command='exit', timestamp=time.time()))
            except Exception as e:
                self._logger.debug(f"发送exit命令失败: {e}")

        if proc and proc.is_alive():
            remaining = max(0.0, deadline - time.monotonic())
            if remaining > 0:
                try:
                    proc.join(timeout=remaining)
                except Exception as e:
                    self._logger.debug(f"等待扫描进程退出失败: {e}")

        if proc and proc.is_alive():
            self._logger.warning("扫描进程未在超时内退出，执行强制终止")
            force_join_timeout = max(0.0, deadline - time.monotonic())
            self._force_stop_process(proc, join_timeout_s=force_join_timeout, kill_timeout_s=1.0)

        self._cleanup_process(expected_token=token, terminate_in_background=False)
        return True

    def _reset_polling_state(self):
        """重置轮询状态，确保新会话以基础频率启动"""
        self._current_poll_interval = self._base_poll_interval
        self._poll_timer.setInterval(self._current_poll_interval)
        self._poll_stats['empty_polls'] = 0
        self._poll_stats['active_polls'] = 0
        self._poll_stats['last_activity'] = time.time()
        for state in self._queue_backoff.values():
            state['empty_streak'] = 0
            state['skip_ticks'] = 0

    @staticmethod
    def _calc_skip_ticks(empty_streak: int, max_skip: int) -> int:
        """根据连续空取次数计算跳帧数，减少空轮询异常开销。"""
        if empty_streak < 3:
            return 0
        if empty_streak < 8:
            return min(max_skip, 1)
        if empty_streak < 16:
            return min(max_skip, 2)
        return max_skip

    def _drain_queue_with_budget(
        self,
        queue_name: str,
        queue_obj: Optional[mp.Queue],
        max_items: int,
        deadline: float,
        consumer: Callable[[Any], None],
    ) -> int:
        """在预算内尽量消费队列元素，优先使用 get_nowait 避免 empty() 误判。"""
        if queue_obj is None or max_items <= 0:
            return 0

        state = self._queue_backoff.get(queue_name)
        if state is not None and state['skip_ticks'] > 0:
            state['skip_ticks'] -= 1
            return 0

        processed = 0
        while processed < max_items and time.perf_counter() < deadline:
            try:
                item = queue_obj.get_nowait()
            except Empty:
                break
            except Exception as e:
                self._logger.debug(f"读取{queue_name}队列失败: {e}")
                break

            try:
                consumer(item)
            except Exception as e:
                self._logger.debug(f"处理{queue_name}队列元素失败: {e}")
                break
            processed += 1

        if state is not None:
            if processed > 0:
                state['empty_streak'] = 0
                state['skip_ticks'] = 0
            else:
                state['empty_streak'] += 1
                state['skip_ticks'] = self._calc_skip_ticks(
                    state['empty_streak'],
                    state['max_skip'],
                )

        return processed

    def _poll_queues(self):
        """轮询队列获取结果 - 自适应轮询机制，限量处理避免阻塞UI"""
        try:
            current_time = time.time()
            has_activity = False

            # 使用局部引用，减少属性查找与并发清理引起的抖动
            status_queue = self._status_queue
            hit_queue = self._hit_queue
            log_queue = self._log_queue

            # 队列尚未初始化或已被清理，直接走空闲路径
            if status_queue is None and hit_queue is None and log_queue is None:
                self._adjust_poll_interval(False, current_time)
                return

            deadline = time.perf_counter() + self._max_tick_budget_s
            signals = self.signals

            # 处理状态更新：仅取本帧的最新若干条，且最终只发射最后一条（合并抖动）
            status_holder = {'latest': None}

            def _consume_status(status: ScannerStatus):
                status_holder['latest'] = status

            status_processed = self._drain_queue_with_budget(
                'status',
                status_queue,
                self._max_status_per_tick,
                deadline,
                _consume_status,
            )
            latest_status = status_holder['latest']
            has_activity = status_processed > 0

            if latest_status is not None:
                # 发射到上层
                signals.status_updated.emit(latest_status)
                # 就绪握手：首次收到 running=True 视为真正“启动完成”
                if latest_status.running and not self._ready:
                    self._ready = True
                    # 成功就绪后重置尝试计数，并关闭看门狗
                    self._startup_attempt = 0
                    if self._startup_watchdog.isActive():
                        try:
                            self._startup_watchdog.stop()
                        except Exception:
                            pass
                    self._logger.info("扫描进程就绪握手完成")

            # 处理命中结果：命中通常不多，限量发射
            hit_processed = self._drain_queue_with_budget(
                'hit',
                hit_queue,
                self._max_hit_per_tick,
                deadline,
                lambda hit: signals.hit_detected.emit(hit),
            )
            has_activity = has_activity or (hit_processed > 0)

            # 处理日志消息：日志量可能很大，严格限流
            log_processed = self._drain_queue_with_budget(
                'log',
                log_queue,
                self._max_log_per_tick,
                deadline,
                lambda log_msg: signals.log_message.emit(log_msg),
            )
            has_activity = has_activity or (log_processed > 0)

            # 自适应调整轮询频率
            self._adjust_poll_interval(has_activity, current_time)

        except Exception as e:
            self._logger.error(f"轮询队列异常: {e}")

    def _adjust_poll_interval(self, has_activity: bool, current_time: float):
        """自适应调整轮询间隔"""
        if has_activity:
            # 有活动：重置空闲计数，并尽快恢复到基础轮询频率
            self._poll_stats['active_polls'] += 1
            self._poll_stats['empty_polls'] = 0
            self._poll_stats['last_activity'] = current_time

            if self._current_poll_interval > self._base_poll_interval:
                self._current_poll_interval = self._base_poll_interval
                self._poll_timer.setInterval(self._current_poll_interval)
                self._logger.debug(f"恢复轮询间隔至基础值: {self._current_poll_interval}ms")
                return

            # 若持续活跃，再小幅提升到更快的轮询频率
            if self._poll_stats['active_polls'] >= 3:
                new_interval = max(self._min_poll_interval, self._current_poll_interval - 10)
                if new_interval != self._current_poll_interval:
                    self._current_poll_interval = new_interval
                    self._poll_timer.setInterval(new_interval)
                    self._logger.debug(f"降低轮询间隔至: {new_interval}ms")
        else:
            # 无活动：增加空闲计数，重置活跃计数
            self._poll_stats['empty_polls'] += 1
            self._poll_stats['active_polls'] = 0

            # 分级拉大空闲轮询间隔，减少空闲时CPU占用
            idle_time = current_time - self._poll_stats['last_activity']
            if idle_time >= 12.0:
                target_interval = self._max_poll_interval
            elif idle_time >= 6.0:
                target_interval = min(
                    self._max_poll_interval,
                    max(self._current_poll_interval + 120, self._base_poll_interval * 4)
                )
            elif self._poll_stats['empty_polls'] >= 8:
                target_interval = min(self._max_poll_interval, self._current_poll_interval + 40)
            else:
                target_interval = self._current_poll_interval

            if target_interval != self._current_poll_interval:
                self._current_poll_interval = int(target_interval)
                self._poll_timer.setInterval(self._current_poll_interval)
                self._logger.debug(f"增加轮询间隔至: {self._current_poll_interval}ms")

    def cleanup(self, blocking: bool = False, timeout_s: float = 6.0):
        """清理资源。blocking=True时执行同步关停，适用于应用退出。"""
        if blocking:
            return self.shutdown(timeout_s=timeout_s)

        if self._running:
            self.stop_scanning()

        with self._state_lock:
            token = self._session_token

        # 等待清理完成（含无事件循环兜底）
        self._schedule_with_fallback(
            3000,
            lambda expected=token: self._cleanup_process(expected),
            "管理器最终清理",
        )


# 全局扫描进程管理器实例
_global_scanner_manager: Optional[ScannerProcessManager] = None


def get_global_scanner_manager() -> ScannerProcessManager:
    """获取全局扫描进程管理器实例"""
    global _global_scanner_manager
    if _global_scanner_manager is None:
        _global_scanner_manager = ScannerProcessManager()
    return _global_scanner_manager
