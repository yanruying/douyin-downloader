#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - User List Dialog
"""
import sys
try:
    from PyQt6 import QtWidgets, QtCore
    from PyQt6.QtCore import Qt
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

from douyin_downloader.gui import cfg
from douyin_downloader.utils.config import save_config
from douyin_downloader.core.api import extract_sec_user_id_from_url
from .widgets import NoFocusRectStyle

class UserListWindow(QtWidgets.QDialog):
    """用户列表窗口"""
    
    def __init__(self, parent=None, checkmark_svg_path=''):
        super().__init__(parent)
        self.checkmark_svg_path = checkmark_svg_path
        self.setWindowTitle('主页链接')
        self.setModal(False)
        self.resize(800, 500)
        
        layout = QtWidgets.QVBoxLayout(self)
        
        self.user_tree = QtWidgets.QTreeWidget()
        self.user_tree.setStyle(NoFocusRectStyle())  # 不绘制焦点虚线框，保持界面干净
        self.user_tree.setHeaderLabels(['选择', '序号', '用户名', '主页链接', '操作'])
        self.user_tree.setRootIsDecorated(False)
        self.user_tree.setUniformRowHeights(False)
        self.user_tree.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.ExtendedSelection)
        self.user_tree.setSelectionBehavior(QtWidgets.QAbstractItemView.SelectionBehavior.SelectRows)
        self.user_tree.setAttribute(QtCore.Qt.WidgetAttribute.WA_MacShowFocusRect, False)
        
        fm = self.user_tree.fontMetrics()
        width0 = fm.horizontalAdvance('选择') + 16
        col4_w = fm.horizontalAdvance('汉' * 4) + 12
        self.user_tree.setColumnWidth(0, width0)
        self.user_tree.setColumnWidth(1, 60)
        self.user_tree.setColumnWidth(2, 100)
        self.user_tree.setColumnWidth(4, int(col4_w))
        
        header = self.user_tree.header()
        if header:
            header.setSectionResizeMode(0, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionResizeMode(1, QtWidgets.QHeaderView.ResizeMode.ResizeToContents)
            header.setSectionResizeMode(2, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionResizeMode(3, QtWidgets.QHeaderView.ResizeMode.Stretch)
            header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionsMovable(False)
            header.setStretchLastSection(False)
        
        layout.addWidget(self.user_tree)
        
        btn_layout = QtWidgets.QHBoxLayout()
        self.select_all_btn = QtWidgets.QPushButton('全选')
        self.delete_btn = QtWidgets.QPushButton('删除')
        self.delete_btn.setStyleSheet('''
            QPushButton {
                background: #d9534f; color: white; padding: 7px 14px;
                border: none; font-weight: 500; font-size: 13px;                 
            }
            QPushButton:hover { background: #fa8480; }
        ''')
        self.close_btn = QtWidgets.QPushButton('关闭')
        
        btn_layout.addWidget(self.select_all_btn)
        btn_layout.addWidget(self.delete_btn)
        btn_layout.addStretch()
        btn_layout.addWidget(self.close_btn)
        layout.addLayout(btn_layout)

        self.select_all_btn.clicked.connect(self.on_select_all)
        self.delete_btn.clicked.connect(self.on_delete)
        self.close_btn.clicked.connect(self.close)
        self.user_tree.itemSelectionChanged.connect(self.on_selection_changed)

        self.setStyleSheet("""
            QDialog { background-color: #ffffff; }
            QTreeWidget {
                background: #ffffff; border: 1px solid #e4e7ed;
                alternate-background-color: #fafbfc; gridline-color: #f2f6fc;
                selection-background-color: #d9eaff; font-size: 13px;
                show-decoration-selected: 0;
            }
            QTreeWidget::item { padding: 0px 4px; color: #222222; outline: 0; }
            QTreeWidget::item:focus { outline: 0; border: 0; }
            QTreeWidget::item:hover { background: #f3f8fe; }
            QTreeWidget::item:selected { background: #cfe4ff; color: #000000; }
            QTreeWidget::item:selected:active { background: #cfe4ff; outline: 0; }
            QTreeWidget::item:selected:!active { background: #cfe4ff; outline: 0; }
            QTreeWidget::indicator {
                width: 16px; height: 16px; border: 1px solid #c0c4cc;
                border-radius: 2px; background: #ffffff;
            }
            QTreeWidget::indicator:hover { border: 1px solid #409EFF; }
            QTreeWidget::indicator:checked {
                background-color: #409EFF; border: 1px solid #409EFF;
                image: url(""" + self.checkmark_svg_path + r""");
            }
            QPushButton {
                background-color: #409EFF; border: 1px solid #409EFF; color: white;
                padding: 6px 14px; border-radius: 0px; font-weight: 500; font-size: 13px;
            }
            QPushButton:hover { background-color: #66b1ff; border: 1px solid #66b1ff; }
            QPushButton:pressed { background-color: #3a8ee6; border: 1px solid #3a8ee6; }
        """)

        self.load_users()
    
    def load_users(self):
        """加载用户列表"""
        self.user_tree.clear()
        users = cfg.get('users', [])
        updated = False
        
        # 检查并更新用户链接为标准化格式
        for user in users:
            original_url = user.get('url', '')
            if original_url and not original_url.startswith('https://www.douyin.com/user/'):
                # 尝试提取sec_user_id并构建标准化URL（纯正则，不阻塞主线程）
                sec_user_id = extract_sec_user_id_from_url(original_url)
                if sec_user_id:
                    normalized_url = f"https://www.douyin.com/user/{sec_user_id}"
                    if original_url != normalized_url:
                        user['url'] = normalized_url
                        updated = True
        
        # 如果有更新，保存配置
        if updated:
            cfg['users'] = users
            from douyin_downloader.utils.config import save_config
            save_config(cfg)
        
        for idx, user in enumerate(users, start=1):
            item = QtWidgets.QTreeWidgetItem(self.user_tree, [' ', str(idx), user.get('username', ''), user.get('url', ''), ''])
            item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
            item.setCheckState(0, Qt.CheckState.Unchecked)
            item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
            item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignCenter))
            item.setData(0, Qt.ItemDataRole.UserRole, user)
            
            # 在操作列添加 "获取" 按钮
            fetch_btn = QtWidgets.QPushButton('获取')
            fm = self.user_tree.fontMetrics()
            btn_width = fm.horizontalAdvance('关闭') + 28
            fetch_btn.setFixedWidth(btn_width)
            fetch_btn.setFixedHeight(28)
            fetch_btn.setStyleSheet("""
                QPushButton {
                    background-color: #409EFF; border: 1px solid #409EFF; color: white;
                    padding: 0px; border-radius: 0px; font-weight: 500; font-size: 13px;
                }
                QPushButton:hover { background-color: #66b1ff; border: 1px solid #66b1ff; }
                QPushButton:pressed { background-color: #3a8ee6; border: 1px solid #3a8ee6; }
            """)
            fetch_btn.clicked.connect(lambda checked, u=user: self.on_fetch_user(u))
            
            btn_container = QtWidgets.QWidget()
            btn_container.setMinimumHeight(50)
            v_layout = QtWidgets.QVBoxLayout(btn_container)
            v_layout.setContentsMargins(0, 0, 0, 0)
            v_layout.setSpacing(0)
            v_layout.addStretch()
            h_widget = QtWidgets.QWidget()
            h_layout = QtWidgets.QHBoxLayout(h_widget)
            h_layout.setContentsMargins(0, 0, 0, 0)
            h_layout.setSpacing(0)
            h_layout.addStretch()
            h_layout.addWidget(fetch_btn)
            h_layout.addStretch()
            v_layout.addWidget(h_widget)
            v_layout.addStretch()
            self.user_tree.setItemWidget(item, 4, btn_container)
            
            # 设置行高
            item.setSizeHint(0, QtCore.QSize(-1, 50))
    
    def on_fetch_user(self, user):
        """点击 "获取" 按钮"""
        try:
            # 获取主窗口
            main_window = self.parent()
            if main_window:
                # 1. 填充主页链接（标准化URL）
                if hasattr(main_window, 'url_edit'):
                    url_edit = getattr(main_window, 'url_edit', None)
                    if url_edit:
                        original_url = user.get('url', '')
                        sec_user_id = extract_sec_user_id_from_url(original_url)
                        if sec_user_id:
                            # 使用标准化URL
                            normalized_url = f"https://www.douyin.com/user/{sec_user_id}"
                            url_edit.setText(normalized_url)
                            
                            # 同时更新配置中的用户链接为标准化链接
                            users = cfg.get('users', [])
                            for u in users:
                                if u.get('url') == original_url:
                                    u['url'] = normalized_url
                                    break
                            cfg['users'] = users
                            from douyin_downloader.utils.config import save_config
                            save_config(cfg)
                        else:
                            # 如果无法提取sec_user_id，则使用原始URL
                            url_edit.setText(original_url)
                # 2. 触发主窗口的 "获取"
                if hasattr(main_window, 'on_fetch'):
                    on_fetch = getattr(main_window, 'on_fetch', None)
                    if on_fetch:
                        on_fetch()
                # 3. 关闭弹窗
                self.close()
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'获取失败: {e}')
    
    def on_delete(self):
        """删除选中的用户"""
        selected_items = []
        for i in range(self.user_tree.topLevelItemCount()):
            item = self.user_tree.topLevelItem(i)
            if item and item.checkState(0) == Qt.CheckState.Checked:
                selected_items.append(item)
        
        if not selected_items:
            QtWidgets.QMessageBox.warning(self, '提示', '请先选择要删除的用户')
            return
        
        msg_box = QtWidgets.QMessageBox(self)
        msg_box.setWindowTitle('确认')
        msg_box.setText(f'确定要删除选中的 {len(selected_items)} 个用户吗？')
        msg_box.setStandardButtons(QtWidgets.QMessageBox.StandardButton.Ok | QtWidgets.QMessageBox.StandardButton.Cancel)
        msg_box.setDefaultButton(QtWidgets.QMessageBox.StandardButton.Cancel)
        button_ok = msg_box.button(QtWidgets.QMessageBox.StandardButton.Ok)
        button_cancel = msg_box.button(QtWidgets.QMessageBox.StandardButton.Cancel)
        if button_ok: button_ok.setText('确认')
        if button_cancel: button_cancel.setText('取消')
        
        if msg_box.exec() != QtWidgets.QMessageBox.StandardButton.Ok:
            return
        
        users = cfg.get('users', [])
        users_to_remove = [item.data(0, Qt.ItemDataRole.UserRole) for item in selected_items]
        
        new_users = [u for u in users if u not in users_to_remove]
    
        cfg['users'] = new_users
        save_config(cfg)

        self.load_users()
    
    def on_select_all(self):
        """全选/反选所有用户"""
        has_unchecked = False
        for i in range(self.user_tree.topLevelItemCount()):
            item = self.user_tree.topLevelItem(i)
            if item and item.checkState(0) == Qt.CheckState.Unchecked:
                has_unchecked = True
                break

        new_state = Qt.CheckState.Checked if has_unchecked else Qt.CheckState.Unchecked
        for i in range(self.user_tree.topLevelItemCount()):
            item = self.user_tree.topLevelItem(i)
            if item:
                item.setCheckState(0, new_state)
    
    def on_selection_changed(self):
        """当选择改变时，同步复选框状态（行选 -> 勾选）"""
        selected_items = self.user_tree.selectedItems()
        # 遍历所有项，根据是否被选中来设置复选框
        for i in range(self.user_tree.topLevelItemCount()):
            item = self.user_tree.topLevelItem(i)
            if item:
                if item in selected_items:
                    item.setCheckState(0, Qt.CheckState.Checked)
                else:
                    item.setCheckState(0, Qt.CheckState.Unchecked)