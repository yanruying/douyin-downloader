#!/usr/bin/env python3
"""生成 tests/golden_abogus.json，用于 Rust 侧 a_bogus 实现的逐字节对拍。

参考实现来自 f2（Johnserf-Seed/f2，Apache-2.0）。脚本把时间与随机数固定，
从而让 44+1 字节的中间数组、随机填充与最终签名都成为确定值，Rust 侧注入
同样的 `fixed_time_ms` / `fixed_random` 即可复现。

用法：
    pip install f2 gmssl
    python scripts/gen_golden.py src-tauri/tests/golden_abogus.json

参数里的 sec_user_id 会先被替换成占位符再参与签名计算 —— 只替换文本而不
重算会得到一份自相矛盾的 golden（sec_user_id 参与 SM3 摘要，会一路影响
ab_dir[38..43] -> sorted_values -> transformed -> abogus）。
"""

import json
import random
import re
import sys
import time

USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0"
)
BROWSER_FP = "1920|1080|1950|1170|0|0|0|0|1920|1080|1920|1080|1920|1080|24|24|Win32"
PLACEHOLDER = "MS4wLjABAAAAsample_user_sec_uid_replace_with_yours"
FIXED_MS = 1700000000.123
FIXED_RANDOM = 0.4242

PARAMS = (
    "device_platform=webapp&aid=6383&channel=channel_pc_web&"
    f"sec_user_id={PLACEHOLDER}&max_cursor=0&locate_query=false&"
    "show_live_replay_strategy=1&need_time_list=1&time_list_query=0&"
    "whale_cut_token=&cut_version=1&count=18&publish_video_strategy_type=2&"
    "from_user_page=1&update_version_code=170400&pc_client_type=1&"
    "pc_libra_divert=Windows&support_h265=1&support_dash=0&version_code=290100&"
    "version_name=29.1.0&cookie_enabled=true&screen_width=1920&screen_height=1080&"
    "browser_language=zh-CN&browser_platform=Win32&browser_name=Edge&"
    "browser_version=131.0.0.0&browser_online=true&engine_name=Blink&"
    "engine_version=131.0.0.0&os_name=Windows&os_version=10&cpu_core_num=12&"
    "device_memory=8&platform=PC&downlink=10&effective_type=4g&round_trip_time=50"
)


def load_reference():
    """优先用 f2 包；退回本地克隆的 core.abogus（旧版工程目录结构）。"""
    try:
        from f2.utils._abogus import ABogus  # type: ignore
        from f2.utils._abogus import CryptoUtility, StringProcessor  # type: ignore

        return ABogus, CryptoUtility, StringProcessor
    except ImportError:
        pass
    try:
        from core.abogus import ABogus, CryptoUtility, StringProcessor  # type: ignore

        return ABogus, CryptoUtility, StringProcessor
    except ImportError:
        sys.exit("未找到参考实现：pip install f2，或把 core/abogus.py 所在的目录加入 PYTHONPATH")


def main() -> None:
    out_path = sys.argv[1] if len(sys.argv) > 1 else "src-tauri/tests/golden_abogus.json"
    ABogus, CryptoUtility, StringProcessor = load_reference()

    # 固定时间与随机数，使输出可复现
    time.time = lambda: FIXED_MS
    random.random = lambda: FIXED_RANDOM

    ab = ABogus(fp=BROWSER_FP, user_agent=USER_AGENT)

    captured = {}
    orig_transform = CryptoUtility.transform_bytes
    orig_randbytes = StringProcessor.generate_random_bytes
    orig_params_to_array = CryptoUtility.params_to_array

    def spy_transform(self, bytes_list):
        captured["sorted_values"] = list(bytes_list)
        out = orig_transform(self, bytes_list)
        captured["transformed_codepoints"] = [ord(c) for c in out]
        return out

    def spy_randbytes(length=3):
        s = orig_randbytes(length)
        captured["random_bytes_codepoints"] = [ord(c) for c in s]
        return s

    def spy_params_to_array(self, param, add_salt=True):
        out = orig_params_to_array(self, param, add_salt)
        captured.setdefault("hash_calls", []).append(list(out))
        return out

    CryptoUtility.transform_bytes = spy_transform
    StringProcessor.generate_random_bytes = staticmethod(spy_randbytes)
    CryptoUtility.params_to_array = spy_params_to_array

    full, sig, _ua, _body = ab.generate_abogus(params=PARAMS, body="")

    calls = captured["hash_calls"]
    if len(calls) != 5:
        sys.exit(f"params_to_array 调用次数异常: {len(calls)}")
    array1, array2, array3 = calls[1], calls[3], calls[4]

    # 复算 ab_dir（与 generate_abogus 内部构造逐项一致）
    start = int(FIXED_MS * 1000)
    end = start  # 时间被冻结，开始与结束相同
    opts, fp = ab.options, ab.browser_fp
    d = {
        8: 3,
        18: 44,
        66: 0,
        69: 0,
        70: 0,
        71: 0,
        20: (start >> 24) & 255,
        21: (start >> 16) & 255,
        22: (start >> 8) & 255,
        23: start & 255,
        24: int(start / 256 / 256 / 256 / 256) >> 0,
        25: int(start / 256 / 256 / 256 / 256 / 256) >> 0,
        26: (opts[0] >> 24) & 255,
        27: (opts[0] >> 16) & 255,
        28: (opts[0] >> 8) & 255,
        29: opts[0] & 255,
        30: int(opts[1] / 256) & 255,
        31: opts[1] % 256,
        32: (opts[1] >> 24) & 255,
        33: (opts[1] >> 16) & 255,
        34: (opts[2] >> 24) & 255,
        35: (opts[2] >> 16) & 255,
        36: (opts[2] >> 8) & 255,
        37: opts[2] & 255,
        38: array1[21],
        39: array1[22],
        40: array2[21],
        41: array2[22],
        42: array3[23],
        43: array3[24],
        44: (end >> 24) & 255,
        45: (end >> 16) & 255,
        46: (end >> 8) & 255,
        47: end & 255,
        48: 3,
        49: int(end / 256 / 256 / 256 / 256) >> 0,
        50: int(end / 256 / 256 / 256 / 256 / 256) >> 0,
        51: (ab.pageId >> 24) & 255,
        52: (ab.pageId >> 16) & 255,
        53: (ab.pageId >> 8) & 255,
        54: ab.pageId & 255,
        55: ab.pageId,
        56: ab.aid,
        57: ab.aid & 255,
        58: (ab.aid >> 8) & 255,
        59: (ab.aid >> 16) & 255,
        60: (ab.aid >> 24) & 255,
        64: len(fp),
        65: len(fp),
    }

    # 自检：复算的 sorted_values 必须与实际传给 transform_bytes 的一致
    fp_chars = StringProcessor.to_char_array(fp)
    ab_xor = (len(fp) & 255) >> 8 & 255
    for i in range(len(ab.sort_index_2) - 1):
        if i == 0:
            ab_xor = d.get(ab.sort_index_2[i], 0)
        ab_xor ^= d.get(ab.sort_index_2[i + 1], 0)
    recomputed = [d.get(i, 0) for i in ab.sort_index] + fp_chars + [ab_xor]
    if recomputed != captured["sorted_values"]:
        sys.exit("ab_dir 复算与算法实际使用的不一致，参考实现版本可能已变更")

    out = {
        "inputs": {
            "params": PARAMS,
            "body": "",
            "user_agent": USER_AGENT,
            "browser_fp": BROWSER_FP,
            "fixed_time_ms": int(start),
            "fixed_random": FIXED_RANDOM,
        },
        "params_array": array1,
        "body_array": array2,
        "ua_array": array3,
        "ab_dir_int": {str(k): v for k, v in d.items()},
        "sorted_values": captured["sorted_values"],
        "random_bytes_codepoints": captured["random_bytes_codepoints"],
        "transformed_codepoints": captured["transformed_codepoints"],
        "abogus": sig,
        "full": full,
    }
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)

    print(f"已写入 {out_path}")
    print(f"sorted_values {len(out['sorted_values'])} 项，随机填充 {len(out['random_bytes_codepoints'])} 字节")
    print("含 >255 的值:", any(v > 255 for v in out["sorted_values"]), "含 395:", 395 in out["sorted_values"])
    print("abogus =", sig)


if __name__ == "__main__":
    main()
