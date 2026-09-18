#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - Cookie Auto Fetch Dialog
"""
import sys
import os
try:
    from PyQt6 import QtWidgets, QtCore
    from PyQt6.QtCore import Qt
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

from douyin_downloader.utils.config import load_config, save_config

class CookieFetchWindow(QtWidgets.QDialog):
    """Cookie自动获取窗口"""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle('Cookie自动获取')
        self.setModal(True)
        self.resize(400, 200)
        
        self.setup_ui()

        # 初始化浏览器相关属性
        self.browser = None
        self.context = None
        self.page = None
        self.playwright = None
        self._poll_timer = None
        
    def setup_ui(self):
        """设置界面"""
        layout = QtWidgets.QVBoxLayout(self)
        layout.setSpacing(15)

        info_text = (
            "点击【开始获取】后，在浏览器中扫码登录账号\n"
            "\n"
            "如有二次验证请完成二次验证\n"
            "\n"
            "登录成功后程序会自动识别并保存 Cookie，并自动关闭浏览器"
        )
        info_label = QtWidgets.QLabel(info_text)
        info_label.setWordWrap(True)
        info_label.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        layout.addWidget(info_label)
        
        layout.addStretch()

        button_layout = QtWidgets.QHBoxLayout()
        self.start_btn = QtWidgets.QPushButton('开始获取')
        self.confirm_btn = QtWidgets.QPushButton('确认')
        self.confirm_btn.setEnabled(False)
        
        button_layout.addWidget(self.start_btn)
        button_layout.addWidget(self.confirm_btn)
        layout.addLayout(button_layout)

        self.start_btn.clicked.connect(self.on_start_fetch)
        self.confirm_btn.clicked.connect(self.on_confirm)

        self.setStyleSheet("""
        QDialog {
            background-color: #ffffff;
        }
        QLabel { 
            color: #303133; 
            font-size: 13px; 
        }
        QPushButton {
            border: 1px solid #dcdfe6; 
            background: #409EFF; 
            color: #ffffff;
            padding: 8px 16px; 
            border-radius: 0px; 
            font-size: 13px;
        }
        QPushButton:hover { 
            background: #66b1ff; 
        }
        QPushButton:pressed {
            border: 1px solid #409EFF; 
            background: #409EFF; 
            color: #ffffff;
        }
        QPushButton:disabled {
            background: #a0cfff; 
            border: 1px solid #a0cfff; 
            color: #f0f0f0;
        }
        """)
    
    def close_browser(self):
        """关闭浏览器并清理资源"""
        try:
            if self._poll_timer is not None:
                try:
                    self._poll_timer.stop()
                except Exception:
                    pass
                self._poll_timer = None
            if self.page:
                self.page = None
            if self.context:
                self.context = None
            if self.browser:
                self.browser.close()
                self.browser = None
            if self.playwright:
                self.playwright.__exit__(None, None, None)
                self.playwright = None
        except Exception:
            pass
        finally:
            self.start_btn.setEnabled(True)
            self.confirm_btn.setEnabled(False)
    
    def validate_cookie(self, cookie_str):
        """验证Cookie是否有效"""
        try:
            # 检查Cookie是否为空
            if not cookie_str or len(cookie_str) < 50:
                return False
            
            # 检查是否包含必需的sessionid字段
            if 'sessionid' not in cookie_str:
                return False
            
            # Cookie有效
            return True
        except Exception:
            return False
    
    def on_start_fetch(self):
        """开始获取Cookie"""
        try:
            # 获取浏览器配置
            config = load_config()
            chrome_path = config.get('chrome_path', '').strip()
            edge_path = config.get('edge_path', '').strip()

            if not chrome_path and not edge_path:
                # 创建消息框并自定义按钮文本
                msg_box = QtWidgets.QMessageBox(self)
                msg_box.setWindowTitle('浏览器未配置')
                msg_box.setText('您尚未配置浏览器路径，是否现在配置浏览器？')
                confirm_button = msg_box.addButton('确认', QtWidgets.QMessageBox.ButtonRole.AcceptRole)
                cancel_button = msg_box.addButton('取消', QtWidgets.QMessageBox.ButtonRole.RejectRole)
                msg_box.setDefaultButton(confirm_button)
                
                # 显示消息框
                result = msg_box.exec()
                
                # 检查用户点击了哪个按钮
                if msg_box.clickedButton() == confirm_button:
                    # 打开浏览器配置窗口
                    from .dialog_browser import BrowserConfigWindow
                    browser_config_window = BrowserConfigWindow(self)
                    browser_config_window.exec()
                    config = load_config()
                    chrome_path = config.get('chrome_path', '').strip()
                    edge_path = config.get('edge_path', '').strip()
                    
                    # 如果用户配置了浏览器路径，启用开始按钮，让用户手动点击开始获取
                    if chrome_path or edge_path:
                        self.start_btn.setEnabled(True)
                        self.confirm_btn.setEnabled(False)
                        # 提示用户已配置浏览器，可以点击开始获取按钮
                        QtWidgets.QMessageBox.information(self, '提示', '浏览器已配置完成，请点击"开始获取"按钮启动浏览器')
                        return
                    # 如果用户仍然没有配置浏览器路径，则返回
                    elif not chrome_path and not edge_path:
                        self.start_btn.setEnabled(True)
                        self.confirm_btn.setEnabled(False)
                        return
                elif msg_box.clickedButton() == cancel_button:
                    # 用户点击了取消，直接返回不执行后续操作
                    return
            
            # 导入Playwright
            from playwright.sync_api import sync_playwright

            self.close_browser()

            self.start_btn.setEnabled(False)
            self.confirm_btn.setEnabled(True)
            
            # 启动浏览器
            self.playwright = sync_playwright()
            playwright_instance = self.playwright.start()
            
            if chrome_path and os.path.exists(chrome_path):
                browser = playwright_instance.chromium.launch(
                    headless=False,
                    executable_path=chrome_path
                )
            elif edge_path and os.path.exists(edge_path):
                browser = playwright_instance.chromium.launch(
                    headless=False,
                    executable_path=edge_path
                )
            else:
                browser = playwright_instance.chromium.launch(headless=False)
            
            self.browser = browser
            self.context = self.browser.new_context()
            self.page = self.context.new_page()
            self.page.goto("https://www.douyin.com/?recommend=1")

            # 启动轮询，自动检测登录成功
            self._start_polling()
            
        except ImportError:
            QtWidgets.QMessageBox.warning(
                self, 
                '错误', 
                '未安装Playwright库，请运行: pip install playwright\n然后运行: playwright install chromium'
            )
            self.start_btn.setEnabled(True)
            self.confirm_btn.setEnabled(False)
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'启动浏览器失败: {str(e)}')
            self.start_btn.setEnabled(True)
            self.confirm_btn.setEnabled(False)
    
    def _start_polling(self):
        """启动定时轮询，检测浏览器中是否已完成登录"""
        self._poll_timer = QtCore.QTimer(self)
        self._poll_timer.timeout.connect(self._on_poll_tick)
        self._poll_timer.start(2000)

    def _on_poll_tick(self):
        """轮询检测登录成功（sessionid 出现即视为登录成功）"""
        if not self.context:
            return
        try:
            cookies = self.context.cookies("https://www.douyin.com")
            has_sessionid = any(c.get('name') == 'sessionid' and c.get('value') for c in cookies)
            if has_sessionid:
                self._poll_timer.stop()
                self._save_cookie(cookies)
        except Exception:
            pass

    def _find_main_window(self):
        """向上查找主窗口，用于刷新 Cookie 状态标签"""
        w = self.parent()
        while w is not None:
            if hasattr(w, '_check_cookie_on_startup'):
                return w
            w = w.parent()
        return None

    def _save_cookie(self, cookies):
        """保存 Cookie 到配置并填回设置窗口，然后关闭对话框"""
        try:
            cookie_str = "; ".join([f"{cookie.get('name', '')}={cookie.get('value', '')}" for cookie in cookies])

            if not self.validate_cookie(cookie_str):
                QtWidgets.QMessageBox.warning(self, '错误', '获取到的Cookie无效，请重新登录获取')
                return

            config = load_config()
            config['cookie'] = cookie_str
            save_config(config)

            # 更新父窗口的Cookie输入框（使用getattr安全访问）
            parent = self.parent()
            if parent:
                settings_cookie = getattr(parent, 'settings_cookie', None)
                if settings_cookie:
                    settings_cookie.setPlainText(cookie_str)

            # 刷新主窗口左下角的 Cookie 状态标签
            main_window = self._find_main_window()
            if main_window:
                main_window._check_cookie_on_startup()

            self.close_browser()

            QtWidgets.QMessageBox.information(self, '成功', 'Cookie已成功获取并保存')

            self.accept()

        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'获取Cookie失败: {str(e)}')

    def on_confirm(self):
        """手动确认并获取Cookie（自动检测未触发时的手动兜底）"""
        if not self.context or not self.browser:
            QtWidgets.QMessageBox.warning(self, '错误', '请先点击"开始获取"按钮，并等待浏览器启动完成')
            return

        try:
            if self._poll_timer is not None:
                try:
                    self._poll_timer.stop()
                except Exception:
                    pass
                self._poll_timer = None
            cookies = self.context.cookies("https://www.douyin.com")
            self._save_cookie(cookies)
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '错误', f'获取Cookie失败: {str(e)}')
    
    def closeEvent(self, a0):
        """窗口关闭事件"""
        self.close_browser()
        super().closeEvent(a0)