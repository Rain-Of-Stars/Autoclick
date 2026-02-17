# -*- coding: utf-8 -*-
"""
WGC预览保存回调冒烟测试。

目标：
- 验证保存成功/失败回调不会因作用域变量缺失抛异常；
- 验证回调结束后能恢复状态并清理待保存文件名。
"""
from __future__ import annotations

import numpy as np

from auto_approve.wgc_preview_dialog import WGCPreviewDialog


class _DummyButton:
    def __init__(self):
        self.enabled = None

    def setEnabled(self, value: bool):
        self.enabled = bool(value)


class _DummyLabel:
    def __init__(self):
        self.text = ""

    def setText(self, value: str):
        self.text = str(value)


def _build_dialog_stub() -> WGCPreviewDialog:
    """构造仅用于回调测试的轻量对象，避免真实UI依赖。"""
    dialog = WGCPreviewDialog.__new__(WGCPreviewDialog)
    dialog.save_btn = _DummyButton()
    dialog.status_label = _DummyLabel()
    dialog.is_capturing = True
    dialog.current_frame = np.zeros((2, 2, 3), dtype=np.uint8)
    dialog._pending_save_filename = "unit_test.png"
    return dialog


def test_save_complete_success_no_name_error(monkeypatch):
    """成功回调不应再访问越域变量导致NameError。"""
    dialog = _build_dialog_stub()

    monkeypatch.setattr(
        "auto_approve.wgc_preview_dialog.QtWidgets.QMessageBox.information",
        lambda *args, **kwargs: None,
    )

    result = {
        "file_size": 1024,
        "dimensions": (2, 2),
        "quality": 100,
        "file_path": "db://export/unit_test.png",
        "image_id": 1,
    }

    WGCPreviewDialog._on_save_complete(dialog, True, result)

    assert dialog.status_label.text.startswith("保存成功")
    assert dialog._pending_save_filename is None
    assert dialog.save_btn.enabled is True


def test_save_complete_error_path_recovers_ui(monkeypatch):
    """失败回调应恢复UI状态并清理待保存文件名。"""
    dialog = _build_dialog_stub()

    monkeypatch.setattr(
        "auto_approve.wgc_preview_dialog.QtWidgets.QMessageBox.critical",
        lambda *args, **kwargs: None,
    )

    WGCPreviewDialog._on_save_complete(dialog, False, "mock error")

    assert dialog.status_label.text == "保存失败"
    assert dialog._pending_save_filename is None
    assert dialog.save_btn.enabled is True
