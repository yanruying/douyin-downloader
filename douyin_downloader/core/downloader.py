#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
下载引擎 - 单文件下载（支持断点续传）
"""
import os
from functools import lru_cache

import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

from douyin_downloader.constants import USER_AGENT, DOWNLOAD_CHUNK_SIZE
from douyin_downloader.utils.file_utils import (
    safe_mkdir, generate_unique_filename, sanitize_filename
)


@lru_cache(maxsize=1)
def _get_default_session():
    """缓存的默认 Session，避免重复创建连接池"""
    s = requests.Session()
    s.headers.update({
        'User-Agent': USER_AGENT,
        'Referer': 'https://www.douyin.com/',
    })
    retry_strategy = Retry(
        total=3,
        backoff_factor=1,
        status_forcelist=[429, 500, 502, 503, 504],
    )
    adapter = HTTPAdapter(
        pool_connections=10,
        pool_maxsize=20,
        max_retries=retry_strategy,
    )
    s.mount("http://", adapter)
    s.mount("https://", adapter)
    return s


def download_single_file(task, base_folder, is_image=False, worker=None, session=None):
    """
    下载单个文件（视频或图片/实况图），支持断点续传。
    由 `Worker.download_tasks` 在线程池中调用。
    """
    url = task['url']
    desc = task['desc']
    ext = task['ext']
    mix_name = task.get('mix_name') or None

    include_date = task.get('include_date_in_filename', True)
    date_str = task.get('date', '')

    base_filename = desc
    if include_date and date_str:
        base_filename = f"{date_str}_{desc}"

    # 1. 确定目标文件夹
    folder = base_folder
    if mix_name:
        mix_clean = sanitize_filename(mix_name, max_length=100)
        folder = os.path.join(folder, mix_clean)

    if is_image:
        folder = os.path.join(folder, 'images')

    safe_mkdir(folder)

    # 2. 生成唯一文件名
    path = generate_unique_filename(base_filename, ext, folder, url, task.get('url_hash'))
    tmp_path = path + '.tmp'

    # 3. 获取 session
    s = session or _get_default_session()
    headers = {}

    # 4. 检查是否有未完成的下载（断点续传）
    existing_size = 0
    if os.path.exists(tmp_path):
        existing_size = os.path.getsize(tmp_path)
        if existing_size > 0:
            headers['Range'] = f'bytes={existing_size}-'

    try:
        with s.get(url, headers=headers, stream=True, timeout=30) as r:
            if r.status_code == 416:  # Range Not Satisfiable — 文件已完整
                os.replace(tmp_path, path)
                return os.path.relpath(path, base_folder)

            if r.status_code not in (200, 206):
                r.raise_for_status()

            # 206 = 服务器支持断点续传，200 = 从头开始
            mode = 'ab' if r.status_code == 206 else 'wb'
            if mode == 'wb' and existing_size > 0:
                existing_size = 0  # 服务器不支持续传，重置

            with open(tmp_path, mode) as f:
                for chunk in r.iter_content(DOWNLOAD_CHUNK_SIZE):
                    if chunk:
                        f.write(chunk)
                        if worker and worker.should_stop_download():
                            raise SystemExit("下载被用户终止")

        # 下载完成，原子替换
        os.replace(tmp_path, path)
        return os.path.relpath(path, base_folder)

    except SystemExit:
        # 用户取消 —— 保留 .tmp 以便下次续传
        return None
    except Exception:
        # 其他错误 —— 保留 .tmp 以便下次续传
        return None
