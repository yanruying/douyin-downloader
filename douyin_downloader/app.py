#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI 启动入口
"""
import sys
import tempfile
import os
try:
    from PyQt6 import QtWidgets, QtGui
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

from douyin_downloader.constants import ICON_BYTES, ICON_BYTES_OPTIONS, CUSTOM_ICON_PATH, OPENPYXL_AVAILABLE
from douyin_downloader.utils.config import load_config
from douyin_downloader.gui.main_window import MainWindow

from douyin_downloader import gui

def get_app_icon():
    """获取应用程序图标"""
    icon_choice = gui.cfg.get('icon_choice', 'default')

    if icon_choice == 'custom' and os.path.exists(CUSTOM_ICON_PATH):
        try:
            with open(CUSTOM_ICON_PATH, 'rb') as f:
                custom_icon_bytes = f.read()
            return custom_icon_bytes
        except Exception as e:
            print(f"Warning: Failed to load custom icon: {e}")

    return ICON_BYTES_OPTIONS.get(icon_choice, ICON_BYTES)


def run_gui():
    """启动PyQt6图形界面"""
    app = QtWidgets.QApplication(sys.argv)

    # Windows系统特殊处理任务栏图标
    if sys.platform.startswith('win'):
        import ctypes
        myappid = 'douyin.downloader.app'  # 应用用户模型ID，确保任务栏图标正确显示
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)

    loaded_cfg = load_config()
    gui.cfg.update(loaded_cfg)

    try:
        icon_bytes = get_app_icon()
        with tempfile.NamedTemporaryFile(suffix='.ico', delete=False) as tmp:
            tmp.write(icon_bytes)
            tmp_icon_path = tmp.name
        app_icon = QtGui.QIcon(tmp_icon_path)
        app.setWindowIcon(app_icon)
    except Exception as e:
        print(f"Warning: Failed to create temp icon: {e}")

    checkmark_path = ''
    try:
        checkmark_svg = b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path fill="white" stroke="white" stroke-width="0.5" d="M13.5 4l-7 7-3.5-3.5 1.5-1.5 2 2 5.5-5.5z"/></svg>'
        with tempfile.NamedTemporaryFile(suffix='.svg', delete=False, mode='wb') as tmp_check:
            tmp_check.write(checkmark_svg)
            checkmark_path = tmp_check.name.replace('\\', '/')
    except Exception as e:
        print(f"Warning: Failed to create temp checkmark svg: {e}")

    try:
        app.setStyleSheet("""
        /* ---------------- 按钮 ---------------- */
        QPushButton {
            background-color: #409EFF; border: 1px solid #409EFF; color: white;
            padding: 6px 14px; border-radius: 0px; font-weight: 500; font-size: 13px;
        }
        QPushButton:hover { background-color: #66b1ff; border: 1px solid #66b1ff; }
        QPushButton:pressed { background-color: #3a8ee6; border: 1px solid #3a8ee6; }
        QPushButton:disabled {
            background-color: #a0cfff; border: 1px solid #a0cfff; color: #f0f0f0;
        }

        /* "停止"按钮的红色样式（通过 running="true" 属性激活） */
        QPushButton[running="true"] {
            background: #d9534f; color: white; padding: 7px 14px;
            border: 1px solid #d9534f; font-weight: 500; font-size: 13px;
        }
        QPushButton[running="true"]:hover {
            background: #fa8480; border: 1px solid #fa8480;
        }
        QPushButton[running="true"]:pressed {
            background: #d9534f; border: 1px solid #d9534f;
        }
        QPushButton[running="true"]:disabled {
            background: #f0b3b3; color: #f8e6e6; border: 1px solid #f0b3b3;
        }


        /* ---------------- 输入框 ---------------- */
        QLineEdit, QTextEdit, QSpinBox {
            border: 1px solid #dcdfe6; background: #ffffff; color: #303133;
            padding: 6px; border-radius: 0px; font-size: 13px;
        }
        QLineEdit:focus, QTextEdit:focus, QSpinBox:focus {
            border: 1px solid #409EFF; background: #f9fcff;
        }

        /* ---------------- 复选框 ---------------- */
        QCheckBox {
            spacing: 8px; font-size: 13px; color: #303133;
        }
        QCheckBox::indicator {
            width: 18px; height: 18px; border: 1px solid #dcdfe6;
            border-radius: 3px; background: #ffffff;
        }
        QCheckBox::indicator:hover {
            border: 1px solid #409EFF;
        }
        QCheckBox::indicator:checked {
            background-color: #409EFF; border: 1px solid #409EFF;
            image: url(""" + checkmark_path + r""");
        }
        QCheckBox::indicator:checked:hover {
            background-color: #66b1ff; border: 1px solid #66b1ff;
        }

        /* ---------------- 列表复选框 (QTreeWidget) ---------------- */
        QTreeView::indicator, QTreeWidget::indicator {
            width: 16px; height: 16px; border: 1px solid #c0c4cc;
            border-radius: 2px; background: #ffffff;
        }
        QTreeView::indicator:hover, QTreeWidget::indicator:hover {
            border: 1px solid #409EFF;
        }
        QTreeView::indicator:checked, QTreeWidget::indicator:checked {
            background-color: #409EFF; border: 1px solid #409EFF;
            image: url(""" + checkmark_path + r""");
        }

        /* ---------------- 列表 QTreeWidget ---------------- */
        QTreeWidget {
            background: #ffffff; border: 1px solid #e4e7ed;
            alternate-background-color: #fafbfc; gridline-color: #f2f6fc;
            selection-background-color: #d9eaff; font-size: 13px;
        }
        QTreeWidget::item { padding: 6px 4px; color: #222222; }
        QTreeWidget::item:hover { background: #f3f8fe; }
        QTreeWidget::item:selected { background: #cfe4ff; color: #000000; }

        /* ---------------- 进度条 ---------------- */
        QProgressBar {
            border: 1px solid #dcdfe6; background: #f5f7fa; height: 22px;
            border-radius: 0px; text-align: center; font-size: 12px; color: #303133;
        }
        QProgressBar::chunk {
            background-color: qlineargradient(x1:0, y1:0, x2:1, y2:0,
                stop:0 #66b1ff, stop:1 #409EFF);
            border-radius: 0px;
        }

        /* ---------------- 滚动条 ---------------- */
        QScrollBar:vertical {
            border: none; background: #f5f7fa; width: 10px; margin: 0px;
        }
        QScrollBar::handle:vertical {
            background: #c0c4cc; border-radius: 0px; min-height: 20px;
        }
        QScrollBar::handle:vertical:hover { background: #a6a9ad; }
        QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
            height: 0px; background: none;
        }
        QScrollBar:horizontal {
            border: none; background: #f5f7fa; height: 10px; margin: 0px;
        }
        QScrollBar::handle:horizontal {
            background: #c0c4cc; border-radius: 0px; min-width: 20px;
        }
        QScrollBar::handle:horizontal:hover { background: #a6a9ad; }
        QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal {
            width: 0px; background: none;
        }
        """)
    except Exception as e:
        print(f"Warning: Failed to set stylesheet: {e}")

    w = MainWindow(checkmark_path)
    w.show()
    app.exec()
