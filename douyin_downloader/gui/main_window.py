#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI界面 - 主窗口
"""
import os
import re
import sys
import threading
from datetime import datetime
try:
    from PyQt6 import QtWidgets, QtCore, QtGui
    from PyQt6.QtCore import Qt
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

from douyin_downloader.constants import (
    TEXT_APP_NAME, OPENPYXL_AVAILABLE, DEFAULT_THREAD_COUNT, CONFIG_FILE, 
    ICON_BYTES_OPTIONS, CUSTOM_ICON_PATH
)
from douyin_downloader.utils.config import save_config
from douyin_downloader.utils.file_utils import sanitize_filename, safe_mkdir
from douyin_downloader.core.api import extract_sec_user_id_from_url

from douyin_downloader.gui.worker import Worker
from douyin_downloader.gui import cfg
from douyin_downloader.gui.widgets import NoFocusRectStyle
from douyin_downloader.gui.dialog_log import LogWindow
from douyin_downloader.gui.dialog_userlist import UserListWindow
from douyin_downloader.gui.dialog_settings import SettingsWindow


def get_app_icon():
    """获取应用程序图标"""
    # 从配置中获取图标选择
    icon_choice = cfg.get('icon_choice', 'default')
    
    # 如果配置为使用自定义图标，且自定义图标文件存在，则加载自定义图标文件
    if icon_choice == 'custom' and os.path.exists(CUSTOM_ICON_PATH):
        try:
            with open(CUSTOM_ICON_PATH, 'rb') as f:
                custom_icon_bytes = f.read()
            return custom_icon_bytes
        except Exception as e:
            print(f"Warning: Failed to load custom icon: {e}")
    
    # 使用预设图标
    from douyin_downloader.constants import ICON_BYTES
    return ICON_BYTES_OPTIONS.get(icon_choice, ICON_BYTES)


class MainWindow(QtWidgets.QMainWindow):
    """主窗口"""
    def __init__(self, checkmark_svg_path=''):
        super().__init__()
        self.checkmark_svg_path = checkmark_svg_path
        self.setWindowTitle(TEXT_APP_NAME)
        self.resize(1200, 700)
        # 设置窗口图标，确保任务栏也显示正确的图标
        self._set_window_icon()
        central = QtWidgets.QWidget()
        self.setCentralWidget(central)
        lay = QtWidgets.QVBoxLayout(central)

        form = QtWidgets.QGridLayout()
        lay.addLayout(form)
        # "主页链接" 标签做成按钮，点击可打开用户列表
        self.url_label_btn = QtWidgets.QPushButton('主页链接:')
        self.url_label_btn.setFlat(True)
        self.url_label_btn.setCursor(QtGui.QCursor(Qt.CursorShape.PointingHandCursor))
        form.addWidget(self.url_label_btn, 0, 0)
        self.url_edit = QtWidgets.QLineEdit()
        form.addWidget(self.url_edit, 0, 1, 1, 1)
        self.like_checkbox = QtWidgets.QCheckBox('点赞作品')
        form.addWidget(self.like_checkbox, 0, 2)
        self.fetch_btn = QtWidgets.QPushButton('获取作品')
        form.addWidget(self.fetch_btn, 0, 3)


        btns = QtWidgets.QHBoxLayout()
        lay.addLayout(btns)
        self.settings_btn = QtWidgets.QPushButton('设置')
        self.clear_btn = QtWidgets.QPushButton('清空列表')
        self.select_all_btn = QtWidgets.QPushButton('全选')
        self.invert_btn = QtWidgets.QPushButton('反选')
        self.export_urls_btn = QtWidgets.QPushButton('导出直链')
        self.export_excel_btn = QtWidgets.QPushButton('导出Excel')
        self.download_btn = QtWidgets.QPushButton('开始下载')

        btns.addWidget(self.settings_btn)
        btns.addWidget(self.export_urls_btn)
        btns.addWidget(self.export_excel_btn)
        btns.addStretch()
        btns.addWidget(self.clear_btn)
        btns.addWidget(self.select_all_btn)
        btns.addWidget(self.invert_btn)
        
        if not OPENPYXL_AVAILABLE:
            self.export_excel_btn.setEnabled(False)
            self.export_excel_btn.setToolTip("请先安装 'openpyxl' (pip install openpyxl) 以启用此功能")
        
        btns.addWidget(self.download_btn)

        self.tree = QtWidgets.QTreeWidget()
        self.tree.setStyle(NoFocusRectStyle())
        self.tree.setHeaderLabels(['选择', '序号', '发布日期', '描述', '合集', '类型 ↓'])

        self.type_filter_menu = QtWidgets.QMenu(self)
        self.type_filter_menu.setWindowFlags(Qt.WindowType.Popup | Qt.WindowType.FramelessWindowHint | Qt.WindowType.NoDropShadowWindowHint)
        self.type_filter_menu.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.type_filter_menu.setStyleSheet("""
            QMenu {
                border: 1px solid #dcdfe6;
                background-color: #ffffff;
                border-radius: 0px;
            }
            QCheckBox {
                spacing: 5px;
                padding: 4px;
            }
            QCheckBox::indicator {
                width: 16px;
                height: 16px;
                border: 1px solid #c0c4cc;
                border-radius: 0px;
                background: #ffffff;
            }
            QCheckBox::indicator:checked {
                background-color: #409EFF;
                border: 1px solid #409EFF;
                image: url(""" + self.checkmark_svg_path + """);
            }
        """)
        filter_widget = QtWidgets.QWidget()
        filter_layout = QtWidgets.QVBoxLayout(filter_widget)
        filter_layout.setContentsMargins(5, 5, 5, 5)
        filter_layout.setSpacing(2)

        self.video_checkbox = QtWidgets.QCheckBox('视频')
        self.image_checkbox = QtWidgets.QCheckBox('图片')
        self.live_checkbox = QtWidgets.QCheckBox('实况')

        self.video_checkbox.setChecked(True)
        self.image_checkbox.setChecked(True)
        self.live_checkbox.setChecked(True)

        filter_layout.addWidget(self.video_checkbox)
        filter_layout.addWidget(self.image_checkbox)
        filter_layout.addWidget(self.live_checkbox)

        filter_action = QtWidgets.QWidgetAction(self.type_filter_menu)
        filter_action.setDefaultWidget(filter_widget)
        self.type_filter_menu.addAction(filter_action)

        self.video_checkbox.stateChanged.connect(self.on_type_filter_changed)
        self.image_checkbox.stateChanged.connect(self.on_type_filter_changed)
        self.live_checkbox.stateChanged.connect(self.on_type_filter_changed)

        fm = self.tree.fontMetrics()
        width0 = fm.horizontalAdvance('选择') + 16
        col4_w = fm.horizontalAdvance('汉' * 4) + 12
        col5_w = fm.horizontalAdvance('汉' * 4) + 12
        self.tree.setColumnWidth(0, width0)
        self.tree.setColumnWidth(1, 60)
        self.tree.setColumnWidth(2, 100)
        self.tree.setColumnWidth(3, 360)
        self.tree.setColumnWidth(4, int(col4_w))
        self.tree.setColumnWidth(5, int(col5_w))

        header = self.tree.header()
        hdr_h = fm.height() + 10
        if header:
            header.setFixedHeight(int(hdr_h))
            header.setSectionResizeMode(0, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionResizeMode(1, QtWidgets.QHeaderView.ResizeMode.ResizeToContents)
            header.setSectionResizeMode(2, QtWidgets.QHeaderView.ResizeMode.ResizeToContents)
            header.setSectionResizeMode(3, QtWidgets.QHeaderView.ResizeMode.Stretch)
            header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionResizeMode(5, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionsMovable(False)
            header.setStretchLastSection(False)

        if header:
            header.sectionClicked.connect(self.on_header_section_clicked)
            header.setSectionsClickable(True)
            
        self.tree.setRootIsDecorated(False)
        self.tree.setUniformRowHeights(True)
        self.tree.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.ExtendedSelection)
        self.tree.setSelectionBehavior(QtWidgets.QAbstractItemView.SelectionBehavior.SelectRows)
        self.tree.setAttribute(QtCore.Qt.WidgetAttribute.WA_MacShowFocusRect, False)
        self.tree.setFrameShape(QtWidgets.QFrame.Shape.Box)
        self.tree.setAlternatingRowColors(True)
        self.tree.setStyleSheet(
            "QTreeWidget { background: #ffffff; border: 1px solid #e6eef8; show-decoration-selected: 0; }"
            "QTreeWidget::item { padding:6px 4px; color: #222222; outline: 0; }"
            "QTreeWidget::item:focus { outline: 0; border: 0; }"
            "QTreeWidget::item:selected { background: #e6f2ff; color: #000000; outline: 0; }"
            "QTreeWidget::item:selected:active { background: #e6f2ff; outline: 0; }"
            "QTreeWidget::item:selected:!active { background: #e6f2ff; outline: 0; }"
        )
        lay.addWidget(self.tree)

        bottom = QtWidgets.QHBoxLayout()
        lay.addLayout(bottom)
        self.progress = QtWidgets.QProgressBar()
        self.progress.setFixedHeight(26)
        self.progress.setTextVisible(True)
        self.progress.setStyleSheet(
            "QProgressBar { border: none; border-radius: 0px; background: #f0f0f0; text-align: center; }"
            "QProgressBar::chunk { background-color: #5aa6ff; border-radius: 0px; }"
        )
        bottom.addWidget(self.progress)
        self.progress.hide()

        status_layout = QtWidgets.QHBoxLayout()
        lay.addLayout(status_layout)
        self.status = QtWidgets.QLabel('')
        self.status.setCursor(QtGui.QCursor(Qt.CursorShape.PointingHandCursor))
        self.status.setMouseTracking(True)
        status_layout.addWidget(self.status)
        status_layout.addStretch()
        status_layout.addWidget(QtWidgets.QLabel('当前用户:'))
        self.nickname_label = QtWidgets.QLabel('')
        font = self.nickname_label.font()
        font.setBold(True)
        self.nickname_label.setFont(font)
        status_layout.addWidget(self.nickname_label)

        self.vtasks_all = []
        self.itasks_all = []
        self.vtasks = []
        self.itasks = []
        self.all_awemes = []
        self.current_nickname = ''

        self.log_window = LogWindow(self)
        self.log_window.hide()
        self.user_list_window = UserListWindow(self, self.checkmark_svg_path)
        self.user_list_window.hide()
        self.settings_window = SettingsWindow(self, self.checkmark_svg_path)
        self.settings_window.hide()

        self.worker = Worker()
        self._thread = None

        btn_font = QtGui.QFont()
        btn_font.setPointSize(11)
        self.like_checkbox.setFont(btn_font)
        for b in (self.fetch_btn, self.download_btn, self.settings_btn, self.clear_btn, self.select_all_btn, self.invert_btn, self.export_urls_btn):
            b.setFont(btn_font)
        button_width = 100
        self.fetch_btn.setFixedWidth(button_width)
        self.download_btn.setFixedWidth(button_width)

        self.clear_btn.setStyleSheet('''
            QPushButton {
                background: #d9534f; color: white; padding: 7px 14px;
                border: none; font-weight: 500; font-size: 13px;                 
            }
            QPushButton:hover { background: #fa8480; }
            QPushButton:disabled { background: #f0b3b3; color: #f8e6e6; }
        ''')

        self.url_label_btn.clicked.connect(self.on_show_user_list)
        self.fetch_btn.clicked.connect(self.on_fetch)
        self.download_btn.clicked.connect(self.on_download)
        self.settings_btn.clicked.connect(self.on_settings)
        self.select_all_btn.clicked.connect(self.on_select_all)
        self.export_excel_btn.clicked.connect(self.on_export_excel)
        self.export_urls_btn.clicked.connect(self.on_export_urls)
        self.invert_btn.clicked.connect(self.on_invert)
        self.clear_btn.clicked.connect(self.on_clear_list)
        self.status.mousePressEvent = lambda ev: self.on_status_click(ev)

        self.worker.log_signal.connect(self.append_log)
        self.worker.tasks_signal.connect(lambda vtasks, itasks, nickname, aweme_list: self.on_tasks_received(vtasks, itasks, nickname, aweme_list))
        self.worker.progress_signal.connect(self.on_progress)
        self.worker.finished.connect(self.on_worker_finished)
        self.worker.fetch_finished.connect(self.on_fetch_finished)
        self.worker.download_finished.connect(self.on_download_finished)
        self.worker.export_finished_signal.connect(self._on_export_finished)
        self.worker.export_error_signal.connect(self._on_export_error)

        self.tree.itemSelectionChanged.connect(self.on_tree_selection_changed)
        self.tree.itemChanged.connect(self.on_tree_item_changed)

        self._programmatic_change = False  # 防止联动循环
        self._last_status_text = ''

        if not os.path.exists(CONFIG_FILE):
            QtCore.QTimer.singleShot(500, self.show_first_time_settings)

    def append_log(self, text):
        """向日志窗口和状态栏输出日志"""
        ts = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        
        if '\n' in text:
            lines = text.split('\n')
            formatted_lines = [f"[{ts}] {line}" for line in lines]
            full_log_text = "\n".join(formatted_lines)
            # 状态栏只显示最后一条
            self._last_status_text = lines[-1]
        else:
            full_log_text = f"[{ts}] {text}"
            self._last_status_text = text

        if self.log_window:
            self.log_window.append_log(full_log_text)

        self.update_status_label()

    def update_status_label(self):
        """更新状态栏（基础文本 + 选择计数）"""
        try:
            count = 0
            for i in range(self.tree.topLevelItemCount()):
                it = self.tree.topLevelItem(i)
                if it and it.checkState(0) == Qt.CheckState.Checked:
                    count += 1
            base = getattr(self, '_last_status_text', '') or ''
            if count > 0:
                self.status.setText(f"{base} （已选择 {count} 个）")
            else:
                self.status.setText(base)
        except Exception:
            pass # 忽略更新失败
    
    def on_header_section_clicked(self, logical_index):
        """处理表头点击事件"""
        if logical_index == 5:  # 类型列
            # 切换菜单显示状态
            header = self.tree.header()
            if header:
                # 获取列的视觉区域
                left = header.sectionPosition(logical_index)
                width = header.sectionSize(logical_index)
                height = header.height()
                
                # 计算菜单显示位置，使右侧对齐
                menu_width = self.type_filter_menu.sizeHint().width()
                point = QtCore.QPoint(left + width - menu_width, height)
                global_point = self.tree.mapToGlobal(point)
                
                # 切换菜单显示/隐藏状态
                if self.type_filter_menu.isVisible():
                    self.type_filter_menu.hide()
                else:
                    # 只有在点击第5列时才在精确位置显示
                    self.type_filter_menu.popup(global_point)
    
    def on_type_filter_changed(self, state):
        """处理类型筛选变化"""
        # 防止在同步过程中再次触发
        if getattr(self, '_programmatic_change', False):
            return
        self.apply_type_filter()

    def apply_type_filter(self):
        """应用类型筛选到列表项的选择状态"""
        select_video = self.video_checkbox.isChecked()
        select_image = self.image_checkbox.isChecked()
        select_live = self.live_checkbox.isChecked()

        self.tree.setUpdatesEnabled(False)
        try:
            for i in range(self.tree.topLevelItemCount()):
                item = self.tree.topLevelItem(i)
                if item:
                    item_type = item.text(5)  # 第5列是类型列
                    
                    should_select = False
                    if item_type == '视频' and select_video:
                        should_select = True
                    elif item_type == '图片' and select_image:
                        should_select = True
                    elif item_type == '实况' and select_live:
                        should_select = True

                    # 如果所有类型都被选中，则选中所有项
                    if select_video and select_image and select_live:
                        should_select = True

                    if should_select:
                        item.setCheckState(0, Qt.CheckState.Checked)
                    else:
                        item.setCheckState(0, Qt.CheckState.Unchecked)
        finally:
            self.tree.setUpdatesEnabled(True)
        
        self.sync_filter_checkboxes()

    def sync_filter_checkboxes(self):
        """同步筛选复选框状态"""
        # 防止在同步过程中再次触发
        if getattr(self, '_programmatic_change', False):
            return
        
        # 统计各类型项的选中情况
        video_total = 0
        video_selected = 0
        image_total = 0
        image_selected = 0
        live_total = 0
        live_selected = 0
        
        for i in range(self.tree.topLevelItemCount()):
            item = self.tree.topLevelItem(i)
            if item:
                item_type = item.text(5)  # 第5列是类型列
                is_selected = item.checkState(0) == Qt.CheckState.Checked
                
                if item_type == '视频':
                    video_total += 1
                    if is_selected:
                        video_selected += 1
                elif item_type == '图片':
                    image_total += 1
                    if is_selected:
                        image_selected += 1
                elif item_type == '实况':
                    live_total += 1
                    if is_selected:
                        live_selected += 1
        
        self._programmatic_change = True  # 防止循环触发
        try:
            if video_total > 0:
                if video_selected == video_total:
                    self.video_checkbox.setCheckState(Qt.CheckState.Checked)
                elif video_selected == 0:
                    self.video_checkbox.setCheckState(Qt.CheckState.Unchecked)
                else:
                    self.video_checkbox.setCheckState(Qt.CheckState.PartiallyChecked)
            
            if image_total > 0:
                if image_selected == image_total:
                    self.image_checkbox.setCheckState(Qt.CheckState.Checked)
                elif image_selected == 0:
                    self.image_checkbox.setCheckState(Qt.CheckState.Unchecked)
                else:
                    self.image_checkbox.setCheckState(Qt.CheckState.PartiallyChecked)
            
            if live_total > 0:
                if live_selected == live_total:
                    self.live_checkbox.setCheckState(Qt.CheckState.Checked)
                elif live_selected == 0:
                    self.live_checkbox.setCheckState(Qt.CheckState.Unchecked)
                else:
                    self.live_checkbox.setCheckState(Qt.CheckState.PartiallyChecked)
        finally:
            self._programmatic_change = False

    def on_tree_selection_changed(self):
        """处理列表选择变化 (行选 -> 勾选)"""
        if getattr(self, '_programmatic_change', False):
            return
        self._programmatic_change = True
        try:
            self.tree.setUpdatesEnabled(False)
            try:
                for i in range(self.tree.topLevelItemCount()):
                    it = self.tree.topLevelItem(i)
                    if it:
                        if it.isSelected():
                            it.setCheckState(0, Qt.CheckState.Checked)
                        else:
                            it.setCheckState(0, Qt.CheckState.Unchecked)
            finally:
                self.tree.setUpdatesEnabled(True)
        finally:
            self._programmatic_change = False
        self.update_status_label()
        self.sync_filter_checkboxes()

    def on_tree_item_changed(self, item, column):
        """处理复选框变化 (勾选 -> 行选)"""
        if getattr(self, '_programmatic_change', False):
            return
        self._programmatic_change = True
        try:
            if column == 0:
                state = item.checkState(0)
                if state == Qt.CheckState.Checked:
                    item.setSelected(True)
                else:
                    item.setSelected(False)
        finally:
            self._programmatic_change = False
        self.update_status_label()
        self.sync_filter_checkboxes()

    def on_progress(self, done, total):
        """更新进度条"""
        if not self.progress.isVisible():
            self.progress.show()
            
        self.progress.setMaximum(total)
        self.progress.setValue(done)
        pct = int((done / max(1, total)) * 100)
        self.progress.setFormat(f"%v / %m ({pct}%)")
        
        # 完成时变绿
        try:
            if total > 0 and done >= total:
                self.progress.setStyleSheet(
                    "QProgressBar { border: none; border-radius: 0px; background: #f0f0f0; text-align: center; }"
                    "QProgressBar::chunk { background-color: #4CC14C; border-radius: 0px; }"
                )
            else:
                # 恢复进行中颜色（蓝色）
                self.progress.setStyleSheet(
                    "QProgressBar { border: none; border-radius: 0px; background: #f0f0f0; text-align: center; }"
                    "QProgressBar::chunk { background-color: #5aa6ff; border-radius: 0px; }"
                )
        except Exception:
            pass
    
    def on_download_finished(self):
        """下载完成处理（确保进度条是绿色）"""
        try:
            maxv = self.progress.maximum() or self.progress.value() or 1
            self.progress.setValue(maxv)
            if hasattr(self.worker, '_download_stop_requested') and self.worker._download_stop_requested:
                self.progress.setFormat(f"%v / %m (已停止)")
                self.progress.hide()
            else:
                self.progress.setFormat(f"%v / %m (完成)")
            self.progress.setStyleSheet(
                "QProgressBar { border: none; border-radius: 0px; background: #f0f0f0; text-align: center; }"
                "QProgressBar::chunk { background-color: #4CC14C; border-radius: 0px; }"
            )
        except Exception:
            pass

    def on_export_excel(self):
        """导出Excel表格"""
        if not hasattr(self, 'all_awemes') or not self.all_awemes:
            QtWidgets.QMessageBox.warning(self, '提示', '没有作品数据可以导出')
            return

        self.export_excel_btn.setText('正在导出')
        self.export_excel_btn.setEnabled(False)

        all_awemes_copy = list(self.all_awemes)
        nickname = self.nickname_label.text() or '抖音用户'
        unique_id = getattr(self, 'current_unique_id', '') or ''
        if unique_id:
            excel_nickname = f"{nickname}-{unique_id}"
        else:
            excel_nickname = nickname

        if getattr(self, '_fetch_mode', '') == 'favorite':
            excel_nickname += '-like'

        base_folder = cfg.get('path', '') or os.getcwd()
        excel_base_folder = os.path.join(base_folder, '作品数据Excel')

        export_thread = threading.Thread(
            target=self.worker.export_excel, 
            args=(all_awemes_copy, excel_nickname, excel_base_folder), 
            daemon=True
        )
        export_thread.start()

    def on_export_urls(self):
        """导出视频直链"""
        if not hasattr(self, 'all_awemes') or not self.all_awemes:
            QtWidgets.QMessageBox.warning(self, '提示', '没有作品数据可以导出')
            return

        video_awemes = [aweme for aweme in self.all_awemes if not aweme.get('images')]
        if not video_awemes:
            QtWidgets.QMessageBox.warning(self, '提示', '没有视频作品可以导出直链')
            return

        try:
            base_folder = cfg.get('path', '') or os.getcwd()
            urls_folder = os.path.join(base_folder, '视频直链')

            try:
                if not os.path.exists(urls_folder):
                    os.makedirs(urls_folder, exist_ok=True)
            except Exception as e:
                QtWidgets.QMessageBox.critical(self, '错误', f'创建目录失败: {urls_folder}\n错误信息: {str(e)}')
                return

            nickname = self.nickname_label.text() or '抖音用户'
            unique_id = getattr(self, 'current_unique_id', '') or ''
            
            if unique_id:
                filename = f"{sanitize_filename(nickname)}-{unique_id}"
            else:
                filename = f"{sanitize_filename(nickname)}"

            if getattr(self, '_fetch_mode', '') == 'favorite':
                filename += '-like'
            filename += '.txt'
            filepath = os.path.join(urls_folder, filename)

            urls = []
            descs = []
            for aweme in video_awemes:
                video_info = aweme.get('video', {})
                if video_info:
                    bit_rate_list = video_info.get('bit_rate', [])
                    if bit_rate_list:
                        best = max(bit_rate_list, key=lambda x: x.get('bit_rate', 0))
                        url_list = best.get('play_addr', {}).get('url_list', [])
                        if len(url_list) >= 3:
                            full_url = url_list[2]
                            import re
                            video_id_match = re.search(r'video_id=([^&]*)', full_url)
                            file_id_match = re.search(r'file_id=([^&]*)', full_url)
                            if video_id_match and file_id_match:
                                video_id = video_id_match.group(1)
                                file_id = file_id_match.group(1)
                                simplified_url = f"https://www.douyin.com/aweme/v1/play/?video_id={video_id}&file_id={file_id}"
                                urls.append(simplified_url)
                                descs.append(aweme.get('desc', ''))

            add_title = cfg.get('add_title_when_export_urls', False)
            if add_title:
                if unique_id:
                    filename = f"{sanitize_filename(nickname)}-{unique_id}"
                else:
                    filename = f"{sanitize_filename(nickname)}"

                if getattr(self, '_fetch_mode', '') == 'favorite':
                    filename += '-like'
                filename += '_desc.txt'
                filepath = os.path.join(urls_folder, filename)

                try:
                    with open(filepath, 'w', encoding='utf-8') as f:
                        for i, (desc, url) in enumerate(zip(descs, urls), 1):
                            f.write(f"{i}.{desc}\n{url}\n\n")
                except Exception as e:
                    QtWidgets.QMessageBox.critical(self, '错误', f'写入文件失败: {filepath}\n错误信息: {str(e)}')
                    return
            else:
                try:
                    with open(filepath, 'w', encoding='utf-8') as f:
                        for url in urls:
                            f.write(url + '\n')
                except Exception as e:
                    QtWidgets.QMessageBox.critical(self, '错误', f'写入文件失败: {filepath}\n错误信息: {str(e)}')
                    return

            # 提示成功
            msg_box = QtWidgets.QMessageBox(self)
            msg_box.setWindowTitle('导出成功')
            msg_box.setText(f'视频直链已保存至:\n{filepath}')
            msg_box.setStandardButtons(QtWidgets.QMessageBox.StandardButton.Ok)
            ok_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Ok)
            if ok_button: ok_button.setText('确认')
            msg_box.exec()

        except Exception as e:
            QtWidgets.QMessageBox.critical(self, '错误', f'导出直链失败: {str(e)}')
            self.append_log(f'[错误] 导出直链失败: {str(e)}')

    def _on_export_finished(self, filepath):
        """导出完成后的UI更新"""
        self.export_excel_btn.setText('导出Excel')
        self.export_excel_btn.setEnabled(True)
        msg_box = QtWidgets.QMessageBox(self)
        msg_box.setWindowTitle('导出成功')
        msg_box.setText(f'Excel文件已保存至:\n{filepath}')
        msg_box.setStandardButtons(QtWidgets.QMessageBox.StandardButton.Ok)
        ok_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Ok)
        if ok_button: ok_button.setText('确认')
        msg_box.exec()

    def _on_export_error(self, error_msg):
        """导出失败后的UI更新"""
        self.export_excel_btn.setText('导出Excel表格')
        self.export_excel_btn.setEnabled(True)
        QtWidgets.QMessageBox.warning(self, '导出失败', error_msg)
        self.append_log(error_msg)

    def on_tasks_received(self, vtasks, itasks, user_info, aweme_list):
        """接收 Worker 增量获取到的作品任务"""
        self.progress.hide()

        nickname = user_info
        unique_id = ''
        if '|' in user_info:
            parts = user_info.split('|', 1)
            nickname = parts[0]
            unique_id = parts[1]

        self.nickname_label.setText(nickname or '')

        self.current_nickname = nickname or ''
        self.current_unique_id = unique_id

        if not hasattr(self, 'vtasks_all'): self.vtasks_all = []
        if not hasattr(self, 'itasks_all'): self.itasks_all = []
        self.vtasks_all.extend(vtasks or [])
        self.itasks_all.extend(itasks or [])

        # 使用Worker中累积的所有aweme数据，而不是只使用当前批次的数据
        if hasattr(self.worker, 'all_awemes'):
            self.all_awemes = self.worker.all_awemes
        else:
            if not hasattr(self, 'all_awemes'): self.all_awemes = []
            self.all_awemes.extend(aweme_list or [])

        def get_type_display(desc, is_image):
            if not is_image: return '视频'
            if desc and isinstance(desc, str):
                if re.search(r'_live\d*$', desc) or '_live' in desc:
                    return '实况'
                if re.search(r'_p\d+$', desc):
                    return '图片'
            return '图片'

        items_to_add = []
        idx = self.tree.topLevelItemCount() + 1
        
        for t in (vtasks or []):
            date_display = t.get('date', '')
            desc_display = t.get('desc', '')
            item = QtWidgets.QTreeWidgetItem([
                ' ', str(idx), date_display, desc_display, 
                t.get('mix_name') or '', '视频'
            ])
            item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
            item.setCheckState(0, Qt.CheckState.Unchecked)
            item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
            item.setData(0, Qt.ItemDataRole.UserRole, (t, False)) # (task, is_image=False)
            items_to_add.append(item)
            idx += 1

        for t in (itasks or []):
            date_display = t.get('date', '')
            desc_display = t.get('desc', '')
            kind = get_type_display(desc_display, True)
            item = QtWidgets.QTreeWidgetItem([
                ' ', str(idx), date_display, desc_display, 
                t.get('mix_name') or '', kind
            ])
            item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
            item.setCheckState(0, Qt.CheckState.Unchecked)
            item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
            item.setData(0, Qt.ItemDataRole.UserRole, (t, True)) # (task, is_image=True)
            items_to_add.append(item)
            idx += 1

        # 批量添加项目到树形控件，分批添加以提高UI响应性
        if items_to_add:
            BATCH_SIZE = 100  # 每批添加100个项目
            total_items = len(items_to_add)
            
            # 如果项目数量较少，一次性添加
            if total_items <= BATCH_SIZE:
                self.tree.setUpdatesEnabled(False)  # 暂时禁用更新以提高性能
                try:
                    self.tree.addTopLevelItems(items_to_add)
                finally:
                    self.tree.setUpdatesEnabled(True)  # 重新启用更新
            else:
                # 分批添加项目
                for i in range(0, total_items, BATCH_SIZE):
                    batch = items_to_add[i:i + BATCH_SIZE]
                    self.tree.setUpdatesEnabled(False)  # 暂时禁用更新以提高性能
                    try:
                        self.tree.addTopLevelItems(batch)
                        # 处理事件队列，保持UI响应
                        QtWidgets.QApplication.processEvents()
                    finally:
                        self.tree.setUpdatesEnabled(True)  # 重新启用更新
                                
            # 有项目时显示相关按钮
            self.clear_btn.setVisible(True)
            self.select_all_btn.setVisible(True)
            self.invert_btn.setVisible(True)
                


        self.vtasks = list(self.vtasks_all)
        self.itasks = list(self.itasks_all)

        self.tree.repaint()

        self.sync_filter_checkboxes()

    def showEvent(self, a0):
        """窗口显示事件（用于修复列宽）"""
        try:
            header = self.tree.header()
            fm = self.tree.fontMetrics()
            target_px = fm.horizontalAdvance('汉' * 6) + 12
            if header:
                header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
                header.resizeSection(4, int(target_px))
        except Exception:
            pass
        return super().showEvent(a0)

    def resizeEvent(self, a0):
        """窗口大小调整事件（用于修复列宽）"""
        try:
            header = self.tree.header()
            fm = self.tree.fontMetrics()
            target_px = fm.horizontalAdvance('汉' * 4) + 12
            if header:
                header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
                header.resizeSection(4, int(target_px))
        except Exception:
            pass
        return super().resizeEvent(a0)
    
    def on_show_user_list(self):
        """显示用户列表窗口"""
        try:
            if self.user_list_window:
                self.user_list_window.load_users()
                self.user_list_window.show()
                self.user_list_window.raise_()
                self.user_list_window.activateWindow()
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'无法打开用户列表: {e}')

    def on_fetch(self):
        """获取作品 / 停止获取"""
        if self.fetch_btn.text() == '停止获取':
            try:
                if hasattr(self.worker, '_fetch_stop_requested'):
                    self.worker._fetch_stop_requested = True
                self.append_log('[信息] 已请求停止获取')
                # 立即更新按钮状态
                self.fetch_btn.setText('获取作品')
                self.fetch_btn.setProperty("running", False)
                style = self.style()
                if style:
                    style.unpolish(self.fetch_btn)
                    style.polish(self.fetch_btn)
                self.url_label_btn.setEnabled(True)
                self.settings_btn.setEnabled(True)
                self.clear_btn.setEnabled(True)
                self.select_all_btn.setEnabled(True)
                self.export_excel_btn.setEnabled(True)
                self.export_urls_btn.setEnabled(True)
                self.invert_btn.setEnabled(True)
                self.download_btn.setEnabled(True)
                self.like_checkbox.setEnabled(True)
            except Exception:
                pass
            if hasattr(self, '_thread') and self._thread and self._thread.is_alive():
                self._thread.join(timeout=3)
            return

        url = self.url_edit.text().strip()
        if not url:
            QtWidgets.QMessageBox.warning(self, '提示', '请输入主页链接')
            return
        cookie = cfg.get('cookie', '')
        if not cookie:
            QtWidgets.QMessageBox.warning(self, '提示', '请在设置中配置 Cookie')
            return
        
        self.url_label_btn.setEnabled(False)
        self.settings_btn.setEnabled(False)
        self.clear_btn.setEnabled(False)
        self.select_all_btn.setEnabled(False)
        self.export_excel_btn.setEnabled(False)
        self.export_urls_btn.setEnabled(False)
        self.invert_btn.setEnabled(False)
        self.download_btn.setEnabled(False)
        self.like_checkbox.setEnabled(False)

        try:
            self.tree.clear()
            self.vtasks_all = []
            self.itasks_all = []
            self.vtasks = []
            self.itasks = []
            self.all_awemes = []  # 清空aweme数据
            self.current_nickname = '' 
            if hasattr(self.worker, 'all_awemes'): self.worker.all_awemes = []  # 清空Worker中的aweme数据
            if hasattr(self.worker, '_completed_tasks'): self.worker._completed_tasks = []
            if hasattr(self.worker, '_failed_tasks'): self.worker._failed_tasks = []
            if hasattr(self.worker, '_total_received'): self.worker._total_received = 0
            self.progress.setValue(0)
            self.progress.hide()
            self.status.setText('')
            self.append_log('[信息] 已清空上次获取的列表')
            

        except Exception:
            pass
        
        fetch_mode = 'favorite' if self.like_checkbox.isChecked() else 'post'
        self._fetch_mode = fetch_mode
        btn_text = '停止获取'
        self.fetch_btn.setText(btn_text)
        self.fetch_btn.setEnabled(True)
        self.fetch_btn.setProperty("running", True)
        style = self.style()
        if style:
            style.unpolish(self.fetch_btn)
            style.polish(self.fetch_btn)

        self.worker._fetch_stop_requested = False
        self._thread = threading.Thread(target=self.worker.fetch_tasks, args=(url, cookie, fetch_mode), daemon=True)
        self._thread.start()

    def closeEvent(self, a0):
        """窗口关闭事件"""
        running_tasks = False
        if hasattr(self, '_thread') and self._thread and self._thread.is_alive():
            running_tasks = True
        elif hasattr(self.worker, '_pause_requested') and getattr(self.worker, '_pause_requested', False):
            running_tasks = True
        
        if running_tasks:
            msg_box = QtWidgets.QMessageBox(self)
            msg_box.setWindowTitle('确认退出')
            msg_box.setText('检测到有下载任务正在运行，确定要关闭程序吗？\n\n注意：关闭程序将终止所有正在进行的下载任务。')
            msg_box.setStandardButtons(QtWidgets.QMessageBox.StandardButton.Ok | QtWidgets.QMessageBox.StandardButton.Cancel)
            msg_box.setDefaultButton(QtWidgets.QMessageBox.StandardButton.Cancel)
            ok_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Ok)
            cancel_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Cancel)
            if ok_button: ok_button.setText('确认退出')
            if cancel_button: cancel_button.setText('取消')
            
            if msg_box.exec() != QtWidgets.QMessageBox.StandardButton.Ok:
                if a0:
                    a0.ignore()
                return
        
        # 确认关闭
        try:
            # 请求停止所有任务
            if hasattr(self.worker, '_fetch_stop_requested'):
                self.worker._fetch_stop_requested = True
            if hasattr(self.worker, '_pause_requested'):
                self.worker._pause_requested = True
            if hasattr(self.worker, '_download_stop_requested'):
                self.worker._download_stop_requested = True
            
            # 等待线程（最多5秒）
            if hasattr(self, '_thread') and self._thread and self._thread.is_alive():
                # 先尝试等待一小段时间
                self._thread.join(timeout=1.0)
                # 如果线程仍然活跃，强制设置标志并再次等待
                if self._thread.is_alive():
                    # 再等待4秒
                    self._thread.join(timeout=4.0)
                
            # 关闭所有子窗口
            for w in (self.log_window, self.user_list_window, self.settings_window):
                if w: 
                    try:
                        w.close()
                    except:
                        pass  # 忽略子窗口关闭时的异常
                
        except Exception as e:
            print(f"[警告] 关闭时清理资源出现异常: {e}")
        
        if a0:
            a0.accept()

    def on_download(self):
        """开始下载按钮处理"""
        # 检查是否正在下载（切换为停止）
        if self.download_btn.text() == '停止下载':
            try:
                if hasattr(self.worker, '_download_stop_requested'):
                    self.worker._download_stop_requested = True
                self.append_log('[信息] 已请求停止下载')
                # 立即更新按钮状态
                self.download_btn.setText('开始下载')
                
                # 设置 "running" 属性为 False，QSS会自动应用蓝色样式
                self.download_btn.setProperty("running", False)
                style = self.style()
                if style:
                    style.unpolish(self.download_btn)
                    style.polish(self.download_btn)
                self.progress.hide()

                self.url_label_btn.setEnabled(True)
                self.settings_btn.setEnabled(True)
                self.clear_btn.setEnabled(True)
                self.select_all_btn.setEnabled(True)
                self.export_excel_btn.setEnabled(True)
                self.export_urls_btn.setEnabled(True)
                self.invert_btn.setEnabled(True)
                self.fetch_btn.setEnabled(True)
                self.like_checkbox.setEnabled(True)
            except Exception:
                pass
            return

        selected = []
        for i in range(self.tree.topLevelItemCount()):
            it = self.tree.topLevelItem(i)
            if it and it.checkState(0) == Qt.CheckState.Checked:
                data = it.data(0, Qt.ItemDataRole.UserRole)
                if data:
                    selected.append(data)

        if not selected:
            QtWidgets.QMessageBox.warning(self, '提示', '请先选择要下载的作品')
            return

        sel_v = [d[0] for d in selected if not d[1]]  # (task, is_image=False)
        sel_i = [d[0] for d in selected if d[1]]  # (task, is_image=True)

        base_folder = cfg.get('path', '') or os.getcwd()
        nickname_for_folder = self.nickname_label.text() or 'Douyin_User'  # 移除了对last_nickname字段的依赖
        unique_id = getattr(self, 'current_unique_id', '') or ''
        
        # 使用用户名-unique_id作为文件夹名
        if unique_id:
            folder_name = f"{nickname_for_folder}-{unique_id}"
        else:
            folder_name = nickname_for_folder or 'Douyin_Downloads'

        if getattr(self, '_fetch_mode', '') == 'favorite':
            folder_name += '-like'

        # 修改路径结构为: 基础路径/作品下载/用户名-unique_id
        download_folder = os.path.join(base_folder, '作品下载')
        user_folder = os.path.join(download_folder, sanitize_filename(folder_name))
        
        if not safe_mkdir(user_folder):
            QtWidgets.QMessageBox.critical(self, '错误', f'创建目录失败: {user_folder}')
            return
            
        threads = int(cfg.get('threads', DEFAULT_THREAD_COUNT))
        use_mix_folder = cfg.get('use_mix_folder', True)
        include_date = cfg.get('include_date_in_filename', True)

        def apply_settings_to_tasks(tasks, is_image):
            out = []
            for t in tasks:
                nt = dict(t)
                if not use_mix_folder:
                    nt['mix_name'] = None

                # 不再处理 desc 字符串，而是将配置存入 task
                nt['include_date_in_filename'] = include_date
                
                out.append(nt)
            return out

        sel_v_proc = apply_settings_to_tasks(sel_v, False)
        sel_i_proc = apply_settings_to_tasks(sel_i, True)

        self.progress.show()
        self.progress.setMaximum(max(1, len(sel_v_proc) + len(sel_i_proc)))
        self.progress.setValue(0)
        self.on_progress(0, max(1, len(sel_v_proc) + len(sel_i_proc))) # 恢复蓝色
        
        # 重置停止标志
        self.worker._download_stop_requested = False
        self.worker._pause_requested = False

        # 禁用所有按钮
        self.url_label_btn.setEnabled(False)
        self.settings_btn.setEnabled(False)
        self.clear_btn.setEnabled(False)
        self.select_all_btn.setEnabled(False)
        self.export_excel_btn.setEnabled(False)
        self.export_urls_btn.setEnabled(False)
        self.invert_btn.setEnabled(False)
        self.fetch_btn.setEnabled(False)
        self.like_checkbox.setEnabled(False)

        # 设置下载按钮为停止下载按钮
        self.download_btn.setText('停止下载')
        self.download_btn.setEnabled(True)
        self.download_btn.setProperty("running", True)
        style = self.style()
        if style:
            style.unpolish(self.download_btn)
            style.polish(self.download_btn)
        self._thread = threading.Thread(
            target=self.worker.download_tasks, 
            args=(sel_v_proc, sel_i_proc, user_folder, threads), 
            daemon=True
        )
        self._thread.start()

    def on_fetch_finished(self):
        """获取完成处理（用于自动全选 和 保存用户）"""
        try:
            url = self.url_edit.text().strip()
            nickname = self.current_nickname # 使用暂存的昵称
            
            if url and nickname:
                current_sec_user_id = extract_sec_user_id_from_url(url)
                if current_sec_user_id:
                    normalized_url = f"https://www.douyin.com/user/{current_sec_user_id}"

                    users = cfg.get('users', [])
                    existing_user = None
                    existing_user_index = -1

                    for idx, user in enumerate(users):
                        user_url = user.get('url', '')
                        user_sec_user_id = extract_sec_user_id_from_url(user_url)
                        if user_sec_user_id == current_sec_user_id:
                            existing_user = user
                            existing_user_index = idx
                            break
                    
                    if not existing_user:
                        users.append({'username': nickname, 'url': normalized_url})
                        cfg['users'] = users
                        save_config(cfg)
                        self.append_log(f'[信息] 已保存用户: {nickname}')
                    else:
                        users[existing_user_index]['username'] = nickname
                        users[existing_user_index]['url'] = normalized_url
                        cfg['users'] = users
                        save_config(cfg)
                        self.append_log(f'[信息] 已更新用户: {nickname}')
        except Exception as e:
            self.append_log(f'[警告] 保存用户信息失败: {e}')

        try:
            if bool(cfg.get('auto_select_after_fetch', False)):
                self.on_select_all()
                self.append_log('[信息] 获取完成，已自动全选')
            else:
                self.append_log('[信息] 获取完成')
        except Exception as e:
            self.append_log(f'[警告] 获取完成处理失败: {e}')

    def on_settings(self):
        """显示设置窗口"""
        try:
            if self.settings_window:
                # 刷新设置窗口的显示内容
                self.settings_window.refresh_settings()
                self.settings_window.show()
                self.settings_window.raise_()
                self.settings_window.activateWindow()
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'无法打开设置窗口: {e}')
    
    def show_first_time_settings(self):
        """首次启动时显示设置窗口"""
        msg_box = QtWidgets.QMessageBox(self)
        msg_box.setWindowTitle('欢迎使用')
        msg_box.setText(
            '欢迎使用抖音主页作品批量下载工具！\n\n'
            '检测到这是您第一次使用本程序，\n'
            '请先配置 Cookie 和保存路径。\n\n'
            '点击“确认”将打开设置窗口。'
        )
        msg_box.setStandardButtons(QtWidgets.QMessageBox.StandardButton.Ok)
        ok_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Ok)
        if ok_button: ok_button.setText('确认')
        msg_box.exec()
        
        self.on_settings()
    
    def on_status_click(self, event):
        """点击状态标签打开日志窗口"""
        try:
            if self.log_window:
                self.log_window.show()
                self.log_window.raise_()
                self.log_window.activateWindow()
        except Exception:
            pass

    def on_select_all(self):
        """全选"""
        self._programmatic_change = True
        try:
            self.tree.setUpdatesEnabled(False)
            try:
                for i in range(self.tree.topLevelItemCount()):
                    it = self.tree.topLevelItem(i)
                    if it:
                        it.setCheckState(0, Qt.CheckState.Checked)
                        it.setSelected(True)
            finally:
                self.tree.setUpdatesEnabled(True)
        finally:
            self._programmatic_change = False
        self.update_status_label()
        self.sync_filter_checkboxes()

    def on_invert(self):
        """反选"""
        self._programmatic_change = True
        try:
            self.tree.setUpdatesEnabled(False)
            try:
                for i in range(self.tree.topLevelItemCount()):
                    it = self.tree.topLevelItem(i)
                    if it:
                        current_state = it.checkState(0)
                        new_state = Qt.CheckState.Unchecked if current_state == Qt.CheckState.Checked else Qt.CheckState.Checked
                        it.setCheckState(0, new_state)
                        it.setSelected(new_state == Qt.CheckState.Checked)
            finally:
                self.tree.setUpdatesEnabled(True)
        finally:
            self._programmatic_change = False
        self.update_status_label()
        self.sync_filter_checkboxes()

    def on_clear_list(self):
        """清空列表"""
        msg_box = QtWidgets.QMessageBox(self)
        msg_box.setWindowTitle('确认')
        msg_box.setText('确定要清空当前列表吗？此操作不会删除已下载的文件。')
        msg_box.setStandardButtons(QtWidgets.QMessageBox.StandardButton.Ok | QtWidgets.QMessageBox.StandardButton.Cancel)
        msg_box.setDefaultButton(QtWidgets.QMessageBox.StandardButton.Cancel)
        ok_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Ok)
        cancel_button = msg_box.button(QtWidgets.QMessageBox.StandardButton.Cancel)
        if ok_button: ok_button.setText('确认')
        if cancel_button: cancel_button.setText('取消')
        
        if msg_box.exec() != QtWidgets.QMessageBox.StandardButton.Ok:
            return
        
        try:
            self.tree.clear()
            self.vtasks_all = []
            self.itasks_all = []
            self.vtasks = []
            self.itasks = []
            self.all_awemes = []  # 同时清空aweme数据
            self.current_nickname = ''
            self.progress.setValue(0)
            self.progress.hide()
            

        except Exception:
            pass
        
        self.append_log('[信息] 已清空当前列表')

    def on_worker_finished(self):
        """工作线程完成处理（Fetch 或 Download）"""
        self.url_label_btn.setEnabled(True)
        self.settings_btn.setEnabled(True)
        self.clear_btn.setEnabled(True)
        self.select_all_btn.setEnabled(True)
        if OPENPYXL_AVAILABLE:
            self.export_excel_btn.setEnabled(True)
        self.export_urls_btn.setEnabled(True)
            
        self.invert_btn.setEnabled(True)
        self.download_btn.setEnabled(True)
        self.fetch_btn.setEnabled(True)
        self.fetch_btn.setText('获取作品')
        self.like_checkbox.setEnabled(True)
        self.download_btn.setText('开始下载')
        
        # 设置 "running" 属性为 False，QSS会自动应用蓝色样式
        self.fetch_btn.setProperty("running", False)
        style = self.style()
        if style:
            style.unpolish(self.fetch_btn)
            style.polish(self.fetch_btn)
        self.download_btn.setProperty("running", False)
        style = self.style()
        if style:
            style.unpolish(self.download_btn)
            style.polish(self.download_btn)

        # 不再隐藏进度条，保持显示下载完成状态

        # 如果进度条是满的，确保是绿色
        if self.progress.value() == self.progress.maximum() and self.progress.maximum() > 0:
            self.on_download_finished()
        
        # 如果是用户主动停止下载，也调用下载完成的处理
        if hasattr(self.worker, '_download_stop_requested') and self.worker._download_stop_requested:
            self.on_download_finished()
        # 如果是用户主动停止下载，隐藏进度条
        elif hasattr(self.worker, '_download_stop_requested') and self.worker._download_stop_requested:
            self.progress.hide()

    def _set_window_icon(self):
        """设置窗口图标，确保任务栏也显示正确的图标"""
        try:
            # Windows系统特殊处理任务栏图标
            if sys.platform.startswith('win'):
                import ctypes
                myappid = 'douyin.downloader.app'  # 设置应用程序用户模型ID
                ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)
            
            icon_bytes = get_app_icon()
            pixmap = QtGui.QPixmap()
            pixmap.loadFromData(icon_bytes)
            icon = QtGui.QIcon(pixmap)
            self.setWindowIcon(icon)
        except Exception as e:
            print(f"Warning: Failed to set window icon: {e}")
