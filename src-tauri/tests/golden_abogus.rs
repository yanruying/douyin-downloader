//! 与 Python 参考实现（`scripts/gen_golden.py`）逐字节对拍的端到端验证。
//!
//! 核心断言（`sorted_values` / `transformed` / `ab_dir` 逐项比对）放在
//! `src/douyin/sign/abogus.rs` 的 `golden_matches_python` 单元测试里——
//! 那里能访问私有中间量。本文件只做公开 API 的回归防线。

use douyin_downloader_lib::douyin::sign::abogus::ABogus;
use serde_json::Value;

fn golden() -> Value {
    serde_json::from_str(include_str!("golden_abogus.json")).expect("golden json 解析失败")
}

#[test]
fn public_api_reproduces_golden_signature() {
    let g = golden();
    let params = g["inputs"]["params"].as_str().unwrap();
    let ua = g["inputs"]["user_agent"].as_str().unwrap();
    let fp = g["inputs"]["browser_fp"].as_str().unwrap();
    let ms = g["inputs"]["fixed_time_ms"].as_u64().unwrap();
    let rd = (g["inputs"]["fixed_random"].as_f64().unwrap() * 10000.0) as u32;

    let mut ab = ABogus::new(Some(ua.to_string()), Some(fp.to_string()));
    let (full, sig) = ab.generate_deterministic(params, "", ms, rd);

    assert_eq!(sig, g["abogus"].as_str().unwrap());
    assert_eq!(full, g["full"].as_str().unwrap());
}

#[test]
fn state_accumulation_is_reproducible_after_reset() {
    // Python 的 big_array 是实例属性、跨调用累积，Rust 必须一致；
    // reset_state 后应能重新复现 golden。
    let g = golden();
    let params = g["inputs"]["params"].as_str().unwrap();
    let ua = g["inputs"]["user_agent"].as_str().unwrap();
    let fp = g["inputs"]["browser_fp"].as_str().unwrap();
    let ms = g["inputs"]["fixed_time_ms"].as_u64().unwrap();
    let rd = (g["inputs"]["fixed_random"].as_f64().unwrap() * 10000.0) as u32;

    let mut ab = ABogus::new(Some(ua.to_string()), Some(fp.to_string()));
    let first = ab.generate_deterministic(params, "", ms, rd).1;
    let second = ab.generate_deterministic(params, "", ms, rd).1;
    assert_eq!(first, g["abogus"].as_str().unwrap());
    assert_ne!(second, first, "置换状态累积应导致第二次不同");

    ab.reset_state();
    let third = ab.generate_deterministic(params, "", ms, rd).1;
    assert_eq!(third, g["abogus"].as_str().unwrap(), "reset 后应复现");
}

#[test]
fn golden_file_is_self_consistent() {
    let g = golden();
    let sorted: Vec<u64> = g["sorted_values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let transformed: Vec<u64> = g["transformed_codepoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();

    // 44 (sort_index) + 69 (fp 码点) + 1 (ab_xor)
    assert_eq!(sorted.len(), 114);
    assert_eq!(transformed.len(), sorted.len());
    assert_eq!(g["random_bytes_codepoints"].as_array().unwrap().len(), 12);

    // 这两条是「必须用 u32 而不是 u8」的证据
    assert!(sorted.iter().any(|&v| v > 255), "sorted 含 >255");
    assert!(transformed.iter().any(|&v| v > 255), "transformed 含 >255");
    assert!(sorted.contains(&395), "ab_dir[24] (=395) 应出现");
}
