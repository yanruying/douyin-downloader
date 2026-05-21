#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
作品解析与任务构建区块 （解析 Aweme → 视频/图片任务）
"""
import hashlib
from datetime import datetime
from douyin_downloader.constants import MAX_DESC_LENGTH
from douyin_downloader.utils.file_utils import get_extension_from_url

def extract_media_links_from_aweme(aweme):
    """
    从单个 aweme JSON 对象中提取媒体链接。
    
    互斥提取逻辑：
      - 如果 image 项包含 'video' 字段 -> 视为实况图（只提取实况图视频 .mp4）
      - 否则 -> 视为普通图片（提取最高分辨率 url_list[-1] .jpg/.png）
      
    返回： desc, videos[], images[], live_images[], date_str, mix_name
    """
    videos, images, live_images = [], [], []
    aweme_id = aweme.get('aweme_id') or ''
    desc = aweme.get('desc', '') or aweme_id or 'no_desc'
    
    # 提取合集名称
    mix_name = None
    mix_info = aweme.get('mix_info', {})
    if isinstance(mix_info, dict):
        mix_name = mix_info.get('mix_name') or mix_info.get('mix_name_str') or None
    if not mix_name:
        mix_name = aweme.get('mix_name') or aweme.get('mix_name_str') or None

    # 转换时间戳 -> YYYY-MM-DD
    ts = aweme.get('create_time')
    date_str = ''
    if ts:
        try:
            date_str = datetime.fromtimestamp(ts).strftime("%Y-%m-%d")
        except Exception:
            pass # 时间戳转换失败

    # 自动截断过长描述
    if len(desc) > MAX_DESC_LENGTH:
        desc = desc[:MAX_DESC_LENGTH] + "......"

    # 1. 提取普通视频作品 (aweme.video)
    video_info = aweme.get('video', {})
    bit_rate_list = video_info.get('bit_rate', [])
    if bit_rate_list:
        try:
            # 选择最高码率
            best = max(bit_rate_list, key=lambda x: x.get('bit_rate', 0))
            url_list = best.get('play_addr', {}).get('url_list', [])
            if url_list:
                videos.append(url_list[0]) # 通常第一个链接最稳定
        except Exception:
            pass # 码率列表格式异常

    # 2. 提取图集作品 (aweme.images)
    if 'images' in aweme and isinstance(aweme['images'], list):
        for img in aweme['images']:
            if not isinstance(img, dict):
                continue

            # 2a. 检查是否为实况图（包含 video 字段）
            vinfo = img.get('video')
            if vinfo and isinstance(vinfo, dict) and 'bit_rate' in vinfo:
                try:
                    rates = vinfo.get('bit_rate') or []
                    if rates:
                        best = max(rates, key=lambda x: x.get('bit_rate', 0))
                        vurl_list = best.get('play_addr', {}).get('url_list', [])
                        if vurl_list:
                            live_images.append(vurl_list[0])
                    # 互斥：提取了实况图，就跳过该项的普通图片提取
                    continue
                except Exception:
                    pass # 码率列表格式异常

            # 2b. 按普通图片处理
            url_list = img.get('url_list', [])
            if url_list and isinstance(url_list, list) and url_list:
                # 默认最后一个是最高分辨率
                images.append(url_list[-1])

    return desc, videos, images, live_images, date_str, mix_name


def parse_all_awemes_to_tasks(all_awemes):
    """将所有aweme解析为下载任务列表"""
    video_tasks, image_tasks = [], []
    album_count = 0
    image_count = 0
    live_count = 0

    for aweme in all_awemes:
        desc, videos, images, live_images, date_str, mix_name = extract_media_links_from_aweme(aweme)

        # 视频任务
        for vurl in videos:
            ext = get_extension_from_url(vurl, '.mp4')
            task = {
                'url': vurl,
                'desc': desc,
                'ext': ext,
                'date': date_str,
                'mix_name': mix_name,
                'aweme': aweme,
                'url_hash': hashlib.md5(vurl.encode('utf-8')).hexdigest()[:8],
            }
            video_tasks.append(task)

        # 如果这个 aweme 有普通图片或实况图，则视为一个图集作品
        if images or live_images:
            album_count += 1

        # 普通图片（按张）
        for idx, iurl in enumerate(images, start=1):
            ext = get_extension_from_url(iurl, '.jpg')
            image_tasks.append({
                'url': iurl, 'desc': f"{desc}_p{idx}", 'ext': ext,
                'date': date_str, 'mix_name': mix_name,
                'url_hash': hashlib.md5(iurl.encode('utf-8')).hexdigest()[:8],
            })
        image_count += len(images)

        # 实况图（按张）
        for idx, lvurl in enumerate(live_images, start=1):
            ext = get_extension_from_url(lvurl, '.mp4')
            image_tasks.append({
                'url': lvurl, 'desc': f"{desc}_live{idx}", 'ext': ext,
                'date': date_str, 'mix_name': mix_name,
                'url_hash': hashlib.md5(lvurl.encode('utf-8')).hexdigest()[:8],
            })
        live_count += len(live_images)

    return video_tasks, image_tasks, album_count, image_count, live_count