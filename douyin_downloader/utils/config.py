#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
配置管理 - INI配置文件读写
"""
import os
import json
import tempfile
import configparser
from douyin_downloader.constants import CONFIG_FILE, DEFAULT_THREAD_COUNT


def _safe_get(cp, section, key, getter='get', default=None, **kwargs):
    """安全读取配置值，任何异常时返回默认值"""
    try:
        if section not in cp:
            return default
        method = getattr(cp[section], getter)
        return method(key, **kwargs)
    except Exception:
        return default


def load_config():
    """加载应用配置"""
    cfg = {}

    # 1. 尝试从旧版 config.txt (json) 迁移
    old_json = 'config.txt'
    if os.path.exists(old_json):
        try:
            with open(old_json, 'r', encoding='utf-8') as f:
                j = json.load(f) or {}
            if isinstance(j, dict):
                cfg.update(j)
        except Exception:
            pass

    # 2. 从 config.ini 加载/覆盖
    if os.path.exists(CONFIG_FILE):
        try:
            cp = configparser.ConfigParser(interpolation=None)
            cp.read(CONFIG_FILE, encoding='utf-8')

            if 'main' in cp:
                cfg['path'] = _safe_get(cp, 'main', 'path', default='')
                cfg['cookie'] = _safe_get(cp, 'main', 'cookie', default='')
                cfg['chrome_path'] = _safe_get(cp, 'main', 'chrome_path', default='')
                cfg['edge_path'] = _safe_get(cp, 'main', 'edge_path', default='')
                cfg['use_mix_folder'] = _safe_get(cp, 'main', 'use_mix_folder', 'getboolean', True)
                cfg['include_date_in_filename'] = _safe_get(cp, 'main', 'include_date_in_filename', 'getboolean', True)
                cfg['auto_select_after_fetch'] = _safe_get(cp, 'main', 'auto_select_after_fetch', 'getboolean', True)
                cfg['add_title_when_export_urls'] = _safe_get(cp, 'main', 'add_title_when_export_urls', 'getboolean', False)
                cfg['threads'] = _safe_get(cp, 'main', 'threads', 'getint', DEFAULT_THREAD_COUNT)
                cfg['icon_choice'] = _safe_get(cp, 'main', 'icon_choice', default='default')

            # 加载用户列表
            cfg['users'] = []
            if 'users' in cp:
                for key in cp['users']:
                    if key.startswith('user'):
                        try:
                            value = cp['users'][key]
                            parts = value.split(',', 1)
                            if len(parts) == 2:
                                cfg['users'].append({'username': parts[0].strip(), 'url': parts[1].strip()})
                        except Exception:
                            pass
        except Exception:
            pass

    # 确保关键默认值存在
    cfg.setdefault('path', '')
    cfg.setdefault('cookie', '')
    cfg.setdefault('chrome_path', '')
    cfg.setdefault('edge_path', '')
    cfg.setdefault('use_mix_folder', True)
    cfg.setdefault('include_date_in_filename', True)
    cfg.setdefault('auto_select_after_fetch', True)
    cfg.setdefault('add_title_when_export_urls', False)
    cfg.setdefault('threads', DEFAULT_THREAD_COUNT)
    cfg.setdefault('icon_choice', 'default')
    cfg.setdefault('users', [])

    return cfg


def save_config(cfg):
    """保存配置到INI文件（原子写入）"""
    try:
        cp = configparser.ConfigParser(interpolation=None)

        # [main] section
        cp['main'] = {
            'path': cfg.get('path', ''),
            'use_mix_folder': str(bool(cfg.get('use_mix_folder', True))),
            'include_date_in_filename': str(bool(cfg.get('include_date_in_filename', True))),
            'auto_select_after_fetch': str(bool(cfg.get('auto_select_after_fetch', True))),
            'add_title_when_export_urls': str(bool(cfg.get('add_title_when_export_urls', False))),
            'threads': str(int(cfg.get('threads', DEFAULT_THREAD_COUNT))),
            'icon_choice': cfg.get('icon_choice', 'default'),
            'chrome_path': cfg.get('chrome_path', ''),
            'edge_path': cfg.get('edge_path', ''),
            'cookie': cfg.get('cookie', ''),
        }

        # [users] section
        if 'users' in cfg and cfg['users']:
            cp['users'] = {}
            for idx, user in enumerate(cfg['users'], start=1):
                cp['users'][f'user{idx}'] = f"{user.get('username', '')},{user.get('url', '')}"

        # 原子写入：先写临时文件，再替换
        tmp_fd, tmp_path = tempfile.mkstemp(
            dir=os.path.dirname(CONFIG_FILE) or '.',
            prefix='.config_', suffix='.tmp'
        )
        try:
            with os.fdopen(tmp_fd, 'w', encoding='utf-8') as f:
                cp.write(f)
            os.replace(tmp_path, CONFIG_FILE)
        except Exception:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)
            raise
    except Exception as e:
        print(f"[警告] 保存 {CONFIG_FILE} 失败: {e}")
