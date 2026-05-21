#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - 自定义控件与样式
"""
import sys
try:
    from PyQt6 import QtWidgets
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

class NoFocusRectStyle(QtWidgets.QProxyStyle):
    """自定义样式类，用于禁用列表/树的焦点虚线框"""
    def drawPrimitive(self, element, option, painter, widget=None):
        # 不绘制焦点虚线框，保持界面干净
        if element == QtWidgets.QStyle.PrimitiveElement.PE_FrameFocusRect:
            return
        super().drawPrimitive(element, option, painter, widget)