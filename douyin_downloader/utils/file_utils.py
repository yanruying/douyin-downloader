#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
工具函数 - 文件名和路径处理
"""
import os
import re
import time
import hashlib
from datetime import datetime
from urllib.parse import unquote, urlparse

# 创建目录缓存，避免重复检查
_created_dirs = set()

# 预编译正则，避免每次调用 sanitize_filename 时重新编译
_ILLEGAL_CHARS_RE = re.compile(r'[\\/*?:"<>|#]')
_WHITESPACE_RE = re.compile(r'\s+')

def sanitize_filename(filename, max_length=100):
    """清理文件名，移除非法字符"""
    if not filename:
        filename = "unknown"
    filename = unquote(str(filename))
    filename = _ILLEGAL_CHARS_RE.sub("_", filename)
    filename = _WHITESPACE_RE.sub(' ', filename).strip()
    if len(filename) > max_length:
        prefix = filename[:max_length // 2 - 2]
        suffix = filename[-(max_length // 2 - 1):]
        filename = f"{prefix}...{suffix}"
    return filename


def safe_mkdir(path):
    """安全创建目录（支持多级目录）"""
    global _created_dirs
    if path in _created_dirs:
        return True

    try:
        os.makedirs(path, exist_ok=True)
        _created_dirs.add(path)
        return True
    except Exception as e:
        print(f"[错误] 创建目录失败: {path} -> {e}")
        return False


def clear_directory_cache():
    """清空目录创建缓存，切换用户时调用"""
    global _created_dirs
    _created_dirs.clear()


def get_extension_from_url(url, default_ext='.mp4'):
    """从URL提取文件扩展名"""
    try:
        parsed = urlparse(url)
        root, ext = os.path.splitext(parsed.path)
        # 确保扩展名有效
        if ext and 1 < len(ext) <= 6:
            return ext
    except Exception:
        pass
    return default_ext


def generate_unique_filename(base, ext, folder, url, url_hash=None):
    """
    生成唯一的文件路径，避免覆盖。
    """
    base_clean = sanitize_filename(base, max_length=150)
    filename = base_clean + ext
    path = os.path.join(folder, filename)

    if len(path) > 240 or os.path.exists(path):
        ts = datetime.now().strftime('%Y%m%d%H%M%S')
        h = url_hash or hashlib.md5(url.encode('utf-8')).hexdigest()[:8]
        filename = f"{base_clean[:80]}_{ts}_{h}{ext}"
        path = os.path.join(folder, filename)

    counter = 1
    original_path_prefix = path[:-len(ext)]
    while os.path.exists(path):
        filename = f"{original_path_prefix}_{counter}{ext}"
        path = os.path.join(folder, filename)
        counter += 1
        if counter > 200:
            h = url_hash or hashlib.md5(url.encode('utf-8')).hexdigest()[:8]
            filename = f"file_{int(time.time())}_{h}{ext}"
            path = os.path.join(folder, filename)
            break

    return path


def build_expected_filename(desc, ext, is_image, mix_name=None, date_str='', include_date=True):
    """
    构建预期的文件相对路径（用于去重检查）。
    此函数模拟 `download_single_file` 中的文件夹和文件名生成逻辑（但不处理hash或重名）。
    """
    # 根据配置动态构建基础文件名
    base_filename = desc
    if include_date and date_str:
        base_filename = f"{date_str}_{desc}"
    
    filename = sanitize_filename(base_filename, 150) + ext
    folder = ''
    
    if mix_name:
        mix_clean = sanitize_filename(mix_name, max_length=100)
        if is_image:
            folder = os.path.join(mix_clean, 'images')
        else:
            folder = mix_clean
    else:
        if is_image:
            folder = 'images'
        else:
            folder = ''
    
    return os.path.join(folder, filename) if folder else filename