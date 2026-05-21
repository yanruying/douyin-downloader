#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
抖音API - 用户信息和作品获取辅助函数
"""
import re
import time
import requests
from douyin_downloader.constants import REQUEST_TIMEOUT, PAGE_COUNT_PER_REQUEST

def extract_sec_user_id_from_url(user_home_url):
    """从主页URL中提取sec_user_id"""
    patterns = [
        r'/user/([A-Za-z0-9_.\-]+)',      # 匹配 /user/SEC_ID
        r'sec_user_id=([A-Za-z0-9_.\-]+)', # 匹配 sec_user_id=SEC_ID
        r'(MS4wLjAB[A-Za-z0-9_-]+)'        # 匹配 MS4wLjAB... 格式的ID
    ]
    
    for pattern in patterns:
        m = re.search(pattern, user_home_url)
        if m:
            return m.group(1)
    
    return None


def resolve_short_url_and_extract(url, timeout=10, session=None):
    """解析短链接/分享链接并提取sec_user_id"""
    try:
        # allow_redirects=True 会自动处理 301/302 跳转
        if session is not None:
            r = session.get(url, allow_redirects=True, timeout=timeout)
        else:
            r = requests.get(url, allow_redirects=True, timeout=timeout)
        final_url = r.url or url
    except Exception:
        final_url = url
    
    # 优先从跳转后的URL提取
    sec = extract_sec_user_id_from_url(final_url)
    if not sec and final_url != url:
    # 如果跳转后没取到，尝试从原始URL取（防止跳转到登录页等）
        sec = extract_sec_user_id_from_url(url)
    
    return sec


def get_user_profile_info(session, sec_user_id):
    """获取用户资料信息，返回 (data_dict, error_msg) — 其一为 None"""
    api_url = (
        f"https://www.douyin.com/aweme/v1/web/user/profile/other/"
        f"?device_platform=webapp&aid=6383&channel=channel_pc_web"
        f"&sec_user_id={sec_user_id}&from_user_page=1"
    )

    try:
        r = session.get(api_url, timeout=REQUEST_TIMEOUT)
        data = r.json()

        if data.get('status_code') == 0 and 'user' in data:
            u = data['user']
            return {
                'nickname': u.get('nickname') or '',
                'unique_id': u.get('unique_id') or '',
                'aweme_count': u.get('aweme_count', None),
            }, None

        return None, f"API status_code={data.get('status_code')}, message={data.get('status_msg', '')}"
    except requests.Timeout:
        return None, "请求超时"
    except requests.RequestException as e:
        return None, f"请求失败: {e}"
    except Exception as e:
        return None, f"未知错误: {e}"


def build_aweme_post_url(sec_user_id, max_cursor=0, count=None, is_first_page=False):
    """构建 aweme/post API 请求参数，返回 (params_dict, base_url)"""
    if count is None:
        count = PAGE_COUNT_PER_REQUEST
    params = {
        'device_platform': 'webapp', 'aid': '6383', 'channel': 'channel_pc_web',
        'sec_user_id': sec_user_id, 'max_cursor': max_cursor, 'count': count,
        'locate_query': 'false', 'show_live_replay_strategy': '1',
        'need_time_list': '1' if is_first_page else '0',
        'publish_video_strategy_type': '2', 'from_user_page': '1',
        'update_version_code': '170400',
    }
    base_url = 'https://www.douyin.com/aweme/v1/web/aweme/post/'
    return params, base_url


def build_aweme_favorite_url(sec_user_id, max_cursor=0, count=None):
    """构建 aweme/favorite API 请求参数，返回 (params_dict, base_url)"""
    if count is None:
        count = PAGE_COUNT_PER_REQUEST
    params = {
        'device_platform': 'webapp', 'aid': '6383', 'channel': 'channel_pc_web',
        'sec_user_id': sec_user_id, 'max_cursor': max_cursor, 'count': count,
    }
    base_url = 'https://www.douyin.com/aweme/v1/web/aweme/favorite/'
    return params, base_url


def api_request_with_retry(session, url, max_retries=3, base_delay=1, timeout=None):
    """带指数退避重试的 API 请求"""
    if timeout is None:
        timeout = REQUEST_TIMEOUT
    for attempt in range(max_retries + 1):
        try:
            r = session.get(url, timeout=timeout)
            r.raise_for_status()
            return r
        except requests.Timeout:
            if attempt < max_retries:
                time.sleep(min(2 ** attempt * base_delay, 30))
                continue
            raise
        except requests.HTTPError as e:
            if e.response is not None and e.response.status_code in (429, 503) and attempt < max_retries:
                time.sleep(min(2 ** attempt * base_delay, 60))
                continue
            raise