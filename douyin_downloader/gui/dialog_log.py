#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - Log Dialog
"""
import sys
try:
    from PyQt6 import QtWidgets, QtGui
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

class LogWindow(QtWidgets.QDialog):
    """日志窗口"""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle('运行日志')
        self.setModal(False)
        self.resize(800, 500)
        
        layout = QtWidgets.QVBoxLayout(self)
        
        self.log_text = QtWidgets.QTextEdit()
        self.log_text.setReadOnly(True)
        layout.addWidget(self.log_text)
        
        btn_layout = QtWidgets.QHBoxLayout()
        self.export_log_btn = QtWidgets.QPushButton('导出日志')
        self.clear_log_btn = QtWidgets.QPushButton('清空日志')
        self.close_btn = QtWidgets.QPushButton('关闭')
        btn_layout.addWidget(self.export_log_btn)
        btn_layout.addStretch()
        btn_layout.addWidget(self.clear_log_btn)
        btn_layout.addWidget(self.close_btn)
        layout.addLayout(btn_layout)
        
        self.export_log_btn.clicked.connect(self.export_log)
        self.clear_log_btn.clicked.connect(self.clear_log)
        self.close_btn.clicked.connect(self.close)
        
        self.setStyleSheet("""
            QDialog { background-color: #ffffff; }
            QTextEdit {
                border: 1px solid #dcdfe6; background: #ffffff; color: #303133;
                padding: 6px; border-radius: 0px; font-size: 15px;
                font-family: Consolas, 'Courier New', monospace;
            }
            QPushButton {
                background-color: #409EFF; border: 1px solid #409EFF; color: white;
                padding: 6px 14px; border-radius: 0px; font-weight: 500; font-size: 13px;
            }
            QPushButton:hover { background-color: #66b1ff; border: 1px solid #66b1ff; }
            QPushButton:pressed { background-color: #3a8ee6; border: 1px solid #3a8ee6; }
        """)

    def append_log(self, text):
        self.log_text.append(text)
        self.log_text.moveCursor(QtGui.QTextCursor.MoveOperation.End)
    
    def clear_log(self):
        self.log_text.clear()
    
    def export_log(self):
        """导出日志到文件"""
        log_content = self.log_text.toPlainText()
        if not log_content:
            QtWidgets.QMessageBox.information(self, '提示', '日志为空，无需导出')
            return
        
        try:
            from . import cfg
            base_path = cfg.get('path', '')
        except:
            import os
            base_path = os.getcwd()
        
        import os
        log_dir = os.path.join(base_path, 'log')
        try:
            if not os.path.exists(log_dir):
                os.makedirs(log_dir)
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'创建日志目录失败: {e}')
            return
        
        from datetime import datetime
        timestamp = datetime.now().strftime('%Y-%m-%d-%H.%M.%S')
        filename = f'{timestamp}.txt'
        filepath = os.path.join(log_dir, filename)
        
        try:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(log_content)
            QtWidgets.QMessageBox.information(self, '导出成功', f'日志已导出到:\n{filepath}')
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '导出失败', f'导出日志失败: {e}')