# -*- coding: utf-8 -*-
"""
Qt 高DPI/多屏缩放管理器

目标：
- 在创建 QApplication 前后分别做好两件事：
  1) 提前设置 Qt 的高DPI缩放因子取整策略（避免跨屏抖动与警告）；
  2) 在应用创建后监听屏幕变化，尽量保持多屏缩放一致的体验，并提供统一的查询接口。

使用说明：
- 在主入口模块最早位置调用 `set_rounding_policy_early()`（在创建 QApplication 之前）。
- 创建 QApplication 之后，实例化 `QtDpiManager(app)` 以监听屏幕变化并暴露查询工具。

注意：
- 本模块的导入不会强依赖 PySide6，不安装 PySide6 也能导入；只有在实际
  调用 Qt 相关 API 时才会访问 Qt 类型，便于编写可运行的单元测试。
"""

from __future__ import annotations
import importlib
import logging
import os
from dataclasses import dataclass
from typing import Optional, Tuple

# ----------------------- 纯函数与配置解析（可单测，无 Qt 依赖） -----------------------

_POLICY_ENV = "AIIDE_DPI_ROUNDING_POLICY"
_LOGGER = logging.getLogger(__name__)
_VALID_POLICY_NAMES = {"PassThrough", "Round", "Ceil", "RoundPreferFloor"}


def resolve_rounding_policy(env: Optional[dict] = None) -> str:
    """解析取整策略（无 Qt 依赖）

    环境变量优先级：
    - AIIDE_DPI_ROUNDING_POLICY: pass|floor|ceil|round（大小写均可）
    - 若未指定，默认使用 RoundPreferFloor（与现有工程测试保持一致）

    返回值为策略名称字符串，用于后续映射到 Qt 枚举：
    - "PassThrough"
    - "Round"
    - "Ceil"
    - "RoundPreferFloor"(默认)
    """
    e = env or os.environ
    val = str(e.get(_POLICY_ENV, "")).strip().lower()
    if val in ("pass", "passthrough", "p"):
        return "PassThrough"
    if val in ("ceil", "c"):
        return "Ceil"
    if val in ("round", "r"):
        return "Round"
    # 默认：与现有工程测试用例一致
    return "RoundPreferFloor"


def compute_scale_for_dpi(dpi: float, base: float = 96.0) -> float:
    """根据 DPI 计算缩放因子（无 Qt 依赖）"""
    try:
        dpi = float(dpi)
        base = float(base) if base else 96.0
        if dpi <= 0:
            return 1.0
        return dpi / base
    except Exception:
        return 1.0


# ----------------------- Qt 相关：提前设置与运行时管理 -----------------------

def _qt_enums():
    """惰性导入 QtCore/QtGui，兼容 Qt5/Qt6 与不同绑定。"""
    candidates = ("PySide6", "PyQt6", "PySide2", "PyQt5")
    last_exc = None
    for binding in candidates:
        try:
            qt_core = importlib.import_module(f"{binding}.QtCore")
            qt_gui = importlib.import_module(f"{binding}.QtGui")
            return qt_core, qt_gui
        except Exception as exc:
            last_exc = exc
    raise ImportError("未找到可用的Qt绑定（PySide6/PyQt6/PySide2/PyQt5）") from last_exc


def _resolve_rounding_policy_enum(QtCore, QtGui, policy_name: str):
    """将策略名称映射为Qt枚举值，兼容Qt5/Qt6的枚举挂载位置。"""
    enum_sources = []
    qt_namespace = getattr(QtCore, "Qt", None)
    if qt_namespace is not None:
        enum_sources.append(getattr(qt_namespace, "HighDpiScaleFactorRoundingPolicy", None))
        enum_sources.append(qt_namespace)
    qgui_cls = getattr(QtGui, "QGuiApplication", None)
    if qgui_cls is not None:
        enum_sources.append(getattr(qgui_cls, "HighDpiScaleFactorRoundingPolicy", None))
        enum_sources.append(qgui_cls)

    # Qt5/Qt6大多保留这些成员名；RoundPreferFloor缺失时回退到Round。
    name_candidates = [policy_name]
    if policy_name == "RoundPreferFloor":
        name_candidates.append("Round")

    for member_name in name_candidates:
        for source in enum_sources:
            if source is None:
                continue
            value = getattr(source, member_name, None)
            if value is not None:
                return value, member_name
    return None, ""


def set_rounding_policy_early(policy_name: Optional[str] = None) -> bool:
    """在创建QApplication之前设置缩放因子取整策略。

    参数：
    - policy_name: 若为None则从环境变量解析；否则应为resolve_rounding_policy的返回值。

    返回：
    - True: 设置成功
    - False: 设置失败或当前Qt不支持（并记录日志）
    """
    name = policy_name or resolve_rounding_policy()
    if name not in _VALID_POLICY_NAMES:
        _LOGGER.warning("未知DPI取整策略'%s'，回退为RoundPreferFloor", name)
        name = "RoundPreferFloor"

    try:
        QtCore, QtGui = _qt_enums()
    except Exception as exc:
        _LOGGER.warning("设置Qt DPI取整策略失败：未找到Qt绑定。错误：%s", exc)
        return False

    setter = getattr(QtGui.QGuiApplication, "setHighDpiScaleFactorRoundingPolicy", None)
    if not callable(setter):
        _LOGGER.info("当前Qt版本不支持setHighDpiScaleFactorRoundingPolicy，跳过设置")
        return False

    enum_value, used_name = _resolve_rounding_policy_enum(QtCore, QtGui, name)
    if enum_value is None:
        _LOGGER.warning("Qt绑定未提供DPI取整策略枚举：%s", name)
        return False

    try:
        setter(enum_value)
        if used_name != name:
            _LOGGER.info("DPI取整策略'%s'在当前Qt中不可用，已回退为'%s'", name, used_name)
        return True
    except Exception as exc:
        # 保持降级行为：失败时不抛异常，仅返回False并记录日志。
        _LOGGER.warning("设置Qt DPI取整策略失败（policy=%s）：%s", name, exc)
        return False


@dataclass
class ScreenScale:
    """屏幕缩放信息快照（简化结构，便于日志与调试）"""
    name: str
    dpi: float
    dpr: float
    scale: float


class QtDpiManager:
    """Qt 高DPI/多屏缩放管理器

    功能：
    - 监听屏幕增删与主屏切换，更新内部缩放信息（用于诊断与可选的统一缩放策略）。
    - 暴露 `effective_scale(widget)` 与 `snapshot()` 便于业务侧做像素/字体等按比例适配。
    - 可选“统一缩放外观”模式：将应用字体按主屏缩放一次性调整，减轻跨屏跳动感。
      注意：该模式不会改变 Qt 的 per-monitor 缩放行为，仅通过字体大小微调达到观感一致。
    """

    def __init__(self, app, unify_appearance: bool = False):
        from PySide6 import QtCore
        self._app = app
        self._unify = unify_appearance
        self._primary = app.primaryScreen()
        self._scales: dict[str, ScreenScale] = {}
        # 记录基准缩放：用于跨屏一致外观计算
        self._base_scale: float = 1.0
        self._refresh_base_scale(self._primary)

        # 建立连接：屏幕增删/主屏变更
        app.screenAdded.connect(self._on_screen_added)
        app.screenRemoved.connect(self._on_screen_removed)
        app.primaryScreenChanged.connect(self._on_primary_changed)

        # 初始化现有屏幕
        for s in app.screens():
            self._attach_screen_signals(s)
            self._update_screen_scale(s)

        if self._unify:
            self._apply_unified_font_scale()

    # ------------------- 公共接口 -------------------
    def effective_scale(self, widget) -> float:
        """返回给定部件当前屏幕的缩放因子（dpi/96）。"""
        try:
            screen = (widget.window().windowHandle().screen() if widget else None) or self._app.primaryScreen()
            if screen is None:
                return 1.0
            dpi = float(screen.logicalDotsPerInch())
            return compute_scale_for_dpi(dpi)
        except Exception:
            return 1.0

    def snapshot(self) -> dict:
        """返回当前所有屏幕的缩放信息快照，便于日志与调试。"""
        return {k: vars(v) for k, v in self._scales.items()}

    # 一致外观：计算相对基准屏的比例（>1 表示更“密”，需要放大像素型尺寸）
    def consistency_ratio(self, widget) -> float:
        """返回相对基准屏的缩放比：current_scale / base_scale。"""
        cur = self.effective_scale(widget)
        base = self._base_scale or 1.0
        return max(0.5, min(4.0, cur / base))

    def scale_px(self, px: int, widget) -> int:
        """将像素尺寸按一致外观进行缩放（用于像素单位的控件尺寸/间距/图标）。"""
        try:
            r = self.consistency_ratio(widget)
            return max(1, int(round(px * r)))
        except Exception:
            return px

    # 供业务调用：为窗口安装跨屏自适应适配器
    def attach_window_adapter(self, widget, on_ratio_changed: Optional[callable] = None):
        """为顶层窗口安装DPI适配器：
        - 监听窗口跨屏移动与screenChanged；
        - 计算一致外观比例并回调业务侧处理像素单位控件；
        - 默认回调为空；业务可在回调内按需调整如QToolBar.iconSize、行高等。
        """
        try:
            from PySide6 import QtCore

            class _Adapter(QtCore.QObject):
                def __init__(self, outer, w, cb):
                    super().__init__(w)
                    self.outer = outer
                    self.w = w
                    self.cb = cb
                    self._last_screen = None
                    self._install()

                def _install(self):
                    try:
                        wh = self.w.windowHandle()
                        if wh is not None:
                            try:
                                wh.screenChanged.connect(self._on_screen_changed)
                            except Exception:
                                pass
                    except Exception:
                        pass
                    self.w.installEventFilter(self)
                    # 首次触发
                    QtCore.QTimer.singleShot(0, self._emit_if_changed)

                def _on_screen_changed(self, *args):
                    self._emit_if_changed()

                def eventFilter(self, obj, ev):
                    et = ev.type()
                    # 监听移动/显示/状态变化等可能导致跨屏
                    if et in (QtCore.QEvent.Move, QtCore.QEvent.Show, QtCore.QEvent.WindowStateChange, QtCore.QEvent.Resize):
                        self._emit_if_changed()
                    return super().eventFilter(obj, ev)

                def _emit_if_changed(self):
                    s = choose_screen_for_widget(self.w)
                    if s is not self._last_screen and s is not None:
                        self._last_screen = s
                        ratio = self.outer.consistency_ratio(self.w)
                        # 为窗口打标，方便调试
                        try:
                            self.w.setProperty('aiide_dpi_ratio', ratio)
                        except Exception:
                            pass
                        if callable(self.cb):
                            try:
                                self.cb(self.w, ratio)
                            except Exception:
                                pass

            _Adapter(self, widget, on_ratio_changed)
        except Exception:
            pass

    # ------------------- 内部实现 -------------------
    def _is_primary_screen(self, screen) -> bool:
        """判断传入屏幕是否当前主屏。"""
        try:
            primary = self._app.primaryScreen() or self._primary
            if primary is None or screen is None:
                return False
            if screen is primary:
                return True
            return (screen.name() or "") == (primary.name() or "")
        except Exception:
            return False

    def _refresh_base_scale(self, screen=None):
        """刷新基准缩放，主屏切换或主屏DPI变化时都需要更新。"""
        try:
            target = screen or self._app.primaryScreen() or self._primary
            dpi = float(target.logicalDotsPerInch()) if target else 96.0
            scale = compute_scale_for_dpi(dpi)
            self._base_scale = scale if scale > 0 else 1.0
        except Exception:
            self._base_scale = 1.0

    def _attach_screen_signals(self, screen):
        from PySide6 import QtCore
        # Qt6: QScreen 没有 dpiChanged 信号，但 logicalDotsPerInchChanged/geometryChanged/dpi (间接)
        # 这里选择 geometryChanged + physicalDotsPerInchX/Y 再取一次值
        try:
            screen.geometryChanged.connect(lambda *_: self._update_screen_scale(screen))
        except Exception:
            pass
        try:
            # 某些平台支持以下信号
            getattr(screen, 'physicalDotsPerInchChanged', None) and screen.physicalDotsPerInchChanged.connect(lambda *_: self._update_screen_scale(screen))
        except Exception:
            pass

    def _update_screen_scale(self, screen):
        try:
            name = screen.name() or "Unnamed"
            dpi = float(screen.logicalDotsPerInch())
            dpr = float(getattr(screen, 'devicePixelRatio', lambda: 1.0)())
            scale = compute_scale_for_dpi(dpi)
            self._scales[name] = ScreenScale(name=name, dpi=dpi, dpr=dpr, scale=scale)
            if self._is_primary_screen(screen):
                self._refresh_base_scale(screen)
            if self._unify:
                self._apply_unified_font_scale()
        except Exception:
            pass

    def _apply_unified_font_scale(self):
        """按主屏缩放一次性调节应用字体，以提高跨屏观感一致性。"""
        try:
            primary = self._app.primaryScreen() or self._primary
            if not primary:
                return
            base_scale = compute_scale_for_dpi(float(primary.logicalDotsPerInch()))
            if base_scale <= 0:
                return
            f = self._app.font()
            # 将 pointSize 调整为相对 10pt 的缩放，避免在多屏来回移动时字体抖动
            ps = max(8, round(10 * base_scale))
            f.setPointSize(ps)
            self._app.setFont(f)
        except Exception:
            pass

    # 屏幕事件处理
    def _on_screen_added(self, screen):
        self._attach_screen_signals(screen)
        self._update_screen_scale(screen)

    def _on_screen_removed(self, screen):
        try:
            name = screen.name() or "Unnamed"
            self._scales.pop(name, None)
            # 屏幕移除后可能触发主屏变化，统一在这里兜底刷新基准缩放。
            self._refresh_base_scale()
        except Exception:
            pass

    def _on_primary_changed(self, screen):
        self._primary = screen or self._app.primaryScreen()
        self._refresh_base_scale(self._primary)
        if self._primary is not None:
            self._update_screen_scale(self._primary)
        elif self._unify:
            # 无主屏时仅保底刷新字体策略，避免状态残留。
            self._apply_unified_font_scale()


# ------------------- 纯算法：窗口所在屏幕判定（可单测） -------------------

def rect_intersection_area(a: Tuple[int, int, int, int], b: Tuple[int, int, int, int]) -> int:
    """返回两个矩形交集面积，矩形为(left, top, right, bottom)。"""
    try:
        l = max(a[0], b[0]); t = max(a[1], b[1])
        r = min(a[2], b[2]); btm = min(a[3], b[3])
        w = max(0, r - l); h = max(0, btm - t)
        return w * h
    except Exception:
        return 0


def choose_screen_by_rect(window_rect: Tuple[int, int, int, int], screens: list[Tuple[int, int, int, int]]) -> int:
    """根据最大交集面积选择窗口所在屏幕，返回索引；若无交集，返回最接近中心点的屏幕索引。"""
    if not screens:
        return -1
    # 先比交集面积
    areas = [rect_intersection_area(window_rect, s) for s in screens]
    idx = max(range(len(screens)), key=lambda i: areas[i])
    if areas[idx] > 0:
        return idx
    # 无交集：取窗口中心点，找包含该点或距离最近的屏
    cx = (window_rect[0] + window_rect[2]) // 2
    cy = (window_rect[1] + window_rect[3]) // 2
    def _dist2(sr):
        # 点到矩形的最短距离平方
        x = cx if sr[0] <= cx <= sr[2] else (sr[0] if cx < sr[0] else sr[2])
        y = cy if sr[1] <= cy <= sr[3] else (sr[1] if cy < sr[1] else sr[3])
        dx = cx - x; dy = cy - y
        return dx*dx + dy*dy
    return min(range(len(screens)), key=lambda i: _dist2(screens[i]))


def choose_screen_for_widget(widget):
    """使用Qt对象判断窗口所在屏幕；在多屏跨越时选择交集面积最大的屏幕。"""
    try:
        from PySide6 import QtCore
        if widget is None:
            return None
        win = widget.window()
        if win is None:
            return None
        fg = win.frameGeometry()
        wr = (fg.left(), fg.top(), fg.right(), fg.bottom())
        app = QtCore.QCoreApplication.instance()
        screens = [s.geometry() for s in app.screens()] if app else []
        rects = [(g.left(), g.top(), g.right(), g.bottom()) for g in screens]
        idx = choose_screen_by_rect(wr, rects)
        if 0 <= idx < len(screens):
            return app.screens()[idx]
    except Exception:
        pass
    # 兜底：直接返回窗口句柄的screen或主屏
    try:
        wh = widget.windowHandle()
        return (wh.screen() if wh else None) or (QtCore.QCoreApplication.instance().primaryScreen())
    except Exception:
        return None
