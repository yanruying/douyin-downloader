#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - Settings Dialog
"""
import os
import sys
import shutil
try:
    from PyQt6 import QtWidgets, QtCore, QtGui
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

from douyin_downloader.gui import cfg
from douyin_downloader.utils.config import save_config
from douyin_downloader.constants import DEFAULT_THREAD_COUNT, ICON_BYTES_OPTIONS, CUSTOM_ICON_PATH
from .dialog_about import AboutWindow, TutorialWindow
from .dialog_cookie import CookieFetchWindow
from .dialog_browser import BrowserConfigWindow

class IconPreviewButton(QtWidgets.QPushButton):
    """图标预览按钮"""
    def __init__(self, icon_data, icon_name, parent=None):
        super().__init__(parent)
        self.icon_data = icon_data
        self.icon_name = icon_name
        self.setup_icon()
        self.setFixedSize(30, 30)
        self.setStyleSheet("""
            QPushButton {
                border: 2px solid #ddd;
                background-color: white;
                border-radius: 0px;
            }
            QPushButton:checked {
                border: 2px solid #409EFF;
                background-color: #ecf5ff;
            }
            QPushButton:hover {
                border: 2px solid #409EFF;
            }
        """)
        self.setCheckable(True)
        
    def setup_icon(self):
        """设置按钮图标"""
        try:
            pixmap = QtGui.QPixmap()
            pixmap.loadFromData(self.icon_data)
            icon = QtGui.QIcon(pixmap)
            self.setIcon(icon)
            self.setIconSize(QtCore.QSize(26, 26))
        except Exception as e:
            print(f"Failed to load icon: {e}")

class SettingsWindow(QtWidgets.QDialog):
    """设置窗口"""
    def __init__(self, parent=None, checkmark_svg_path=''):
        super().__init__(parent)
        self.checkmark_svg_path = checkmark_svg_path
        self.setWindowTitle('设置')
        self.setModal(False)
        self.resize(500, 550)
        
        layout = QtWidgets.QVBoxLayout(self)
        layout.setSpacing(12)
        
        top_btns_layout = QtWidgets.QHBoxLayout()
        self.view_log_btn = QtWidgets.QPushButton('查看日志')
        self.tutorial_btn = QtWidgets.QPushButton('查看教程')
        self.about_btn = QtWidgets.QPushButton('关于')
        top_btns_layout.addWidget(self.view_log_btn)
        top_btns_layout.addStretch()
        top_btns_layout.addWidget(self.tutorial_btn)
        top_btns_layout.addWidget(self.about_btn)
        layout.addLayout(top_btns_layout)
        layout.addSpacing(6)
        
        icon_layout = QtWidgets.QHBoxLayout()
        icon_label = QtWidgets.QLabel('图标选择:')
        icon_label.setFixedWidth(80)
        icon_layout.addWidget(icon_label)
        
        self.icon_buttons = QtWidgets.QButtonGroup(self)
        self.icon_buttons.setExclusive(True)
        
        self.default_icon_btn = IconPreviewButton(ICON_BYTES_OPTIONS["default"], "default")
        self.icon_buttons.addButton(self.default_icon_btn)
        icon_layout.addWidget(self.default_icon_btn)
        
        self.alt1_icon_btn = IconPreviewButton(ICON_BYTES_OPTIONS["alternative1"], "alternative1")
        self.icon_buttons.addButton(self.alt1_icon_btn)
        icon_layout.addWidget(self.alt1_icon_btn)
        
        # 自定义图标按钮（用于显示和选择自定义图标）
        self.custom_icon_preview_btn = IconPreviewButton(ICON_BYTES_OPTIONS["default"], "custom")
        self.custom_icon_preview_btn.setCheckable(True)
        self.icon_buttons.addButton(self.custom_icon_preview_btn)
        icon_layout.addWidget(self.custom_icon_preview_btn)
        
        self.custom_icon_btn = QtWidgets.QPushButton("自定义")
        self.custom_icon_btn.setFixedHeight(30)
        self.custom_icon_btn.setStyleSheet("""
            QPushButton {
                background-color: #409EFF; 
                border: 1px solid #409EFF; 
                color: white;
                border-radius: 0px; 
                font-weight: 500; 
                font-size: 13px;
                padding: 0 10px;
            }
            QPushButton:hover { 
                background-color: #66b1ff; 
                border: 1px solid #66b1ff; 
            }
            QPushButton:pressed { 
                background-color: #3a8ee6; 
                border: 1px solid #3a8ee6; 
            }
        """)
        self.custom_icon_btn.clicked.connect(self.on_custom_icon)
        icon_layout.addWidget(self.custom_icon_btn)
        
        icon_layout.addStretch()
        layout.addLayout(icon_layout)
        layout.addSpacing(6)
        
        cookie_btn_layout = QtWidgets.QHBoxLayout()
        self.cookie_auto_btn = QtWidgets.QPushButton('Cookie自动获取')
        self.browser_config_btn = QtWidgets.QPushButton('配置浏览器')
        cookie_btn_layout.addWidget(self.cookie_auto_btn)
        cookie_btn_layout.addWidget(self.browser_config_btn)
        cookie_btn_layout.addStretch()
        layout.addLayout(cookie_btn_layout)
        
        self.settings_cookie = QtWidgets.QTextEdit()
        self.settings_cookie.setPlainText(cfg.get('cookie', ''))
        self.settings_cookie.setFixedHeight(80)
        layout.addWidget(self.settings_cookie)
        layout.addSpacing(6)
        
        # 保存路径
        path_layout = QtWidgets.QHBoxLayout()
        path_layout.setSpacing(3)
        path_layout.addWidget(QtWidgets.QLabel('保存路径:'))
        self.settings_path = QtWidgets.QLineEdit()
        path_value = cfg.get('path', '')
        if not path_value:
            path_value = os.getcwd()
        self.settings_path.setText(path_value)
        path_layout.addWidget(self.settings_path)
        self.settings_browse_btn = QtWidgets.QPushButton('浏览')
        path_layout.addWidget(self.settings_browse_btn)
        self.settings_open_dir_btn = QtWidgets.QPushButton('打开目录')
        path_layout.addWidget(self.settings_open_dir_btn)
        layout.addLayout(path_layout)
        layout.addSpacing(6)
        
        threads_layout = QtWidgets.QHBoxLayout()
        threads_layout.addWidget(QtWidgets.QLabel('下载线程:'))
        self.threads_spin = QtWidgets.QSpinBox()
        self.threads_spin.setMinimum(1)
        self.threads_spin.setMaximum(64)
        try:
            self.threads_spin.setValue(int(cfg.get('threads', DEFAULT_THREAD_COUNT)))
        except Exception:
            self.threads_spin.setValue(DEFAULT_THREAD_COUNT)
        try:
            self.threads_spin.setButtonSymbols(QtWidgets.QAbstractSpinBox.ButtonSymbols.NoButtons)
        except Exception:
            pass
        threads_layout.addWidget(self.threads_spin)
        threads_layout.addStretch()
        layout.addLayout(threads_layout)
        layout.addSpacing(10)
        
        self.chk_mix_setting = QtWidgets.QCheckBox('合集统一下载到【合集名称】文件夹')
        self.chk_mix_setting.setChecked(bool(cfg.get('use_mix_folder', True)))
        layout.addWidget(self.chk_mix_setting)
        layout.addSpacing(4)
        
        self.chk_date_setting = QtWidgets.QCheckBox('文件名前缀增加作品发布时间')
        self.chk_date_setting.setChecked(bool(cfg.get('include_date_in_filename', True)))
        layout.addWidget(self.chk_date_setting)
        layout.addSpacing(4)
        
        self.chk_auto_select = QtWidgets.QCheckBox('作品获取完成后自动全选')
        self.chk_auto_select.setChecked(bool(cfg.get('auto_select_after_fetch', True)))
        layout.addWidget(self.chk_auto_select)
        layout.addSpacing(4)
        
        self.chk_add_title_when_export_urls = QtWidgets.QCheckBox('导出直链时增加标题')
        self.chk_add_title_when_export_urls.setChecked(bool(cfg.get('add_title_when_export_urls', False)))
        layout.addWidget(self.chk_add_title_when_export_urls)
        
        layout.addStretch()
        
        button_layout = QtWidgets.QHBoxLayout()
        self.save_settings_btn = QtWidgets.QPushButton('保存')
        self.cancel_btn = QtWidgets.QPushButton('取消')
        button_layout.addWidget(self.save_settings_btn)
        button_layout.addWidget(self.cancel_btn)
        layout.addLayout(button_layout)

        self.save_settings_btn.clicked.connect(self.save_settings)
        self.cancel_btn.clicked.connect(self.close)
        self.settings_browse_btn.clicked.connect(self.on_browse_path)
        self.settings_open_dir_btn.clicked.connect(self.on_open_directory)
        self.about_btn.clicked.connect(self.on_view_about)
        self.view_log_btn.clicked.connect(self.on_view_log)
        self.tutorial_btn.clicked.connect(self.on_view_tutorial)
        self.cookie_auto_btn.clicked.connect(self.on_cookie_auto_fetch)
        self.browser_config_btn.clicked.connect(self.on_browser_config)
        
        self.about_window = AboutWindow(self)
        self.about_window.hide()
        self.tutorial_window = TutorialWindow(self)
        self.tutorial_window.hide()
        self.cookie_fetch_window = CookieFetchWindow(self)
        self.cookie_fetch_window.hide()
        self.browser_config_window = BrowserConfigWindow(self)
        self.browser_config_window.hide()
        
        self.setStyleSheet("""QDialog {
background-color: #ffffff;
}
QLabel { color: #303133; font-size: 13px; }
QLineEdit {
border: 1px solid #dcdfe6; background: #ffffff; color: #303133;
padding: 6px; border-radius: 0px; font-size: 13px;
}
QLineEdit:focus { border: 1px solid #409EFF; background: #f9fcff; }
QTextEdit {
border: 1px solid #dcdfe6; background: #ffffff; color: #303133;
padding: 6px; border-radius: 0px; font-size: 13px;
}
QTextEdit:focus { border: 1px solid #409EFF; background: #f9fcff; }
QPushButton {
border: 1px solid #dcdfe6; background: #409EFF; color: #ffffff;
padding: 6px 12px; border-radius: 0px; font-size: 13px;
}
QPushButton:hover { background: #66b1ff; }
QPushButton:pressed {
border: 1px solid #409EFF; background: #409EFF; color: #ffffff;
}
QCheckBox { color: #303133; font-size: 13px; spacing: 8px; }
QCheckBox::indicator {
width: 16px; height: 16px; border: 1px solid #c0c4cc;
border-radius: 2px; background: #ffffff;
}
QCheckBox::indicator:hover { border: 1px solid #409EFF; }
QCheckBox::indicator:checked {
background-color: #409EFF; border: 1px solid #409EFF;
image: url(""" + self.checkmark_svg_path + r""");
}
QSpinBox {
border: 1px solid #dcdfe6; padding: 6px; background: #ffffff;
color: #303133; border-radius: 0px; font-size: 13px;
}
QSpinBox:focus { border: 1px solid #409EFF; background: #f9fcff; }
/* 底部 Save/Cancel 按钮样式 */
QPushButton#save_settings_btn, QPushButton#cancel_btn {
background-color: #409EFF; border: 1px solid #409EFF; color: white;
padding: 6px 14px; border-radius: 0px; font-weight: 500; font-size: 13px;
}
QPushButton#save_settings_btn:hover, QPushButton#cancel_btn:hover {
background-color: #66b1ff; border: 1px solid #66b1ff;
}
QPushButton#save_settings_btn:pressed, QPushButton#cancel_btn:pressed {
background-color: #3a8ee6; border: 1px solid #3a8ee6;
}
""")
        self.save_settings_btn.setObjectName("save_settings_btn")
        self.cancel_btn.setObjectName("cancel_btn")
        
        self.init_icon_selection()
    
    def init_icon_selection(self):
        """初始化图标选择状态"""
        icon_choice = cfg.get('icon_choice', 'default')
        if icon_choice == 'default':
            self.default_icon_btn.setChecked(True)
        elif icon_choice == 'alternative1':
            self.alt1_icon_btn.setChecked(True)
        elif icon_choice == 'custom':
            # 如果是自定义图标，选中自定义图标预览按钮
            self.custom_icon_preview_btn.setChecked(True)
            # 更新自定义图标预览按钮的图标
            self.update_custom_icon_preview()
        else:
            # 默认选择
            self.default_icon_btn.setChecked(True)
        
        # 确保自定义图标预览按钮的状态正确
        self.update_custom_icon_preview()
    
    def update_custom_icon_preview(self):
        """更新自定义图标预览按钮的图标"""
        if os.path.exists(CUSTOM_ICON_PATH):
            try:
                with open(CUSTOM_ICON_PATH, 'rb') as f:
                    custom_icon_data = f.read()
                pixmap = QtGui.QPixmap()
                pixmap.loadFromData(custom_icon_data)
                icon = QtGui.QIcon(pixmap)
                self.custom_icon_preview_btn.setIcon(icon)
                self.custom_icon_preview_btn.setIconSize(QtCore.QSize(26, 26))
            except Exception as e:
                print(f"Failed to load custom icon for preview: {e}")
        else:
            # 如果没有自定义图标，清空图标
            self.custom_icon_preview_btn.setIcon(QtGui.QIcon())
    
    def on_custom_icon(self):
        """处理自定义图标选择"""
        file_dialog = QtWidgets.QFileDialog(self)
        file_dialog.setNameFilter("Image Files (*.png *.jpg *.jpeg *.ico)")
        file_dialog.setFileMode(QtWidgets.QFileDialog.FileMode.ExistingFile)
        
        if file_dialog.exec():
            selected_files = file_dialog.selectedFiles()
            if selected_files:
                source_path = selected_files[0]
                try:
                    # 检查是否选择了当前的自定义图标文件
                    if os.path.abspath(source_path) == os.path.abspath(CUSTOM_ICON_PATH):
                        # 如果用户选择了当前的自定义图标文件，则不需要复制
                        QtWidgets.QMessageBox.information(self, '提示', '您选择的文件已经是当前的自定义图标文件！')
                    elif os.path.exists(source_path):
                        shutil.copy2(source_path, CUSTOM_ICON_PATH)
                        QtWidgets.QMessageBox.information(self, '成功', '自定义图标设置成功！')
                    
                    # 更新自定义图标预览按钮
                    self.update_custom_icon_preview()
                    # 选中自定义图标预览按钮
                    self.custom_icon_preview_btn.setChecked(True)
                except Exception as e:
                    QtWidgets.QMessageBox.warning(self, '错误', f'设置自定义图标失败: {str(e)}')
    
    def on_view_about(self):
        """显示关于窗口"""
        try:
            if self.about_window:
                self.about_window.show()
                self.about_window.raise_()
                self.about_window.activateWindow()
        except Exception:
            pass
    
    def on_view_log(self):
        """显示日志窗口（从主窗口获取）"""
        try:
            parent_window = self.parent()
            if parent_window and hasattr(parent_window, 'log_window'):
                log_window = getattr(parent_window, 'log_window', None)
                if log_window:
                    log_window.show()
                    log_window.raise_()
                    log_window.activateWindow()
        except Exception:
            pass
    
    def on_cookie_auto_fetch(self):
        """显示Cookie自动获取窗口"""
        try:
            if self.cookie_fetch_window:
                self.cookie_fetch_window.show()
                self.cookie_fetch_window.raise_()
                self.cookie_fetch_window.activateWindow()
        except Exception:
            pass

    def on_browser_config(self):
        """显示浏览器配置窗口"""
        try:
            if self.browser_config_window:
                self.browser_config_window.show()
                self.browser_config_window.raise_()
                self.browser_config_window.activateWindow()
        except Exception:
            pass
    
    def on_view_tutorial(self):
        """显示教程窗口"""
        try:
            if self.tutorial_window:
                self.tutorial_window.show()
                self.tutorial_window.raise_()
                self.tutorial_window.activateWindow()
        except Exception:
            pass
    
    def on_browse_path(self):
        """浏览选择保存路径"""
        dlg = QtWidgets.QFileDialog(self)
        p = dlg.getExistingDirectory(self, '选择目录', self.settings_path.text() or os.getcwd())
        if p:
            self.settings_path.setText(p)
    
    def on_open_directory(self):
        """打开设置的目录"""
        path = self.settings_path.text().strip()
        if not path:
            path = os.getcwd()
        
        if not os.path.exists(path):
            QtWidgets.QMessageBox.warning(self, '提示', f'目录不存在:\n{path}')
            return

        try:
            if sys.platform == 'win32':
                os.startfile(path)
            elif sys.platform == 'darwin':
                os.system(f'open "{path}"')
            else:
                os.system(f'xdg-open "{path}"')
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'无法打开目录:\n{path}\n\n错误信息: {e}')
    
    def refresh_settings(self):
        """刷新设置显示"""
        self.settings_cookie.setPlainText(cfg.get('cookie', ''))
        path_value = cfg.get('path', '')
        if not path_value:
            path_value = os.getcwd()
        self.settings_path.setText(path_value)
        self.chk_mix_setting.setChecked(bool(cfg.get('use_mix_folder', True)))
        self.chk_date_setting.setChecked(bool(cfg.get('include_date_in_filename', True)))
        self.chk_auto_select.setChecked(bool(cfg.get('auto_select_after_fetch', True)))
        self.chk_add_title_when_export_urls.setChecked(bool(cfg.get('add_title_when_export_urls', False)))
        try:
            self.threads_spin.setValue(int(cfg.get('threads', DEFAULT_THREAD_COUNT)))
        except Exception:
            self.threads_spin.setValue(DEFAULT_THREAD_COUNT)
            
        # 刷新图标选择
        self.init_icon_selection()
        
        # 确保浏览器路径配置正确显示
        # 不从文件重新加载浏览器路径，而是使用当前cfg中的值
        # 这样可以确保清空的路径不会被恢复
    
    def save_settings(self):
        """保存设置"""
        # 1. 将设置写入全局 cfg 变量
        cfg['cookie'] = self.settings_cookie.toPlainText().strip()
        path_value = self.settings_path.text().strip()
        if not path_value:
            path_value = os.getcwd()
        cfg['path'] = path_value
        cfg['use_mix_folder'] = bool(self.chk_mix_setting.isChecked())
        cfg['include_date_in_filename'] = bool(self.chk_date_setting.isChecked())
        cfg['auto_select_after_fetch'] = bool(self.chk_auto_select.isChecked())
        cfg['add_title_when_export_urls'] = bool(self.chk_add_title_when_export_urls.isChecked())
        cfg['threads'] = int(self.threads_spin.value())
        
        # 保存图标选择
        # 根据用户选择的按钮来决定图标类型
        checked_button = self.icon_buttons.checkedButton()
        if checked_button == self.default_icon_btn:
            cfg['icon_choice'] = 'default'
        elif checked_button == self.alt1_icon_btn:
            cfg['icon_choice'] = 'alternative1'
        else:
            # 如果没有选中预设图标按钮，检查是否应该使用自定义图标
            # 这种情况发生在用户通过"自定义"按钮选择了图标但没有选中预设按钮
            if os.path.exists(CUSTOM_ICON_PATH):
                cfg['icon_choice'] = 'custom'
            else:
                cfg['icon_choice'] = 'default'
        
        # 确保浏览器路径配置正确处理
        # 如果浏览器路径为空字符串，保持为空
        if 'chrome_path' not in cfg:
            cfg['chrome_path'] = ''
        if 'edge_path' not in cfg:
            cfg['edge_path'] = ''
        
        try:
            # 2. 持久化到 config.ini
            save_config(cfg)
            
            # 3. 通知主窗口
            parent_window = self.parent()
            if parent_window and hasattr(parent_window, 'append_log'):
                append_log_func = getattr(parent_window, 'append_log', None)
                if append_log_func:
                    append_log_func('[信息] 设置已保存')
            self.close()
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '警告', f'保存设置失败: {e}')
