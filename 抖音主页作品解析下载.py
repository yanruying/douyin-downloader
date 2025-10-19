# ============================================================
#  抖音主页作品解析下载 V3.1
#  Copyright (c) 2025 颜如嘤 (YanRuYing)
#  All rights reserved.
#
#  Author: 颜如嘤 (YanRuYing)
#  Website: https://bbs.hookyun.cn
#  License: MIT License
#
#  Description:
#  本项目用于学习与研究，自动解析抖音主页内容并下载作品。
#  禁止将本程序用于任何商业或违法用途。
# ============================================================

# --------------------------
# 1. 导入模块 (Imports)
# --------------------------
import os
import re
import sys
import time
import json
import configparser
import hashlib
import requests
import tempfile
from urllib.parse import unquote, urlparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from tqdm import tqdm
from datetime import datetime
import threading

# =============================================
# 1. 文本配置与提示信息区块 （UI Text / 提示语常量）
# =============================================
TEXT_APP_NAME = "抖音主页作品批量下载 V3.1 - YanRuYing"
TEXT_DESCRIPTION = "说明：自动获取指定用户全部作品并下载视频、图片和实况图片"

TEXT_PROMPT_URL = "[输入] 请输入抖音主页链接："
TEXT_PROMPT_COOKIE = "[输入] 请输入 Cookie："
TEXT_PROMPT_PATH = "[输入] 下载路径（默认当前目录）："
TEXT_PROMPT_THREAD = "[输入] 下载线程（默认 8）："
TEXT_PROMPT_RETRY_DECISION = "[提示] 仍有失败任务，是否继续重试？(Y继续 / N退出): "
TEXT_PROMPT_RESTART = "[提示] 主页链接或 Cookie 错误，按回车重新加载脚本。"

TEXT_INFO_COOKIE_LOCAL = "[信息] 检测到本地 Cookie"
TEXT_INFO_COOKIE_OK = "[信息] 正在验证Cookie"
TEXT_INFO_USER = "[信息] 抖音用户：{nickname}"
MAX_DESC_LENGTH = 60  # 描述最大长度，超过会自动截断

TEXT_INFO_WORKS_TOTAL = "[信息] 作品数量：共 {total} 条"
TEXT_INFO_WORKS_VIDEO = "[信息] 视频作品 {count} 条"
TEXT_INFO_WORKS_ALBUM = "[信息] 图集作品 {count} 条：共包含 {images} 张图片 | {live_images} 张实况图"

TEXT_INFO_FETCH = "[进程] 正在获取作品数据..."
TEXT_INFO_FETCH_PAGE = "[进程] 收到第 {page} 页，作品数: {total}【+ {count}】"
TEXT_INFO_FETCH_DONE = "[完成] 获取完成"

TEXT_INFO_DOWNLOAD_START = "[下载] 开始下载 (视频 {vcount}, 图片+实况 {icount}, 线程数 {threads})"
TEXT_INFO_BASE_DIR = "[目录] 下载目录: {path}"
TEXT_INFO_RETRY = "[重试 {retry}/{max_retry}] 正在重试失败任务 (视频 {vfail}，图片+实况图 {ifail}) ..."
TEXT_INFO_SUMMARY = "下载统计汇总"
TEXT_INFO_DONE = "任务完成"

TEXT_WARN_COOKIE_INVALID = "[警告] 本地 Cookie 失效，请重新输入。"
TEXT_WARN_SAVE_FAIL = "[警告] 保存 Cookie 失败: {error}"
TEXT_WARN_PROFILE_FAIL = "[错误] 获取用户信息异常: Cookie 错误，请重新获取"
TEXT_WARN_EMPTY_RESULT = "[错误] 未获取到任何作品。"
TEXT_WARN_FETCH_FAIL = "[错误] 获取第一页失败: {error}"
TEXT_WARN_PAGE_FAIL = "[警告] 第 {page} 页请求异常: {error}"

TEXT_RESULT_SUCCESS = "成功: {success}  失败: {fail}"
TEXT_RESULT_PATH = "保存目录: {path}"
TEXT_LINE = "=" * 60

ICON_BYTES = b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x10\x00\x00\x00\x10\x08\x06\x00\x00\x00\x1f\xf3\xffa\x00\x00\x00\x01sRGB\x00\xae\xce\x1c\xe9\x00\x00\x01\x15IDAT8Ocd\x80\x80\xffP\x9aT\x8a\x91\x91\x02\xcd`\xcb\x08\x1bp\xfd\x1c\xc4U\x9aFX]G\xd0\x80\xccg\x0f\x19\xa6I\xca1\x88\xb2p0\xbc\xf9\xfb\x13\xc3\x10\xa2\r\x00;\x17\xeccT@\x92\x01 \xadO5|\x19dnn\x81\x9b\x82\xdd\x80\x15\x8b\x18\x18\x82\xfc\x18\x18\xd8\x04\x18`^8h\x1a\xc8`\x7fz=\xc3\x7f\xdd0\x06\xa6+\xab\xf1\x18\xf0\xeb\x03\xc3\x7fV~\xb0\x02\x056n\x06\xaf\x87\xd7\xc1a\x80\xcd\xf9\x98\xb1\xf0\xf3=\xc3\x7f6\x01\x86\xffm\xf3\x18\xfe/\xdf\xc1\xa0ts+\x89\x06@m\x7f\xac\xe1\xc3 ws+\xd8\x150/\x10\xe5\x02\x96\xefo\x19~s\x081\x94\x89j2t\xbf\xb9\x016`\xe7\x97\x8f\x0cn\xdc|Dz\x81\x87\x87\xe1\xff\xe7\xcf`\x8d\xf2\xbf?30\xff\xfd\xcbp\x8fC\x00#\xe0\x90#\x12#\x16\xc4\xc5%\x18^\xbcx\x0eW\xf3\xdf0\x92A\xe4\xf2:\x86w\x7f\x7f\x91\x96\x12\x8d8\x05\x19\xfe\xfd\xff\xcfp\xe1\xc7\x07\xbc\x19\x8c`B"\x94=ai\x93\xec\xec\x0c\x00gfj\x03\xfb\x1e\xc6.\x00\x00\x00\x00IEND\xaeB`\x82'

# =============================================
# 2. 常量配置区块 （Constants / 全局配置）
# =============================================
USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.6261.95 Safari/537.36'
COOKIE_FILE = 'douyin_cookie.txt'
CONFIG_FILE = 'config.ini'
DEFAULT_THREAD_COUNT = 8
REQUEST_TIMEOUT = 12
PAGE_COUNT_PER_REQUEST = 50
DELAY_BETWEEN_PAGES = 0.1
MAX_AUTOMATIC_RETRIES = 3

# =============================================
# 3. 工具函数区块 （Utilities / 文件/字符串/路径处理）
# =============================================
def sanitize_filename(filename, max_length=100):
    if not filename:
        filename = "unknown"
    filename = unquote(str(filename))
    filename = re.sub(r'[\\/*?:"<>|#]', "_", filename)
    filename = re.sub(r'\s+', ' ', filename).strip()
    if len(filename) > max_length:
        prefix = filename[:max_length // 2 - 2]
        suffix = filename[-(max_length // 2 - 1):]
        filename = f"{prefix}...{suffix}"
    return filename


def safe_mkdir(path):
    try:
        os.makedirs(path, exist_ok=True)
        return True
    except Exception as e:
        print(f"[错误] 创建目录失败: {path} -> {e}")
        return False


def get_extension_from_url(url, default_ext='.mp4'):
    parsed = urlparse(url)
    root, ext = os.path.splitext(parsed.path)
    if ext and len(ext) <= 6:
        return ext
    return default_ext


def generate_unique_filename(base, ext, folder, url):
    base_clean = sanitize_filename(base, max_length=150)
    filename = base_clean + ext
    path = os.path.join(folder, filename)
    if len(path) > 240 or os.path.exists(path):
        ts = datetime.now().strftime('%Y%m%d%H%M%S')
        h = hashlib.md5(url.encode('utf-8')).hexdigest()[:8]
        filename = f"{base_clean[:80]}_{ts}_{h}{ext}"
        path = os.path.join(folder, filename)
    counter = 1
    while os.path.exists(path):
        name_only, _ = os.path.splitext(filename)
        filename = f"{name_only}_{counter}{ext}"
        path = os.path.join(folder, filename)
        counter += 1
        if counter > 200:
            filename = f"file_{int(time.time())}_{h}{ext}"
            path = os.path.join(folder, filename)
            break
    return path


def build_expected_filename(desc, ext, is_image):
    folder = 'images' if is_image else ''
    filename = sanitize_filename(desc, 150) + ext
    return os.path.join(folder, filename) if folder else filename


# =============================================
# 4. 配置管理区块 （Config: load / save JSON）
# =============================================
def load_config():
    cfg = {}
    # 如果存在旧的 JSON 配置 (config.txt)，优先尝试迁移
    old_json = 'config.txt'
    if os.path.exists(old_json):
        try:
            with open(old_json, 'r', encoding='utf-8') as f:
                j = json.load(f) or {}
            if isinstance(j, dict):
                cfg.update(j)
        except Exception:
            pass

    # 使用 INI 配置优先
    if os.path.exists(CONFIG_FILE):
        try:
            cp = configparser.ConfigParser(interpolation=None)
            cp.read(CONFIG_FILE, encoding='utf-8')
            if 'main' in cp:
                cfg['path'] = cp['main'].get('path', cfg.get('path', ''))
                cfg['cookie'] = cp['main'].get('cookie', cfg.get('cookie', ''))
        except Exception:
            pass

    # 兼容单独 cookie 文件
    if not cfg.get('cookie') and os.path.exists(COOKIE_FILE):
        try:
            with open(COOKIE_FILE, 'r', encoding='utf-8') as f:
                c = f.read().strip()
            if c:
                cfg['cookie'] = c
        except Exception:
            pass

    return cfg


def save_config(cfg):
    try:
        cp = configparser.ConfigParser(interpolation=None)
        cp['main'] = {'path': cfg.get('path', ''), 'cookie': cfg.get('cookie', '')}
        with open(CONFIG_FILE, 'w', encoding='utf-8') as f:
            cp.write(f)
    except Exception as e:
        print(f"[警告] 保存 {CONFIG_FILE} 失败: {e}")


# =============================================
# 5. Cookie 处理区块 （输入/验证/保存）
# =============================================
def test_cookie(cookie, user_home_url):
    headers = {'User-Agent': USER_AGENT, 'Cookie': cookie, 'Referer': user_home_url}
    try:
        r = requests.get(user_home_url, headers=headers, timeout=10)
        return r.status_code == 200 and '未登录' not in r.text
    except Exception:
        return False


def input_cookie_and_save(user_home_url, cfg):
    cookie = cfg.get("cookie")
    if cookie:
        try:
            print(TEXT_INFO_COOKIE_LOCAL)
        except Exception:
            pass
        if test_cookie(cookie, user_home_url):
            try:
                print(TEXT_INFO_COOKIE_OK)
            except Exception:
                pass
            return cookie
        else:
            try:
                print(TEXT_WARN_COOKIE_INVALID)
            except Exception:
                pass

    while True:
        cookie = input(TEXT_PROMPT_COOKIE).strip()
        if not cookie:
            print("[警告] Cookie 不能为空，请重新输入。")
            continue
        if test_cookie(cookie, user_home_url):
            try:
                cfg["cookie"] = cookie
                save_config(cfg)
            except Exception as e:
                print(TEXT_WARN_SAVE_FAIL.format(error=e))
            print(TEXT_INFO_COOKIE_OK)
            return cookie
        else:
            print("[错误] Cookie 无效或已过期，请重新获取。")


# =============================================
# 6. 用户信息与身份解析区块 （SecUID / 昵称 / 主页信息）
# =============================================
def extract_sec_user_id_from_url(user_home_url):
    for pattern in [
        r'/user/([A-Za-z0-9_.\-]+)',
        r'sec_user_id=([A-Za-z0-9_.\-]+)',
        r'(MS4wLjAB[A-Za-z0-9_-]+)'
    ]:
        m = re.search(pattern, user_home_url)
        if m:
            return m.group(1)
    return None


def resolve_short_url_and_extract(url, timeout=10):
    try:
        r = requests.get(url, allow_redirects=True, timeout=timeout)
        final_url = r.url or url
    except Exception:
        final_url = url

    sec = extract_sec_user_id_from_url(final_url)
    if not sec and final_url != url:
        sec = extract_sec_user_id_from_url(url)
    return sec


def get_user_profile_info(headers, sec_user_id):
    url = f"https://www.douyin.com/aweme/v1/web/user/profile/other/?device_platform=webapp&aid=6383&channel=channel_pc_web&sec_user_id={sec_user_id}&from_user_page=1"
    try:
        r = requests.get(url, headers=headers, timeout=REQUEST_TIMEOUT)
        data = r.json()
        if data.get('status_code') == 0 and 'user' in data:
            u = data['user']
            return {'nickname': u.get('nickname') or '', 'aweme_count': u.get('aweme_count', None)}
        return None
    except Exception:
        print(TEXT_WARN_PROFILE_FAIL)
        return None


def fetch_all_aweme_pages(headers, sec_user_id, expected_count=None):
    print(TEXT_INFO_FETCH)
    all_awemes = []

    base_first = "https://www.douyin.com/aweme/v1/web/aweme/post/?device_platform=webapp&aid=6383&channel=channel_pc_web"
    first_url = base_first + f"&sec_user_id={sec_user_id}&max_cursor=0&count={PAGE_COUNT_PER_REQUEST}&locate_query=false&show_live_replay_strategy=1&need_time_list=1&publish_video_strategy_type=2&from_user_page=1&update_version_code=170400"

    try:
        r = requests.get(first_url, headers=headers, timeout=REQUEST_TIMEOUT)
        r.raise_for_status()
        data = r.json()
    except Exception as e:
        print(TEXT_WARN_FETCH_FAIL.format(error=e))
        return all_awemes

    aweme_list = data.get('aweme_list', []) or []
    all_awemes.extend(aweme_list)
    max_cursor = data.get('max_cursor', 0)
    has_more = data.get('has_more', 0) == 1
    page = 1
    total_received = 0

    total_received += len(aweme_list)
    print(TEXT_INFO_FETCH_PAGE.format(page=page, count=len(aweme_list), total=total_received))

    while True:
        time.sleep(DELAY_BETWEEN_PAGES)

        next_url = f"https://www-hj.douyin.com/aweme/v1/web/aweme/post/?" \
                   f"device_platform=webapp&aid=6383&channel=channel_pc_web" \
                   f"&sec_user_id={sec_user_id}&max_cursor={max_cursor}" \
                   f"&count={PAGE_COUNT_PER_REQUEST}&locate_query=false" \
                   f"&show_live_replay_strategy=1&need_time_list=0" \
                   f"&publish_video_strategy_type=2&from_user_page=1&update_version_code=170400"

        try:
            r = requests.get(next_url, headers=headers, timeout=REQUEST_TIMEOUT)
            r.raise_for_status()
            data = r.json()
        except Exception as e:
            print(TEXT_WARN_PAGE_FAIL.format(page=page+1, error=e))
            break

        aweme_list = data.get('aweme_list', []) or []
        if not aweme_list:
            break
        all_awemes.extend(aweme_list)

        max_cursor = data.get('max_cursor', 0)
        has_more = data.get('has_more', 0) == 1
        page += 1
        total_received += len(aweme_list)
        print(TEXT_INFO_FETCH_PAGE.format(page=page, count=len(aweme_list), total=total_received))

        if not has_more:
            break

    print(TEXT_INFO_FETCH_DONE)
    return all_awemes

# =============================================
# 8. 作品解析与任务构建区块 （解析 Aweme → 视频/图片任务）
# =============================================
# ========================= 解析任务 =========================
def extract_media_links_from_aweme(aweme):
    """
    对单个 aweme 的 images 做互斥提取：
      - 如果 image 项包含 video 字段 -> 视为实况图（只提取实况图视频）
      - 否则提取普通图片（最高分辨率 url_list[-1]）
    返回： desc, videos[], images[], live_images[]
    """
    videos, images, live_images = [], [], []
    aweme_id = aweme.get('aweme_id') or ''
    desc = aweme.get('desc', '') or aweme_id or 'no_desc'

    # 转换时间戳 -> YYYY-MM-DD
    ts = aweme.get('create_time')
    date_str = ''
    if ts:
        date_str = datetime.fromtimestamp(ts).strftime("%Y-%m-%d")
        desc = f"{date_str}_{desc}"

    # 自动截断过长描述，避免文件名过长或显示异常
    if len(desc) > MAX_DESC_LENGTH:
        desc = desc[:MAX_DESC_LENGTH] + "......"

    # 普通 aweme.video（视频作品）
    video_info = aweme.get('video', {}) or {}
    bit_rate_list = video_info.get('bit_rate', []) or []
    if bit_rate_list:
        best = max(bit_rate_list, key=lambda x: x.get('bit_rate', 0))
        url_list = best.get('play_addr', {}).get('url_list', [])
        if url_list:
            videos.append(url_list[0])

    # images：逐条处理，互斥
    if 'images' in aweme and isinstance(aweme['images'], list):
        for idx, img in enumerate(aweme['images'], start=1):
            # 若包含 video 字段，则当作实况图（只提取实况图视频），并跳过普通图片提取
            vinfo = img.get('video')
            if vinfo and isinstance(vinfo, dict) and 'bit_rate' in vinfo:
                rates = vinfo.get('bit_rate') or []
                if rates:
                    best = max(rates, key=lambda x: x.get('bit_rate', 0))
                    vurl = best.get('play_addr', {}).get('url_list', [])
                    if vurl:
                        live_images.append(vurl[0])
                continue  # 互斥：有 video 的就不要再提取该条的普通图片

            # 否则按普通图片处理
            url_list = img.get('url_list', [])
            if url_list:
                images.append(url_list[-1])

    return desc, videos, images, live_images, date_str


def parse_all_awemes_to_tasks(all_awemes):
    """
    将所有 aweme 解析为任务：
      - video_tasks: 普通视频任务
      - image_tasks: 普通图片 + 实况图（都放 images 文件夹）
      - album_count: 包含普通图片或实况图的 aweme 数（图集作品数）
      - image_count: 普通图片总数
      - live_count: 实况图总数
    """
    video_tasks, image_tasks = [], []
    album_count = 0
    image_count = 0
    live_count = 0

    for aweme in all_awemes:
        desc, videos, images, live_images, date_str = extract_media_links_from_aweme(aweme)

        # 视频任务
        for vurl in videos:
            ext = get_extension_from_url(vurl, '.mp4')
            video_tasks.append({'url': vurl, 'desc': desc, 'ext': ext, 'date': date_str})

        # 如果这个 aweme 有普通图片或实况图，则视为一个图集作品
        if images or live_images:
            album_count += 1

        # 普通图片（按张）
        for idx, iurl in enumerate(images, start=1):
            ext = get_extension_from_url(iurl, '.jpg')
            image_tasks.append({'url': iurl, 'desc': f"{desc}_p{idx}", 'ext': ext, 'date': date_str})
        image_count += len(images)

        # 实况图（按张），也放到 image_tasks，但 ext 可能是 .mp4
        for idx, lvurl in enumerate(live_images, start=1):
            ext = get_extension_from_url(lvurl, '.mp4')
            image_tasks.append({'url': lvurl, 'desc': f"{desc}_live{idx}", 'ext': ext, 'date': date_str})
        live_count += len(live_images)

    return video_tasks, image_tasks, album_count, image_count, live_count

# =============================================
# 9. 下载任务执行区块 （多线程下载视频/图片）
# =============================================
# ========================= 下载 =========================
def download_single_file(task, base_folder, is_image=False):
    url = task['url']
    desc = task['desc']
    ext = task['ext']

    folder = os.path.join(base_folder, 'images') if is_image else base_folder
    safe_mkdir(folder)

    path = generate_unique_filename(desc, ext, folder, url)
    headers = {'User-Agent': USER_AGENT, 'Referer': 'https://www.douyin.com/'}
    try:
        with requests.get(url, headers=headers, stream=True, timeout=30) as r:
            r.raise_for_status()
            with open(path, 'wb') as f:
                for chunk in r.iter_content(8192):
                    f.write(chunk)
        # 返回相对路径，供日志使用（成功时）
        relative_name = os.path.relpath(path, base_folder)
        return relative_name
    except Exception:
        return None


def download_tasks(video_tasks, image_tasks, base_folder, threads):
    print(TEXT_INFO_DOWNLOAD_START.format(
        vcount=len(video_tasks),
        icount=len(image_tasks),
        threads=threads
    ))

    results = {
        'success': 0, 'fail': 0,
        'video_success': 0, 'image_success': 0,
        'fail_list_videos': [], 'fail_list_images': [],
        'success_files': set()
    }

    all_tasks = ([(t, False) for t in video_tasks] +
                 [(t, True) for t in image_tasks])

    with ThreadPoolExecutor(max_workers=threads) as ex, tqdm(total=len(all_tasks), desc="下载进度", unit="文件") as bar:
        future_map = {ex.submit(download_single_file, t, base_folder, is_img): (t, is_img) for t, is_img in all_tasks}
        for fut in as_completed(future_map):
            task, is_img = future_map[fut]
            try:
                relpath = fut.result()
                ok = relpath is not None
            except Exception:
                ok = False
                relpath = None

            if ok:
                results['success'] += 1
                # 收集成功文件相对路径用于写日志和后续合并
                results['success_files'].add(relpath)
                if is_img:
                    results['image_success'] += 1
                else:
                    results['video_success'] += 1
            else:
                results['fail'] += 1
                if is_img:
                    results['fail_list_images'].append(task)
                else:
                    results['fail_list_videos'].append(task)
            bar.update(1)
    return results

# =============================================
# 10. 日志与去重模块区块 （记录已下载，避免重复）
# =============================================
# ========================= 主程序 =========================
def filter_existing_tasks(video_tasks, image_tasks, existing_files):
    """
    过滤已有日志中存在的任务（通过预期文件名对比）
    返回新的 video_tasks, image_tasks, skipped_count
    """
    new_vtasks, new_itasks = [], []
    skipped = 0
    for t in video_tasks:
        expected = build_expected_filename(t['desc'], t['ext'], False)
        if expected in existing_files:
            skipped += 1
        else:
            new_vtasks.append(t)
    for t in image_tasks:
        expected = build_expected_filename(t['desc'], t['ext'], True)
        if expected in existing_files:
            skipped += 1
        else:
            new_itasks.append(t)
    return new_vtasks, new_itasks, skipped

# =============================================
# 11. 主程序流程区块 （Main 控制逻辑）
# =============================================
# 新增：下载路径输入函数（保留原注释/行为风格）
def input_download_path(nickname, cfg):
    """
    获取并保存下载路径：
    - 优先读取 cfg['path']（来自 config.txt）
    - 如果没有，则按照原逻辑提示输入；回车使用当前目录并保存
    """
    custom_base = cfg.get("path")
    if custom_base:
        print("[信息] 检测到已保存路径")
    else:
        # 与原代码行为保持一致的提示文本
        while True:
            custom_base = input(TEXT_PROMPT_PATH).strip()
            if not custom_base:
                custom_base = os.getcwd()
            else:
                custom_base = custom_base.strip('"').strip("'")
            # 尝试创建目录（保持原有的循环逻辑）
            base_folder = os.path.join(custom_base, sanitize_filename(nickname) or "Douyin_Downloads")
            if safe_mkdir(base_folder):
                # 保存选择到 cfg
                cfg["path"] = custom_base
                save_config(cfg)
                return base_folder
            else:
                print("[提示] 请重新输入有效的下载路径。")
    # 若 cfg 中已有路径，直接使用（并保证目录存在）
    base_folder = os.path.join(custom_base, sanitize_filename(nickname) or "Douyin_Downloads")
    if not safe_mkdir(base_folder):
        print("[错误] 创建目录失败，请检查路径")
        sys.exit(1)
    return base_folder


def main():
    print(TEXT_LINE)
    print(TEXT_APP_NAME)
    print(TEXT_DESCRIPTION)
    print(TEXT_LINE)

    url = input(TEXT_PROMPT_URL).strip()
    # 先尝试解析短链/重定向再提取 sec_user_id（兼容 v.douyin.com 等短链）
    sec_uid = resolve_short_url_and_extract(url)
    if not sec_uid:
        print("[错误] 无法解析 sec_user_id，请检查链接或使用浏览器复制的真实链接。")
        return

    # 加载配置（config.txt JSON），用于 cookie 与 path 的持久化
    cfg = load_config()

    # cookie 读取/保存：使用 cfg 优先，其次要求输入并保存
    cookie = input_cookie_and_save(url, cfg)
    headers = {'User-Agent': USER_AGENT, 'Cookie': cookie, 'Referer': url}
    profile = get_user_profile_info(headers, sec_uid)
    if not profile:
        input(TEXT_PROMPT_RESTART)
        os.execv(sys.executable, [sys.executable] + sys.argv)

    nickname = profile.get('nickname') or ''
    print(TEXT_INFO_USER.format(nickname=nickname or '未知用户'))
    print()

    # ====== 自定义下载路径输入（改为使用 config.txt 存储/读取） ======
    base_folder = input_download_path(nickname, cfg)
    print(TEXT_INFO_BASE_DIR.format(path=base_folder))


    all_awemes = fetch_all_aweme_pages(headers, sec_uid, profile.get('aweme_count'))
    if not all_awemes:
        print(TEXT_WARN_EMPTY_RESULT)
        return

    video_tasks, image_tasks, album_count, image_count, live_count = parse_all_awemes_to_tasks(all_awemes)
    total = len(all_awemes)

    # 输出统计（为0则不显示对应行）
    print(TEXT_INFO_WORKS_TOTAL.format(total=total))
    if len(video_tasks) > 0:
        print(TEXT_INFO_WORKS_VIDEO.format(count=len(video_tasks)))
    if album_count > 0:
        print(TEXT_INFO_WORKS_ALBUM.format(count=album_count, images=image_count, live_images=live_count))

    # ====== 加载旧日志 ======
    log_file = os.path.join(base_folder, f"{sanitize_filename(nickname)}_下载日志.txt")
    existing_files = set()
    if os.path.exists(log_file):
        with open(log_file, "r", encoding="utf-8") as f:
            existing_files = set(line.strip() for line in f if line.strip())

    # ====== 过滤已存在的任务（如果日志中已有则跳过） ======
    video_tasks, image_tasks, skipped = filter_existing_tasks(video_tasks, image_tasks, existing_files)
    if skipped > 0:
        print()
        print(f"[提示] 已跳过 {skipped} 个已存在的文件，不再重复下载")
        print()

    threads = input(TEXT_PROMPT_THREAD).strip()
    threads = int(threads) if threads.isdigit() else DEFAULT_THREAD_COUNT

    stats = download_tasks(video_tasks, image_tasks, base_folder, threads)

    # 自动重试机制（每次把上次失败的任务重试）
    retry_count = 0
    while (stats['fail_list_videos'] or stats['fail_list_images']) and retry_count < MAX_AUTOMATIC_RETRIES:
        retry_count += 1
        print(TEXT_INFO_RETRY.format(
            retry=retry_count, max_retry=MAX_AUTOMATIC_RETRIES,
            vfail=len(stats['fail_list_videos']),
            ifail=len(stats['fail_list_images'])
        ))
        retry_stats = download_tasks(stats['fail_list_videos'], stats['fail_list_images'], base_folder, threads)
        # 合并统计
        stats['success'] += retry_stats['success']
        stats['video_success'] += retry_stats['video_success']
        stats['image_success'] += retry_stats['image_success']
        stats['fail'] = retry_stats['fail']
        stats['fail_list_videos'] = retry_stats['fail_list_videos']
        stats['fail_list_images'] = retry_stats['fail_list_images']
        # 合并成功文件集合（用于写入日志）
        stats['success_files'].update(retry_stats.get('success_files', set()))

    # 如果仍有失败，询问是否重试整脚本
    if stats['fail_list_videos'] or stats['fail_list_images']:
        choice = input(TEXT_PROMPT_RETRY_DECISION).strip().lower()
        if choice == 'y':
            os.execv(sys.executable, [sys.executable] + sys.argv)

    # ====== 更新日志 ======
    all_files = existing_files.union(stats.get('success_files', set()))
    with open(log_file, "w", encoding="utf-8") as f:
        for name in sorted(all_files):
            f.write(name + "\n")
    print(f"[日志] 下载日志已更新: {log_file}")

    print("\n" + TEXT_LINE)
    print(TEXT_INFO_SUMMARY)
    print(TEXT_LINE)
    print(f"视频成功: {stats['video_success']}")
    print(f"图片 + 实况成功: {stats['image_success']}")
    print(TEXT_RESULT_SUCCESS.format(success=stats['success'], fail=stats['fail']))
    print(TEXT_RESULT_PATH.format(path=base_folder))
    print(TEXT_LINE)
    print(TEXT_INFO_DONE)
    print(TEXT_LINE)


# =============================================
# 12. 无交互执行与 GUI 支持（顶层定义）
# =============================================
def process_run(url, cookie, base_path, threads, log_callback=print):
    """
    使用传入参数执行一次完整流程（无交互）。
    log_callback: 函数，接收单条日志字符串（用于 GUI 时回调显示）
    返回: dict 状态信息或 None（发生错误）
    """
    def log(s):
        try:
            log_callback(s)
        except Exception:
            pass

    log(TEXT_LINE)
    log(TEXT_APP_NAME)
    log(TEXT_DESCRIPTION)
    log(TEXT_LINE)

    sec_uid = resolve_short_url_and_extract(url)
    if not sec_uid:
        log("[错误] 无法解析 sec_user_id，请检查链接或使用浏览器复制的真实链接。")
        return None

    cfg = load_config()

    # 使用传入 cookie 优先并保存
    if cookie:
        cfg['cookie'] = cookie
        save_config(cfg)

    # 验证 cookie
    if not test_cookie(cfg.get('cookie', ''), url):
        log(TEXT_WARN_COOKIE_INVALID)
        return None

    headers = {'User-Agent': USER_AGENT, 'Cookie': cfg.get('cookie', ''), 'Referer': url}
    profile = get_user_profile_info(headers, sec_uid)
    if not profile:
        log(TEXT_WARN_PROFILE_FAIL)
        return None

    nickname = profile.get('nickname') or ''
    log(TEXT_INFO_USER.format(nickname=nickname or '未知用户'))

    # 处理下载路径
    if base_path:
        custom_base = base_path
        base_folder = os.path.join(custom_base, sanitize_filename(nickname) or "Douyin_Downloads")
        if not safe_mkdir(base_folder):
            log(f"[错误] 创建目录失败: {base_folder}")
            return None
        cfg['path'] = custom_base
        save_config(cfg)
    else:
        base_folder = input_download_path(nickname, cfg)

    log(TEXT_INFO_BASE_DIR.format(path=base_folder))

    all_awemes = fetch_all_aweme_pages(headers, sec_uid, profile.get('aweme_count'))
    if not all_awemes:
        log(TEXT_WARN_EMPTY_RESULT)
        return None

    video_tasks, image_tasks, album_count, image_count, live_count = parse_all_awemes_to_tasks(all_awemes)
    total = len(all_awemes)

    log(TEXT_INFO_WORKS_TOTAL.format(total=total))
    if len(video_tasks) > 0:
        log(TEXT_INFO_WORKS_VIDEO.format(count=len(video_tasks)))
    if album_count > 0:
        log(TEXT_INFO_WORKS_ALBUM.format(count=album_count, images=image_count, live_images=live_count))

    # 加载旧日志
    log_file = os.path.join(base_folder, f"{sanitize_filename(nickname)}_下载日志.txt")
    existing_files = set()
    if os.path.exists(log_file):
        try:
            with open(log_file, "r", encoding="utf-8") as f:
                existing_files = set(line.strip() for line in f if line.strip())
        except Exception:
            existing_files = set()

    video_tasks, image_tasks, skipped = filter_existing_tasks(video_tasks, image_tasks, existing_files)
    if skipped > 0:
        log(f"[提示] 已跳过 {skipped} 个已存在的文件，不再重复下载")

    threads = int(threads) if isinstance(threads, int) or (isinstance(threads, str) and threads.isdigit()) else DEFAULT_THREAD_COUNT

    stats = download_tasks(video_tasks, image_tasks, base_folder, threads)

    # 自动重试（简化：不弹交互，按 MAX_AUTOMATIC_RETRIES 自动重试）
    retry_count = 0
    while (stats['fail_list_videos'] or stats['fail_list_images']) and retry_count < MAX_AUTOMATIC_RETRIES:
        retry_count += 1
        log(TEXT_INFO_RETRY.format(
            retry=retry_count, max_retry=MAX_AUTOMATIC_RETRIES,
            vfail=len(stats['fail_list_videos']),
            ifail=len(stats['fail_list_images'])
        ))
        retry_stats = download_tasks(stats['fail_list_videos'], stats['fail_list_images'], base_folder, threads)
        stats['success'] += retry_stats['success']
        stats['video_success'] += retry_stats['video_success']
        stats['image_success'] += retry_stats['image_success']
        stats['fail'] = retry_stats['fail']
        stats['fail_list_videos'] = retry_stats['fail_list_videos']
        stats['fail_list_images'] = retry_stats['fail_list_images']
        stats['success_files'].update(retry_stats.get('success_files', set()))

    # 不再写入基于昵称的下载日志：已通过目标目录中是否存在文件来去重
    log(f"[日志] 本次成功下载文件 {len(stats.get('success_files', set()))} 个，目录: {base_folder}")

    log("\n" + TEXT_LINE)
    log(TEXT_INFO_SUMMARY)
    log(TEXT_LINE)
    log(f"视频成功: {stats['video_success']}")
    log(f"图片 + 实况成功: {stats['image_success']}")
    log(TEXT_RESULT_SUCCESS.format(success=stats['success'], fail=stats['fail']))
    log(TEXT_RESULT_PATH.format(path=base_folder))
    log(TEXT_LINE)
    log(TEXT_INFO_DONE)
    log(TEXT_LINE)

    return stats


def run_gui():
    """PyQt6 GUI：保持与 Tk 界面等效的核心功能：
    - 自动从 config.txt 读取 cookie 与 path
    - 获取作品并在列表显示（可多选/复选）
    - 查看日志、选择/反选、全选
    - 开始下载（在后台线程，显示进度）
    """
    try:
        from PyQt6 import QtWidgets, QtCore, QtGui
        from PyQt6.QtCore import Qt
    except Exception as e:
        print(f"[错误] PyQt6 未安装或无法导入: {e}\n请安装 PyQt6 后重试（pip install PyQt6）。")
        return

    cfg = load_config()

    class Worker(QtCore.QObject):
        log_signal = QtCore.pyqtSignal(str)
        progress_signal = QtCore.pyqtSignal(int, int)
        tasks_signal = QtCore.pyqtSignal(object, object, object)
        view_switch_signal = QtCore.pyqtSignal(str)
        finished = QtCore.pyqtSignal()

        def __init__(self, parent=None):
            super().__init__(parent)

        def fetch_tasks(self, url, cookie):
            try:
                headers = {'User-Agent': USER_AGENT, 'Cookie': cookie, 'Referer': url}
                self.log_signal.emit('[信息] 开始获取用户信息')
                sec = resolve_short_url_and_extract(url)
                if not sec:
                    self.log_signal.emit('[错误] 无法解析 sec_user_id')
                    self.finished.emit()
                    return
                profile = get_user_profile_info(headers, sec)
                if not profile:
                    self.log_signal.emit('[错误] 获取用户信息失败，Cookie 可能无效')
                    self.finished.emit()
                    return
                nickname = profile.get('nickname', '') or ''
                self.log_signal.emit(f"[信息] 抖音用户: {nickname}")

                # 分页增量获取：每获取一页就解析并 emit 给 GUI
                page = 1
                max_cursor = 0
                while True:
                    if page == 1:
                        req_url = "https://www.douyin.com/aweme/v1/web/aweme/post/?device_platform=webapp&aid=6383&channel=channel_pc_web"
                        req_url = req_url + f"&sec_user_id={sec}&max_cursor=0&count={PAGE_COUNT_PER_REQUEST}&locate_query=false&show_live_replay_strategy=1&need_time_list=1&publish_video_strategy_type=2&from_user_page=1&update_version_code=170400"
                    else:
                        req_url = f"https://www-hj.douyin.com/aweme/v1/web/aweme/post/?device_platform=webapp&aid=6383&channel=channel_pc_web&sec_user_id={sec}&max_cursor={max_cursor}&count={PAGE_COUNT_PER_REQUEST}&locate_query=false&show_live_replay_strategy=1&need_time_list=0&publish_video_strategy_type=2&from_user_page=1&update_version_code=170400"
                    try:
                        r = requests.get(req_url, headers=headers, timeout=REQUEST_TIMEOUT)
                        r.raise_for_status()
                        data = r.json()
                    except Exception as e:
                        self.log_signal.emit(f"[警告] 第 {page} 页请求异常: {e}")
                        break

                    # 解析本页返回的 aweme 列表并处理（在请求成功后执行）
                    aweme_list = data.get('aweme_list', []) or []
                    if not aweme_list:
                        break
                    vtasks, itasks, album_count, image_count, live_count = parse_all_awemes_to_tasks(aweme_list)
                    # 将本页任务发回 GUI（增量）
                    try:
                        self.tasks_signal.emit(vtasks, itasks, nickname)
                    except Exception as e:
                        self.log_signal.emit(f"[警告] tasks_signal.emit 失败: {e}")
                    # 维护累计计数并输出格式：累计 total_received【+ 本页count】
                    try:
                        if not hasattr(self, '_total_received'):
                            self._total_received = 0
                        prev = self._total_received
                        self._total_received += len(aweme_list)
                        self.log_signal.emit(TEXT_INFO_FETCH_PAGE.format(page=page, count=len(aweme_list), total=self._total_received))
                    except Exception:
                        self.log_signal.emit(TEXT_INFO_FETCH_PAGE.format(page=page, count=len(aweme_list), total=len(aweme_list)))
                    max_cursor = data.get('max_cursor', 0)
                    has_more = data.get('has_more', 0) == 1
                    page += 1
                    time.sleep(DELAY_BETWEEN_PAGES)
                    if not has_more:
                        break
                self.log_signal.emit('[完成] 获取完成')
            except Exception as e:
                self.log_signal.emit(f"[错误] 获取异常: {e}")
            finally:
                self.finished.emit()

        def download_tasks(self, vtasks, itasks, base_folder, threads):
            try:
                # 过滤已存在
                self.log_signal.emit('[信息] 开始下载')
                all_tasks = []
                results_success_files = set()
                for t in vtasks:
                    expected = build_expected_filename(t['desc'], t['ext'], False)
                    fullpath = os.path.join(base_folder, expected)
                    if os.path.exists(fullpath):
                        self.log_signal.emit(f"[跳过] 已存在: {expected}")
                        results_success_files.add(expected)
                    else:
                        all_tasks.append((t, False))
                for t in itasks:
                    expected = build_expected_filename(t['desc'], t['ext'], True)
                    fullpath = os.path.join(base_folder, expected)
                    if os.path.exists(fullpath):
                        self.log_signal.emit(f"[跳过] 已存在: {expected}")
                        results_success_files.add(expected)
                    else:
                        all_tasks.append((t, True))

                total = len(all_tasks)
                done = 0
                from concurrent.futures import ThreadPoolExecutor, as_completed
                # 支持暂停：在提交任务前检查 self._pause_requested
                future_map = {}
                total = len(all_tasks)
                done = 0
                from concurrent.futures import ThreadPoolExecutor, as_completed
                # 支持暂停：在提交任务前检查 self._pause_requested
                future_map = {}
                with ThreadPoolExecutor(max_workers=threads) as ex:
                    for t, is_img in all_tasks:
                        # 如果请求暂停则等待（协作式）
                        while getattr(self, '_pause_requested', False):
                            time.sleep(0.2)
                        f = ex.submit(download_single_file, t, base_folder, is_img)
                        future_map[f] = (t, is_img)

                    for fut in as_completed(future_map):
                        task, is_img = future_map[fut]
                        try:
                            rel = fut.result()
                            ok = rel is not None
                        except Exception as e:
                            ok = False
                            rel = None
                        if ok:
                            results_success_files.add(rel)
                            # 记录已完成任务为结构化记录，便于 UI 区分和匹配
                            try:
                                if not hasattr(self, '_completed_tasks'):
                                    self._completed_tasks = []
                                rec = {'task': task, 'is_image': is_img, 'path': rel}
                                self._completed_tasks.append(rec)
                            except Exception:
                                pass
                            self.log_signal.emit(f"[下载成功] {rel}")
                        else:
                            # 记录失败任务到 worker 的失败列表，供界面查看（结构化）
                            try:
                                if not hasattr(self, '_failed_tasks'):
                                    self._failed_tasks = []
                                rec = dict(task)
                                rec['is_image'] = is_img
                                self._failed_tasks.append(rec)
                            except Exception:
                                pass
                            try:
                                self.log_signal.emit(f"[下载失败] {task.get('desc')}")
                            except Exception:
                                pass
                        done += 1
                        self.progress_signal.emit(done, max(1, total))

                # 不再写入基于昵称的下载日志，统一通过目标目录文件存在检测避免重复
                # 若需要，可将 results_success_files 写入到一个 global 日志文件，这里只记录一条提示
                try:
                    self.log_signal.emit(f"[日志] 本次成功下载文件 {len(results_success_files)} 个（目录: {base_folder}）")
                except Exception:
                    pass

                # 下载完成后请求主界面切换到已完成视图（若有成功项）
                try:
                    if getattr(self, '_completed_tasks', None):
                        self.view_switch_signal.emit('completed')
                except Exception:
                    pass

            except Exception as e:
                self.log_signal.emit(f"[错误] 下载异常: {e}")
            finally:
                self.finished.emit()


    class MainWindow(QtWidgets.QMainWindow):
        def __init__(self):
            super().__init__()
            self.setWindowTitle(TEXT_APP_NAME)
            self.resize(1000, 700)
            central = QtWidgets.QWidget()
            self.setCentralWidget(central)
            lay = QtWidgets.QVBoxLayout(central)

            # 表单区
            form = QtWidgets.QGridLayout()
            lay.addLayout(form)
            form.addWidget(QtWidgets.QLabel('主页链接:'), 0, 0)
            self.url_edit = QtWidgets.QLineEdit()
            form.addWidget(self.url_edit, 0, 1, 1, 3)
            # 将获取按钮放在链接输入框右侧
            self.fetch_btn = QtWidgets.QPushButton('获取作品')
            form.addWidget(self.fetch_btn, 0, 4)

            form.addWidget(QtWidgets.QLabel('Cookie:'), 1, 0)
            self.cookie_edit = QtWidgets.QTextEdit()
            self.cookie_edit.setFixedHeight(70)
            self.cookie_edit.setPlainText(cfg.get('cookie', ''))
            form.addWidget(self.cookie_edit, 1, 1, 1, 4)

            form.addWidget(QtWidgets.QLabel('保存路径:'), 2, 0)
            self.path_edit = QtWidgets.QLineEdit(cfg.get('path', ''))
            form.addWidget(self.path_edit, 2, 1)
            self.browse_btn = QtWidgets.QPushButton('选择路径')
            form.addWidget(self.browse_btn, 2, 2)
            form.addWidget(QtWidgets.QLabel('线程数:'), 2, 3)
            self.threads_spin = QtWidgets.QSpinBox()
            self.threads_spin.setMinimum(1)
            self.threads_spin.setMaximum(64)
            self.threads_spin.setValue(DEFAULT_THREAD_COUNT)
            # 去掉上下箭头按钮（无按钮样式）
            try:
                self.threads_spin.setButtonSymbols(QtWidgets.QAbstractSpinBox.ButtonSymbols.NoButtons)
            except Exception:
                try:
                    self.threads_spin.setButtonSymbols(QtWidgets.QSpinBox.ButtonSymbols.NoButtons)
                except Exception:
                    pass
            form.addWidget(self.threads_spin, 2, 4)
            # 让 SpinBox 看起来像 QLineEdit，一致的外观
            self.threads_spin.setStyleSheet("""
                QSpinBox {
                    border: 1px solid #dde9f7;
                    padding: 6px;
                    border-radius: 4px;
                    background: #ffffff;
                    color: #222222;
                }
                QSpinBox:focus {
                    border: 1px solid #5aa6ff;
                    background: #f9fcff;
                }
            """)


            # 按钮区
            btns = QtWidgets.QHBoxLayout()
            lay.addLayout(btns)
            # self.fetch_btn 已在上方 URL 行创建
            self.download_btn = QtWidgets.QPushButton('开始下载')
            # 按钮顺序调整：开始下载 / 下载失败 / 查看日志
            self.view_log_btn = QtWidgets.QPushButton('查看日志')
            self.clear_btn = QtWidgets.QPushButton('清空列表')
            self.select_all_btn = QtWidgets.QPushButton('全选')
            self.invert_btn = QtWidgets.QPushButton('反选')
            # fetch_btn 已放在 URL 行，不再在此重复添加
            btns.addWidget(self.download_btn)
            btns.addWidget(self.view_log_btn)
            btns.addWidget(self.clear_btn)
            btns.addStretch()
            btns.addWidget(QtWidgets.QLabel('当前用户:'))
            self.nickname_label = QtWidgets.QLabel('')
            font = self.nickname_label.font()
            font.setBold(True)
            self.nickname_label.setFont(font)
            btns.addWidget(self.nickname_label)
            btns.addWidget(self.select_all_btn)
            btns.addWidget(self.invert_btn)

            # 列表区
            self.tree = QtWidgets.QTreeWidget()
            # 列顺序：选择 | 序号 | 发布日期 | 描述 | 类型(最右)
            self.tree.setHeaderLabels(['选择', '序号', '发布日期', '描述', '类型'])
            # 使类型列标题右对齐
            try:
                hitem = self.tree.headerItem()
                hitem.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
            except Exception:
                pass
            # 首列宽度根据标题文字计算，保持最小占位并固定
            width0 = self.tree.fontMetrics().horizontalAdvance('选择') + 16
            self.tree.setColumnWidth(0, width0)
            self.tree.setColumnWidth(1, 60)
            self.tree.setColumnWidth(2, 100)
            self.tree.setColumnWidth(3, 480)
            # 精确设置为 8 个等宽字符的宽度（基于 'm' 宽度）再加少量内边距
            fm = self.tree.fontMetrics()
            type_w = fm.horizontalAdvance('m' * 8) + 12
            self.tree.setColumnWidth(4, type_w)
            # 固定第一列宽度并禁止拖动列
            # 固定第一列宽度并禁止拖动，支持鼠标框选（ExtendedSelection）
            header = self.tree.header()
            header.setSectionResizeMode(0, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionResizeMode(1, QtWidgets.QHeaderView.ResizeMode.ResizeToContents)
            header.setSectionResizeMode(2, QtWidgets.QHeaderView.ResizeMode.ResizeToContents)
            header.setSectionResizeMode(3, QtWidgets.QHeaderView.ResizeMode.Stretch)
            # keep 类型 column fixed
            header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
            header.setSectionsMovable(False)
            # 禁止最后一列被自动拉伸（我们在 showEvent/resizeEvent 中手动固定宽度）
            try:
                header.setStretchLastSection(False)
            except Exception:
                pass
            self.tree.setRootIsDecorated(False)
            self.tree.setUniformRowHeights(True)
            self.tree.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.ExtendedSelection)
            self.tree.setSelectionBehavior(QtWidgets.QAbstractItemView.SelectionBehavior.SelectRows)
            # 给列表添加边框与行分隔线样式
            self.tree.setFrameShape(QtWidgets.QFrame.Shape.Box)
            # 使用更现代的树样式：去除重边框、开启交替行色以利于阅读
            self.tree.setAlternatingRowColors(True)
            self.tree.setStyleSheet(
                "QTreeWidget { background: #ffffff; border: 1px solid #e6eef8; }"
                "QTreeWidget::item { padding:6px 4px; color: #222222; }"
                "QTreeWidget::item:selected { background: #e6f2ff; color: #000000; }"
            )
            lay.addWidget(self.tree)

            # 进度与日志
            bottom = QtWidgets.QHBoxLayout()
            lay.addLayout(bottom)
            self.progress = QtWidgets.QProgressBar()
            self.progress.setFixedHeight(26)
            self.progress.setTextVisible(True)
            try:
                # 无边框样式：去掉外框，仅保留浅背景与进度块
                self.progress.setStyleSheet(
                    "QProgressBar { border: none; border-radius: 0px; background: #f0f0f0; text-align: center; }"
                    "QProgressBar::chunk { background-color: #5aa6ff; border-radius: 0px; }")
            except Exception:
                pass
            bottom.addWidget(self.progress)
            self.log_view = QtWidgets.QTextEdit()
            self.log_view.setReadOnly(True)
            self.log_view.setFixedHeight(150)
            self.log_view.hide()
            lay.addWidget(self.log_view)

            # 状态
            self.status = QtWidgets.QLabel('')
            lay.addWidget(self.status)

            # 数据
            self.vtasks = []
            self.itasks = []

            # 线程/worker
            self.worker = Worker()
            self.thread = None

            # 放大按钮字体
            btn_font = QtGui.QFont()
            btn_font.setPointSize(11)
            for b in (self.fetch_btn, self.download_btn, self.view_log_btn, self.clear_btn, self.select_all_btn, self.invert_btn):
                b.setFont(btn_font)
                try:
                    b.setCursor(Qt.PointingHandCursor)
                except Exception:
                    pass
                try:
                    # 统一为扁平按钮，样式由全局样式表控制；保留可见焦点/按下状态
                    b.setFlat(False)
                except Exception:
                    pass
            # 给“清空列表”一个醒目的危险色样式（不影响其它按钮）
            try:
                self.clear_btn.setStyleSheet('background:#d9534f; color:white; padding:6px 8px;')
            except Exception:
                pass

            # 事件绑定
            self.browse_btn.clicked.connect(self.on_browse)
            self.fetch_btn.clicked.connect(self.on_fetch)
            self.download_btn.clicked.connect(self.on_download)
            self.view_log_btn.clicked.connect(self.on_view_log)
            # failed button removed
            self.select_all_btn.clicked.connect(self.on_select_all)
            self.invert_btn.clicked.connect(self.on_invert)
            self.clear_btn.clicked.connect(self.on_clear_list)

            self.worker.log_signal.connect(self.append_log)
            self.worker.tasks_signal.connect(self.on_tasks_received)
            self.worker.progress_signal.connect(self.on_progress)
            self.worker.finished.connect(self.on_worker_finished)
            # 当 worker 请求切换视图时刷新界面
            try:
                self.worker.view_switch_signal.connect(self.refresh_tree)
            except Exception:
                pass
            # worker 状态：支持暂停控制与失败记录
            self.worker._pause_requested = False
            self.worker._failed_tasks = []
            # 绑定复选/选择相关信号
            self.tree.itemSelectionChanged.connect(self.on_tree_selection_changed)
            self.tree.itemChanged.connect(self.on_tree_item_changed)

            # 标记以避免在程序化修改中触发互相更新循环
            self._programmatic_change = False
            # 当前视图：queue / completed / failed
            self.current_view = 'queue'
            # 最近的状态文本（不含选择计数），由 append_log 更新
            self._last_status_text = ''

        def append_log(self, text):
            ts = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
            line = f"[{ts}] {text}"
            self.log_view.append(line)
            # 保存基础状态文本并刷新状态标签（包括选择计数）
            try:
                self._last_status_text = text
            except Exception:
                self._last_status_text = ''
            self.update_status_label()

        def update_status_label(self):
            # 计算已选择项数并在状态栏显示
            try:
                count = 0
                for i in range(self.tree.topLevelItemCount()):
                    it = self.tree.topLevelItem(i)
                    if it.checkState(0) == Qt.CheckState.Checked:
                        count += 1
                base = getattr(self, '_last_status_text', '') or ''
                if count > 0:
                    self.status.setText(f"{base} （已选择 {count} 个）")
                else:
                    self.status.setText(base)
            except Exception:
                try:
                    self.status.setText(getattr(self, '_last_status_text', ''))
                except Exception:
                    pass

        def refresh_tree(self, mode=None):
            """根据 mode 刷新主列表：
            mode: 'queue' (默认、过滤已完成), 'completed', 'failed'
            """
            mode = mode or self.current_view
            self.current_view = mode
            # 暂时禁止程序化更改触发选择/复选联动
            self._programmatic_change = True
            try:
                self.tree.clear()
                idx = 1
                completed = getattr(self.worker, '_completed_tasks', []) or []
                completed_urls = set()
                for rec in completed:
                    try:
                        completed_urls.add(rec.get('task', {}).get('url'))
                    except Exception:
                        pass

                if mode == 'failed':
                    failed = getattr(self.worker, '_failed_tasks', []) or []
                    for t in failed:
                        date_display = t.get('date', '')
                        desc_display = t.get('desc', '')
                        kind = 'image' if t.get('is_image') else 'video'
                        item = QtWidgets.QTreeWidgetItem(self.tree, [' ', str(idx), date_display, desc_display, kind])
                        item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
                        item.setCheckState(0, Qt.CheckState.Unchecked)
                        item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
                        item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
                        item.setData(0, Qt.ItemDataRole.UserRole, (t, t.get('is_image', False)))
                        idx += 1
                    return

                if mode == 'completed':
                    for rec in completed:
                        t = rec.get('task', {})
                        is_img = rec.get('is_image', False)
                        date_display = t.get('date', '')
                        desc_display = t.get('desc', '')
                        kind = 'image' if is_img else 'video'
                        item = QtWidgets.QTreeWidgetItem(self.tree, [' ', str(idx), date_display, desc_display, kind])
                        item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
                        item.setCheckState(0, Qt.CheckState.Unchecked)
                        item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
                        item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
                        item.setData(0, Qt.ItemDataRole.UserRole, (t, is_img))
                        idx += 1
                    return

                # 默认 or 'queue'：显示当前待下载队列，排除已完成项
                completed_test_urls = completed_urls
                for t in (self.vtasks_all or []):
                    if t.get('url') in completed_test_urls:
                        continue
                    date_display = t.get('date', '')
                    desc_display = t.get('desc', '')
                    item = QtWidgets.QTreeWidgetItem(self.tree, [' ', str(idx), date_display, desc_display, 'video'])
                    item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
                    item.setCheckState(0, Qt.CheckState.Unchecked)
                    item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
                    item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
                    item.setData(0, Qt.ItemDataRole.UserRole, (t, False))
                    idx += 1
                for t in (self.itasks_all or []):
                    if t.get('url') in completed_test_urls:
                        continue
                    date_display = t.get('date', '')
                    desc_display = t.get('desc', '')
                    item = QtWidgets.QTreeWidgetItem(self.tree, [' ', str(idx), date_display, desc_display, 'image'])
                    item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
                    item.setCheckState(0, Qt.CheckState.Unchecked)
                    item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
                    item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
                    item.setData(0, Qt.ItemDataRole.UserRole, (t, True))
                    idx += 1
            finally:
                self._programmatic_change = False

        def on_tree_selection_changed(self):
            if getattr(self, '_programmatic_change', False):
                return
            # selection -> checkbox: 选中行的复选框置为 checked
            self._programmatic_change = True
            try:
                sel = set()
                for i in range(self.tree.topLevelItemCount()):
                    it = self.tree.topLevelItem(i)
                    if it.isSelected():
                        it.setCheckState(0, Qt.CheckState.Checked)
                    else:
                        it.setCheckState(0, Qt.CheckState.Unchecked)
            finally:
                self._programmatic_change = False
            # 更新选择计数显示
            try:
                self.update_status_label()
            except Exception:
                pass

        def on_tree_item_changed(self, item, column):
            if getattr(self, '_programmatic_change', False):
                return
            # checkbox -> selection: 若 checkbox 被勾选，则选中该行；反之取消选中
            self._programmatic_change = True
            try:
                if column == 0:
                    state = item.checkState(0)
                    if state == Qt.CheckState.Checked:
                        item.setSelected(True)
                    else:
                        item.setSelected(False)
            finally:
                self._programmatic_change = False

        def on_progress(self, done, total):
            self.progress.setMaximum(total)
            self.progress.setValue(done)
            try:
                pct = int((done / max(1, total)) * 100)
            except Exception:
                pct = 0
            # 显示更直观的文字
            self.progress.setFormat(f"%v / %m ({pct}%)")
            # 当完成时，将进度条颜色改为绿色作为提示
            try:
                if total > 0 and done >= total:
                    self.progress.setStyleSheet(
                        "QProgressBar { border: 1px solid #9f9f9f; border-radius: 0px; background: #f0f0f0; text-align: center; }"
                        "QProgressBar::chunk { background-color: #4CC14C; border-radius: 0px; }"
                    )
                else:
                    self.progress.setStyleSheet(
                        "QProgressBar { border: 1px solid #9f9f9f; border-radius: 0px; background: #f0f0f0; text-align: center; }"
                        "QProgressBar::chunk { background-color: #5aa6ff; border-radius: 0px; }"
                    )
            except Exception:
                pass

        def on_tasks_received(self, vtasks, itasks, nickname):

            # 保存 nickname 到 cfg，更新界面显示
            try:
                cfg['last_nickname'] = nickname or ''
                save_config(cfg)
            except Exception:
                pass
            self.nickname_label.setText(nickname or '')

            # 确保每次接收新任务时都重新计算 desc_display，不复用旧缓存的描述
            self.tree.repaint()

            if not hasattr(self, 'vtasks_all'):
                self.vtasks_all = []
            if not hasattr(self, 'itasks_all'):
                self.itasks_all = []

            self.vtasks_all.extend(vtasks or [])
            self.itasks_all.extend(itasks or [])

            def _strip_date_prefix(desc, date_str):
                if not desc:
                    return ''
                if date_str and desc.startswith(date_str):
                    s = desc[len(date_str):]
                    return s.lstrip(" _-：: ")
                return desc

            idx = self.tree.topLevelItemCount() + 1
            for t in (vtasks or []):
                date_display = t.get('date', '')
                desc_display = _strip_date_prefix(t.get('desc', ''), date_display)
                item = QtWidgets.QTreeWidgetItem(self.tree, [' ', str(idx), date_display, desc_display, 'video'])
                item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
                item.setCheckState(0, Qt.CheckState.Unchecked)
                item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
                item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
                try:
                    brush = QtGui.QBrush(QtGui.QColor('#000000'))
                    for col in range(self.tree.columnCount()):
                        item.setForeground(col, brush)
                except Exception:
                    pass
                item.setData(0, Qt.ItemDataRole.UserRole, (t, False))
                idx += 1

            for t in (itasks or []):
                date_display = t.get('date', '')
                desc_display = _strip_date_prefix(t.get('desc', ''), date_display)
                item = QtWidgets.QTreeWidgetItem(self.tree, [' ', str(idx), date_display, desc_display, 'image'])
                item.setFlags(item.flags() | Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsSelectable | Qt.ItemFlag.ItemIsEnabled)
                item.setCheckState(0, Qt.CheckState.Unchecked)
                item.setTextAlignment(0, int(Qt.AlignmentFlag.AlignLeft))
                item.setTextAlignment(4, int(Qt.AlignmentFlag.AlignRight))
                try:
                    brush = QtGui.QBrush(QtGui.QColor('#000000'))
                    for col in range(self.tree.columnCount()):
                        item.setForeground(col, brush)
                except Exception:
                    pass
                item.setData(0, Qt.ItemDataRole.UserRole, (t, True))
                idx += 1

            # 保持兼容：更新 self.vtasks / self.itasks 为累积列表，供下载使用
            try:
                self.vtasks = list(self.vtasks_all)
                self.itasks = list(self.itasks_all)
            except Exception:
                pass

            # 确保 '类型' 列保持为 8 个字符宽度的固定列（与初始化一致）
            try:
                fm = self.tree.fontMetrics()
                type_w = fm.horizontalAdvance('m' * 8) + 12
                self.tree.setColumnWidth(4, type_w)
                self.tree.header().setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
            except Exception:
                pass

            try:
                self.tree.show()
                self.tree.repaint()
            except Exception:
                pass

        def showEvent(self, event):
            # 在窗口显示后强制设置“类型”列为固定像素宽，防止布局覆盖
            try:
                header = self.tree.header()
                fm = self.tree.fontMetrics()
                target_px = fm.horizontalAdvance('m' * 8) + 12
                header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
                header.resizeSection(4, int(target_px))
            except Exception:
                pass
            return super().showEvent(event)

        def resizeEvent(self, event):
            # 窗口调整大小时重复强制类型列宽，确保布局变化不会覆盖我们的设置
            try:
                header = self.tree.header()
                fm = self.tree.fontMetrics()
                target_px = fm.horizontalAdvance('m' * 8) + 12
                header.setSectionResizeMode(4, QtWidgets.QHeaderView.ResizeMode.Fixed)
                header.resizeSection(4, int(target_px))
            except Exception:
                pass
            return super().resizeEvent(event)

        def on_browse(self):
            dlg = QtWidgets.QFileDialog(self)
            p = dlg.getExistingDirectory(self, '选择目录', self.path_edit.text() or os.getcwd())
            if p:
                self.path_edit.setText(p)

        def on_fetch(self):
            url = self.url_edit.text().strip()
            if not url:
                QtWidgets.QMessageBox.warning(self, '提示', '请输入主页链接')
                return
            cookie = self.cookie_edit.toPlainText().strip() or cfg.get('cookie', '')
            # 保存 cookie/path
            cfg['cookie'] = cookie
            cfg['path'] = self.path_edit.text().strip()
            save_config(cfg)
            # 启动后台获取
            self.fetch_btn.setEnabled(False)
            self.thread = threading.Thread(target=self.worker.fetch_tasks, args=(url, cookie), daemon=True)
            self.thread.start()

        def on_show_queue(self):
            # 切换到队列视图（默认），在主列表中显示队列（已完成项会被过滤）
            self.refresh_tree('queue')

        def on_pause_toggle(self):
            # 切换暂停标志：当被请求暂停时，阻止 Worker 提交新的任务；已提交的任务仍会继续
            self.worker._pause_requested = not getattr(self.worker, '_pause_requested', False)
            if self.worker._pause_requested:
                self.pause_btn.setText('继续')
                self.append_log('[信息] 已请求暂停：不会提交新的下载任务')
            else:
                self.pause_btn.setText('暂停')
                self.append_log('[信息] 已请求继续：将提交新的下载任务')

        def on_show_failed(self):
            # 切换到失败视图
            self.refresh_tree('failed')

        def on_show_completed(self):
            # 切换到已完成视图，在主列表中显示
            self.refresh_tree('completed')

        def closeEvent(self, event):
            # 关闭窗口时询问确认
            res = QtWidgets.QMessageBox.question(self, '退出确认', '确定要关闭程序吗？', QtWidgets.QMessageBox.StandardButton.Yes | QtWidgets.QMessageBox.StandardButton.No)
            if res == QtWidgets.QMessageBox.StandardButton.Yes:
                event.accept()
            else:
                event.ignore()

        def on_download(self):
            # 收集选中条目
            selected = []
            for i in range(self.tree.topLevelItemCount()):
                it = self.tree.topLevelItem(i)
                if it.checkState(0) == Qt.CheckState.Checked:
                    data = it.data(0, Qt.ItemDataRole.UserRole)
                    if data:
                        selected.append(data)
            if not selected:
                # 未选择则询问是否全部
                if QtWidgets.QMessageBox.question(self, '确认', '未选择任何作品，是否下载全部？') != QtWidgets.QMessageBox.StandardButton.Yes:
                    return
                sel_v = self.vtasks
                sel_i = self.itasks
            else:
                sel_v = [d[0] for d in selected if not d[1]]
                sel_i = [d[0] for d in selected if d[1]]
            base_folder = self.path_edit.text().strip() or os.getcwd()
            nickname_for_folder = self.nickname_label.text() or cfg.get('last_nickname') or 'Douyin_User'
            user_folder = os.path.join(base_folder, sanitize_filename(nickname_for_folder) or 'Douyin_Downloads')
            if not safe_mkdir(user_folder):
                QtWidgets.QMessageBox.critical(self, '错误', f'创建目录失败: {user_folder}')
                return
            threads = self.threads_spin.value()
            self.download_btn.setEnabled(False)
            self.fetch_btn.setEnabled(False)
            self.thread = threading.Thread(target=self.worker.download_tasks, args=(sel_v, sel_i, user_folder, threads), daemon=True)
            self.thread.start()

        def on_view_log(self):
            if self.log_view.isHidden():
                self.log_view.show()
            else:
                self.log_view.hide()

        def on_select_all(self):
            for i in range(self.tree.topLevelItemCount()):
                it = self.tree.topLevelItem(i)
                it.setCheckState(0, Qt.CheckState.Checked)
            try:
                self.update_status_label()
            except Exception:
                pass

        def on_invert(self):
            for i in range(self.tree.topLevelItemCount()):
                it = self.tree.topLevelItem(i)
                it.setCheckState(0, Qt.CheckState.Unchecked if it.checkState(0) == Qt.CheckState.Checked else Qt.CheckState.Checked)
            try:
                self.update_status_label()
            except Exception:
                pass

        def on_clear_list(self):
            # 确认后清空当前列表及内存任务缓存
            if QtWidgets.QMessageBox.question(self, '确认', '确定要清空当前列表吗？此操作不会删除已下载的文件。',
                                              QtWidgets.QMessageBox.StandardButton.Yes | QtWidgets.QMessageBox.StandardButton.No) != QtWidgets.QMessageBox.StandardButton.Yes:
                return
            try:
                # 清空 UI 列表
                self.tree.clear()
                # 清空内存中的任务集合（用于下载）
                self.vtasks_all = []
                self.itasks_all = []
                self.vtasks = []
                self.itasks = []
            except Exception:
                pass
            # 更新状态与日志
            try:
                self.append_log('[信息] 已清空当前列表')
            except Exception:
                try:
                    self.status.setText('[信息] 已清空当前列表')
                except Exception:
                    pass

        def on_worker_finished(self):
            self.download_btn.setEnabled(True)
            self.fetch_btn.setEnabled(True)

    app = QtWidgets.QApplication(sys.argv)
    with tempfile.NamedTemporaryFile(suffix='.ico', delete=False) as tmp:
        tmp.write(ICON_BYTES)
        tmp_icon_path = tmp.name

    app_icon = QtGui.QIcon(tmp_icon_path)
    app.setWindowIcon(app_icon)
    try:
        app.setStyleSheet("""
        QPushButton {
            background-color: #409EFF;
            border: 1px solid #409EFF;
            color: white;
            padding: 6px 14px;
            border-radius: 0px;
            font-weight: 500;
            font-size: 13px;
        }
        QPushButton:hover {
            background-color: #66b1ff;
            border: 1px solid #66b1ff;
        }
        QPushButton:pressed {
            background-color: #3a8ee6;
            border: 1px solid #3a8ee6;
        }
        QPushButton:disabled {
            background-color: #a0cfff;
            border: 1px solid #a0cfff;
            color: #f0f0f0;
        }
        QLineEdit, QTextEdit, QSpinBox {
            border: 1px solid #dcdfe6;
            background: #ffffff;
            color: #303133;
            padding: 6px;
            border-radius: 0px;
            font-size: 13px;
        }
        QLineEdit:focus, QTextEdit:focus, QSpinBox:focus {
            border: 1px solid #409EFF;
            background: #f9fcff;
        }
        QTreeView::indicator,
        QTreeWidget::indicator {
            width: 16px;
            height: 16px;
            border: 1px solid #c0c4cc;
            border-radius: 2px;
            background: #ffffff;
        }
        QTreeView::indicator:hover,
        QTreeWidget::indicator:hover {
            border: 1px solid #409EFF;
        }
        QTreeView::indicator:checked,
        QTreeWidget::indicator:checked {
            background-color: #409EFF;
            border: 1px solid #409EFF;
            image: url("data:image/svg+xml;utf8,\
                <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'>\
                <path fill='white' d='M6.5 10.5l-2.5-2.5 1-1 1.5 1.5 4-4 1 1z'/>\
                </svg>");
        }
        QTreeWidget {
            background: #ffffff;
            border: 1px solid #e4e7ed;
            alternate-background-color: #fafbfc;
            gridline-color: #f2f6fc;
            selection-background-color: #d9eaff;
            font-size: 13px;
        }
        QTreeWidget::item {
            padding: 6px 4px;
            color: #222222;
        }
        QTreeWidget::item:hover {
            background: #f3f8fe;
        }
        QTreeWidget::item:selected {
            background: #cfe4ff;
            color: #000000;
        }
        QProgressBar {
            border: 1px solid #dcdfe6;
            background: #f5f7fa;
            height: 22px;
            border-radius: 0px;
            text-align: center;
            font-size: 12px;
            color: #303133;
        }
        QProgressBar::chunk {
            background-color: qlineargradient(x1:0, y1:0, x2:1, y2:0,
                stop:0 #66b1ff, stop:1 #409EFF);
            border-radius: 0px;
        }
        QScrollBar:vertical {
            border: none;
            background: #f5f7fa;
            width: 10px;
            margin: 0px;
        }
        QScrollBar::handle:vertical {
            background: #c0c4cc;
            border-radius: 0px;
            min-height: 20px;
        }
        QScrollBar::handle:vertical:hover {
            background: #a6a9ad;
        }
        QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
            height: 0px;
            background: none;
        }
        QScrollBar:horizontal {
            border: none;
            background: #f5f7fa;
            height: 10px;
            margin: 0px;
        }
        QScrollBar::handle:horizontal {
            background: #c0c4cc;
            border-radius: 0px;
            min-width: 20px;
        }
        QScrollBar::handle:horizontal:hover {
            background: #a6a9ad;
        }
        QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal {
            width: 0px;
            background: none;
        }
        """)



    except Exception:
        pass
    w = MainWindow()
    w.show()
    app.exec()


if __name__ == "__main__":
    # 启动策略：
    # - 没有额外参数时（默认），启动 GUI
    # - '--gui' 强制启动 GUI
    # - '--cli' / '--no-gui' / '--nogui' 强制进入命令行模式
    args = set(sys.argv[1:])
    force_cli_keys = {'--cli', '--no-gui', '--nogui'}
    if '--gui' in args or len(args) == 0:
        run_gui()
    elif args & force_cli_keys:
        while True:
            try:
                main()
            except KeyboardInterrupt:
                print("\n[信息] 用户中断。")
            choice = input("\n[输入] 是否继续运行 [Y/N] (回车默认Y): ").strip().lower()
            if choice not in ("", "y", "yes"):
                print("\n[信息] 程序已退出。")
                break
            # 清屏
            if os.name == 'nt':
                os.system('cls')
            else:
                os.system('clear')
    else:
        # 兼容：如果传入未知参数但不是强制 CLI，仍尝试启动 GUI
        run_gui()
