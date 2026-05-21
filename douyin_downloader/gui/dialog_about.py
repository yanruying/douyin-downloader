#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - About and Tutorial Dialogs
"""
import sys
try:
    from PyQt6 import QtWidgets
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

class AboutWindow(QtWidgets.QDialog):
    """关于窗口"""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle('关于')
        self.setModal(False)
        self.resize(600, 300)
        
        layout = QtWidgets.QVBoxLayout(self)
        
        self.about_text = QtWidgets.QTextEdit()
        self.about_text.setReadOnly(True)
        about_content = """
抖音主页作品解析下载 V3.8

作者:颜如嘤-YanRuYing

B站主页：https://space.bilibili.com/80435723

更新地址:https://www.52pojie.cn/thread-2064455-1-1.html

开源地址:https://github.com/yanruying/douyin-downloader

博客:https://bbs.hookyun.cn

本项程序完全免费，仅用于学习与研究

禁止将本程序用于任何商业或违法用途
        """
        self.about_text.setPlainText(about_content)
        layout.addWidget(self.about_text)
        
        btn_layout = QtWidgets.QHBoxLayout()
        self.close_btn = QtWidgets.QPushButton('关闭')
        btn_layout.addStretch()
        btn_layout.addWidget(self.close_btn)
        layout.addLayout(btn_layout)
        
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


class TutorialWindow(QtWidgets.QDialog):
    """Cookie 教程窗口"""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle('Cookie 获取教程')
        self.setModal(False)
        self.resize(800, 500)
        
        layout = QtWidgets.QVBoxLayout(self)
        
        self.tutorial_text = QtWidgets.QTextEdit()
        self.tutorial_text.setReadOnly(True)
        tutorial_content = """
【注意事项】
- 打开软件第一次获取作品等待时间过长请停止获取后重新点击获取
- Cookie 具有时效性，如果出现获取失败，请重新获取 Cookie
- 不要将 Cookie 分享给他人，避免账号被盗

【手动获取 Cookie 教程】

1. 打开浏览器（推荐使用 Chrome 或 Edge）

2. 访问抖音网页版：https://www.douyin.com/?recommend=1

3. 登录你的抖音账号

4. 按 F12 打开开发者工具

5. 点击顶部的 "Network" 或 "网络" 标签

6. 按F5刷新页面

7. 找到 ?recommend=1 请求，一般第一个就是 

8. 点击请求，查看请求标头（Request Headers）

9. 找到 "Cookie" 字段，复制全部

10. 将复制的 Cookie 粘贴到下面的 Cookie 输入框中

11. 点击保存按钮
        """
        self.tutorial_text.setPlainText(tutorial_content)
        layout.addWidget(self.tutorial_text)
        
        btn_layout = QtWidgets.QHBoxLayout()
        self.close_btn = QtWidgets.QPushButton('关闭')
        btn_layout.addStretch()
        btn_layout.addWidget(self.close_btn)
        layout.addLayout(btn_layout)
        
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