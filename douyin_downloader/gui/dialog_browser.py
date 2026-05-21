#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
GUI - Browser Configuration Dialog
"""
import sys
import os
try:
    from PyQt6 import QtWidgets
    from PyQt6.QtCore import Qt
except ImportError:
    print("[错误] PyQt6 未安装或无法导入: \n请安装 PyQt6 后重试（pip install PyQt6）。")
    sys.exit(1)

from douyin_downloader.utils.config import load_config, save_config

class BrowserConfigWindow(QtWidgets.QDialog):
    """浏览器配置窗口"""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle('配置浏览器')
        self.setModal(True)
        self.resize(500, 200)
        
        self.config = load_config()

        self.setup_ui()

        self.load_settings()
        
    def setup_ui(self):
        """设置界面"""
        layout = QtWidgets.QVBoxLayout(self)
        layout.setSpacing(15)
        
        info_text = (
            "配置浏览器安装路径\n\n"
            "请输入Chrome或Edge浏览器的程序路径\n\n"
            "例如：\n"
            "【 Chrome: 】 C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\n"
            "\n"
            "【 Edge: 】 C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"
        )
        info_label = QtWidgets.QLabel(info_text)
        info_label.setWordWrap(True)
        info_label.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        layout.addWidget(info_label)
        
        browser_layout = QtWidgets.QHBoxLayout()
        browser_label = QtWidgets.QLabel("浏览器路径:")
        self.browser_path_edit = QtWidgets.QLineEdit()
        self.browser_path_edit.setPlaceholderText("")
        browser_browse_btn = QtWidgets.QPushButton("浏览")
        browser_browse_btn.clicked.connect(self.browse_file)
        
        browser_layout.addWidget(browser_label)
        browser_layout.addWidget(self.browser_path_edit)
        browser_layout.addWidget(browser_browse_btn)
        layout.addLayout(browser_layout)
        
        layout.addStretch()
        
        button_layout = QtWidgets.QHBoxLayout()
        self.test_btn = QtWidgets.QPushButton('测试')
        self.test_btn.clicked.connect(self.on_test)
        self.ok_btn = QtWidgets.QPushButton('确定')
        self.cancel_btn = QtWidgets.QPushButton('取消')
        
        button_layout.addWidget(self.test_btn)
        button_layout.addStretch()
        button_layout.addWidget(self.ok_btn)
        button_layout.addWidget(self.cancel_btn)
        layout.addLayout(button_layout)
        
        self.ok_btn.clicked.connect(self.on_ok)
        self.cancel_btn.clicked.connect(self.reject)
        
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
        QLineEdit {
            padding: 5px;
            border: 1px solid #dcdfe6;
            border-radius: 0px;
        }
        """)
    
    def browse_file(self):
        """浏览文件选择浏览器路径"""
        file_path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self, 
            "选择浏览器", 
            "", 
            "Executable Files (*.exe);;All Files (*)"
        )
        
        if file_path:
            self.browser_path_edit.setText(file_path)
    
    def load_settings(self):
        """加载设置"""
        from douyin_downloader.utils.config import load_config
        latest_config = load_config()
        chrome_path = latest_config.get('chrome_path', '')
        edge_path = latest_config.get('edge_path', '')
        
        # 修复：即使路径存在也优先显示当前输入框中的值（如果有的话）
        current_text = self.browser_path_edit.text().strip()
        if current_text:
            return
        elif chrome_path and os.path.exists(chrome_path):
            self.browser_path_edit.setText(chrome_path)
        elif edge_path and os.path.exists(edge_path):
            self.browser_path_edit.setText(edge_path)
        elif chrome_path:
            self.browser_path_edit.setText(chrome_path)
        elif edge_path:
            self.browser_path_edit.setText(edge_path)

    def showEvent(self, a0):
        """窗口显示事件"""
        # 修复：只在输入框为空时才加载配置
        if not self.browser_path_edit.text().strip():
            self.load_settings()
        super().showEvent(a0)
    
    def save_settings(self):
        """保存设置"""
        browser_path = self.browser_path_edit.text().strip()

        if not browser_path:
            # 如果路径为空，清空两个配置
            self.config['chrome_path'] = ''
            self.config['edge_path'] = ''
        elif 'chrome.exe' in browser_path.lower():
            # Chrome浏览器
            self.config['chrome_path'] = browser_path
            self.config['edge_path'] = ''
        elif 'edge.exe' in browser_path.lower():
            # Edge浏览器
            self.config['edge_path'] = browser_path
            self.config['chrome_path'] = ''
        else:
            # 无法识别的浏览器类型，默认设置为Chrome
            self.config['chrome_path'] = browser_path
            self.config['edge_path'] = ''

        from douyin_downloader.utils.config import save_config
        save_config(self.config)
        
        # 同时更新全局配置对象，确保其他窗口能看到最新配置
        from douyin_downloader.gui import cfg
        cfg['chrome_path'] = self.config['chrome_path']
        cfg['edge_path'] = self.config['edge_path']
    
    def on_ok(self):
        """确定按钮事件"""
        self.save_settings()
        self.accept()
    
    def on_test(self):
        """测试按钮事件"""
        self.save_settings()

        from douyin_downloader.utils.config import load_config
        config = load_config()
        chrome_path = config.get('chrome_path', '').strip()
        edge_path = config.get('edge_path', '').strip()
        
        if not chrome_path and not edge_path:
            QtWidgets.QMessageBox.warning(self, '测试失败', '请先配置浏览器路径')
            return
        
        playwright = None
        browser = None
        context = None
        page = None
        
        try:
            from playwright.sync_api import sync_playwright
            
            playwright = sync_playwright()
            playwright_instance = playwright.start()
            
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

            context = browser.new_context()
            page = context.new_page()
            page.goto("https://www.douyin.com/?recommend=1")
            
            # 如果能成功打开页面，则测试成功
            QtWidgets.QMessageBox.information(self, '测试成功', '浏览器配置正确，可以正常启动')
            
        except ImportError:
            QtWidgets.QMessageBox.warning(
                self, 
                '测试失败', 
                '未安装Playwright库，请运行: pip install playwright\n然后运行: playwright install chromium'
            )
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, '测试失败', f'浏览器启动失败: {str(e)}')
        finally:
            # 确保正确关闭所有资源
            try:
                if page:
                    page.close()
            except Exception:
                pass
                
            try:
                if context:
                    context.close()
            except Exception:
                pass
                
            try:
                if browser:
                    browser.close()
            except Exception:
                pass
                
            try:
                if playwright:
                    playwright.__exit__(None, None, None)
            except Exception:
                pass
