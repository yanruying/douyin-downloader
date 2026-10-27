//! a_bogus 签名算法（1.0.1.19）。
//!
//! 算法源自 Johnserf-Seed/f2（Apache-2.0），包含 SM3、自定义 base64 字母表与 RC4 编排。
//!
//! 实现中有三处细节必须与参考实现保持一致，否则签名为空：
//! 1. 中间量按 **码点**（`chr`/`ord`）流转，会超过 255。`ab_dir[24]`、`ab_dir[56]`（=aid 6383）、
//!    `ab_xor` 都是 >255 的值，用 `u8` 会截断并改变结果 → 全程用 `u32`。
//! 2. `transform_bytes` 里的 `big_array` 是**实例属性**，跨调用累积置换状态 → 用 `&mut self` 保留。
//! 3. 算法含随机填充与毫秒时间戳，同一 `params` 两次输出必然不同，测试只能断言结构不能断言字节。

use rand::Rng;
use sm3::{Digest, Sm3};

pub const UA_DEFAULT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0";

const CHARACTER: &str = "Dkdpgh2ZmsQB80/MfvV36XI1R45-WUAlEixNLwoqYTOPuzKFjJnry79HbGcaStCe";
const CHARACTER2: &str = "ckdp1h4ZKsUB80/Mfvw36XIgR25+WQAlEi7NLboqYTOPuzmFjJnryx9HVGDaStCe";

const SALT: &str = "cus";
const UA_KEY: [u8; 3] = [0x00, 0x01, 0x0E];

const SORT_INDEX: [usize; 44] = [
    18, 20, 52, 26, 30, 34, 58, 38, 40, 53, 42, 21, 27, 54, 55, 31, 35, 57, 39, 41, 43, 22, 28, 32,
    60, 36, 23, 29, 33, 37, 44, 45, 59, 46, 47, 48, 49, 50, 24, 25, 65, 66, 70, 71,
];

const SORT_INDEX_2: [usize; 44] = [
    18, 20, 26, 30, 34, 38, 40, 42, 21, 27, 31, 35, 39, 41, 43, 22, 28, 32, 36, 23, 29, 33, 37, 44,
    45, 46, 47, 48, 49, 50, 24, 25, 52, 53, 54, 55, 57, 58, 59, 60, 65, 66, 70, 71,
];

/// 256 项置换表，RC4 变体的 S 盒初值。
const BIG_ARRAY: [u8; 256] = [
    121, 243, 55, 234, 103, 36, 47, 228, 30, 231, 106, 6, 115, 95, 78, 101, 250, 207, 198, 50, 139,
    227, 220, 105, 97, 143, 34, 28, 194, 215, 18, 100, 159, 160, 43, 8, 169, 217, 180, 120, 247, 45,
    90, 11, 27, 197, 46, 3, 84, 72, 5, 68, 62, 56, 221, 75, 144, 79, 73, 161, 178, 81, 64, 187, 134,
    117, 186, 118, 16, 241, 130, 71, 89, 147, 122, 129, 65, 40, 88, 150, 110, 219, 199, 255, 181,
    254, 48, 4, 195, 248, 208, 32, 116, 167, 69, 201, 17, 124, 125, 104, 96, 83, 80, 127, 236, 108,
    154, 126, 204, 15, 20, 135, 112, 158, 13, 1, 188, 164, 210, 237, 222, 98, 212, 77, 253, 42, 170,
    202, 26, 22, 29, 182, 251, 10, 173, 152, 58, 138, 54, 141, 185, 33, 157, 31, 252, 132, 233, 235,
    102, 196, 191, 223, 240, 148, 39, 123, 92, 82, 128, 109, 57, 24, 38, 113, 209, 245, 2, 119, 153,
    229, 189, 214, 230, 174, 232, 63, 52, 205, 86, 140, 66, 175, 111, 171, 246, 133, 238, 193, 99,
    60, 74, 91, 225, 51, 76, 37, 145, 211, 166, 151, 213, 206, 0, 200, 244, 176, 218, 44, 184, 172,
    49, 216, 93, 168, 53, 21, 183, 41, 67, 85, 224, 155, 226, 242, 87, 177, 146, 70, 190, 12, 162,
    19, 137, 114, 25, 165, 163, 192, 23, 59, 9, 94, 179, 107, 35, 7, 142, 131, 239, 203, 149, 136,
    61, 249, 14, 156,
];

pub fn sm3_bytes(data: &[u8]) -> Vec<u8> {
    let mut h = Sm3::new();
    h.update(data);
    h.finalize().to_vec()
}

fn params_to_array(param: &[u8], add_salt: bool) -> Vec<u8> {
    if add_salt {
        let mut v = param.to_vec();
        v.extend_from_slice(SALT.as_bytes());
        sm3_bytes(&v)
    } else {
        sm3_bytes(param)
    }
}

/// 置换异或：按码点运算，结果可能大于 255，故用 u32。
/// 置换表跨调用累积（通过 `&mut` 传入）。
fn transform_bytes(s: &mut [u8; 256], data: &[u32]) -> Vec<u32> {
    let n = s.len();
    let mut out = Vec::with_capacity(data.len());

    let mut index_b = s[1] as usize;
    let mut initial_value = 0u32;
    let mut value_e = 0u32;

    for (index, &ch) in data.iter().enumerate() {
        let sum_initial;
        if index == 0 {
            initial_value = s[index_b] as u32;
            sum_initial = (index_b as u32 + initial_value) as usize;
            s[1] = initial_value as u8;
            s[index_b] = index_b as u8;
        } else {
            sum_initial = (initial_value + value_e) as usize;
        }

        let sum_initial = sum_initial % n;
        let value_f = s[sum_initial] as u32;
        out.push(ch ^ value_f);

        let idx2 = (index + 2) % n;
        value_e = s[idx2] as u32;
        let sum2 = (index_b + value_e as usize) % n;
        initial_value = s[sum2] as u32;
        s[sum2] = s[idx2];
        s[idx2] = initial_value as u8;
        index_b = sum2;
    }
    out
}

/// `CryptoUtility.base64_encode`：8 位一组重排成 6 位，用自定义字母表。
fn base64_encode_std(input: &[u8], alphabet: &str) -> String {
    let alpha: Vec<char> = alphabet.chars().collect();
    let mut bits: Vec<u8> = Vec::with_capacity(input.len() * 8);
    for b in input {
        for k in (0..8).rev() {
            bits.push((b >> k) & 1);
        }
    }
    let padding = (6 - bits.len() % 6) % 6;
    bits.extend(std::iter::repeat_n(0u8, padding));

    let mut out = String::new();
    for chunk in bits.chunks(6) {
        let mut idx = 0usize;
        for &bit in chunk {
            idx = (idx << 1) | bit as usize;
        }
        out.push(alpha[idx]);
    }
    for _ in 0..(padding / 2) {
        out.push('=');
    }
    out
}

/// `abogus_encode`：3 码点一组、最多 4 字符输出，掩码顺序与标准 base64 不同。
/// 码点可能 > 255，故用 u64 累加避免溢出。
fn abogus_encode(cps: &[u32], alphabet: &str) -> String {
    let alpha: Vec<char> = alphabet.chars().collect();
    let mut out = String::new();
    let len = cps.len();
    let mut i = 0usize;
    while i < len {
        let n: u64 = if i + 2 < len {
            ((cps[i] as u64) << 16) | ((cps[i + 1] as u64) << 8) | cps[i + 2] as u64
        } else if i + 1 < len {
            ((cps[i] as u64) << 16) | ((cps[i + 1] as u64) << 8)
        } else {
            (cps[i] as u64) << 16
        };

        for (j, k) in [(18u32, 0xFC0000u64), (12, 0x03F000), (6, 0x0FC0), (0, 0x3F)] {
            if j == 6 && i + 1 >= len {
                break;
            }
            if j == 0 && i + 2 >= len {
                break;
            }
            let idx = ((n & k) >> j) as usize;
            out.push(alpha[idx]);
        }
        i += 3;
    }
    let pad = (4 - out.chars().count() % 4) % 4;
    for _ in 0..pad {
        out.push('=');
    }
    out
}

/// 标准 RC4。
fn rc4_encrypt(key: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = [0; 256];
    for (i, v) in s.iter_mut().enumerate() {
        *v = i as u8;
    }
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) % 256;
        s.swap(i, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    let mut out = Vec::with_capacity(plaintext.len());
    for &b in plaintext {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        let k = s[(s[i] as usize + s[j] as usize) % 256];
        out.push(b ^ k);
    }
    out
}

/// 随机填充：取一次随机数 `rd`，生成的 4 字节模板重复 3 组，共 12 字节。
/// 注入 `rd` 即可完全复现，这是 deterministic 模式的基础。
fn random_bytes_from_rd(rd: u32) -> Vec<u32> {
    let lo = (rd & 255) as u32;
    let hi = ((rd >> 8) & 255) as u32;
    let quad = [
        (lo & 170) | 1,
        (lo & 85) | 2,
        (hi & 170) | 5,
        (hi & 85) | 40,
    ];
    quad.iter().copied().cycle().take(12).collect()
}

/// 浏览器指纹：`inner|inner|outer|outer|sx|sy|0|0|sw|sh|aw|ah|inner|inner|24|24|platform`
pub fn generate_fingerprint(platform: &str) -> String {
    let mut rng = rand::rng();
    let inner_w: u32 = rng.random_range(1024..=1920);
    let inner_h: u32 = rng.random_range(768..=1080);
    let outer_w = inner_w + rng.random_range(24..=32);
    let outer_h = inner_h + rng.random_range(75..=90);
    let screen_y: u32 = if rng.random_range(0..2) == 0 { 0 } else { 30 };
    let size_w: u32 = rng.random_range(1024..=1920);
    let size_h: u32 = rng.random_range(768..=1080);
    let avail_w: u32 = rng.random_range(1280..=1920);
    let avail_h: u32 = rng.random_range(800..=1080);
    format!(
        "{inner_w}|{inner_h}|{outer_w}|{outer_h}|0|{screen_y}|0|0|{size_w}|{size_h}|{avail_w}|{avail_h}|{inner_w}|{inner_h}|24|24|{platform}"
    )
}

pub struct ABogus {
    pub user_agent: String,
    pub browser_fp: String,
    /// GET 用 [0,1,8]，POST 用 [0,1,14]（14 兼容 8）。本项目全走 GET。
    pub options: [u32; 3],
    /// 置换表实例状态，跨调用累积。
    big_array: [u8; 256],
    /// 测试专用：记录最近一次的中间量，供 golden 对拍。
    #[cfg(test)]
    last_sorted: Vec<u32>,
    #[cfg(test)]
    last_transformed: Vec<u32>,
    #[cfg(test)]
    last_ab: [u32; 80],
}

impl ABogus {
    pub fn new(user_agent: Option<String>, fp: Option<String>) -> Self {
        Self {
            user_agent: user_agent
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| UA_DEFAULT.into()),
            browser_fp: fp
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| generate_fingerprint("Win32")),
            options: [0, 1, 14],
            big_array: BIG_ARRAY,
            #[cfg(test)]
            last_sorted: Vec::new(),
            #[cfg(test)]
            last_transformed: Vec::new(),
            #[cfg(test)]
            last_ab: [0; 80],
        }
    }

    #[cfg(test)]
    pub fn debug_intermediates(&self) -> (Vec<u32>, Vec<u32>, [u32; 80]) {
        (
            self.last_sorted.clone(),
            self.last_transformed.clone(),
            self.last_ab,
        )
    }

    pub fn reset_state(&mut self) {
        self.big_array = BIG_ARRAY;
    }

    /// 返回 `(params&a_bogus, a_bogus)`。时间与随机填充用系统值。
    pub fn generate(&mut self, params: &str, body: &str) -> (String, String) {
        let start_ms = chrono::Local::now().timestamp_millis() as u64;
        let rd: u32 = {
            let mut rng = rand::rng();
            rng.random_range(0..10000)
        };
        // 先完成全部哈希，再取结束时间戳
        // 双重哈希的加盐规则：内层输入为字符串，需要加盐；
        // 外层输入为字节序列，不满足字符串条件，因此不再加盐。
        let a1 = params_to_array(&params_to_array(params.as_bytes(), true), false);
        let a2 = params_to_array(&params_to_array(body.as_bytes(), true), false);
        let ua_rc4 = rc4_encrypt(&UA_KEY, self.user_agent.as_bytes());
        let ua_b64 = base64_encode_std(&ua_rc4, CHARACTER2);
        let a3 = params_to_array(ua_b64.as_bytes(), false);
        let end_ms = chrono::Local::now().timestamp_millis() as u64;
        self.assemble(params, a1, a2, a3, start_ms, end_ms, rd)
    }

    /// 可注入时间与随机源，用于与参考实现逐字节对拍。
    /// `rd` 为一次性抽样的随机数，复用到 3 组填充模板。
    pub fn generate_deterministic(
        &mut self,
        params: &str,
        body: &str,
        now_ms: u64,
        rd: u32,
    ) -> (String, String) {
        // 双重哈希的加盐规则：内层输入为字符串，需要加盐；
        // 外层输入为字节序列，不满足字符串条件，因此不再加盐。
        let a1 = params_to_array(&params_to_array(params.as_bytes(), true), false);
        let a2 = params_to_array(&params_to_array(body.as_bytes(), true), false);
        let ua_rc4 = rc4_encrypt(&UA_KEY, self.user_agent.as_bytes());
        let ua_b64 = base64_encode_std(&ua_rc4, CHARACTER2);
        let a3 = params_to_array(ua_b64.as_bytes(), false);
        self.assemble(params, a1, a2, a3, now_ms, now_ms, rd)
    }

    /// 组装 80 槽数组并编码。start/end 时间与随机数全部由调用方给定，
    /// 保证同一输入必然产出同一签名——这是 golden 对拍的前提。
    fn assemble(
        &mut self,
        params: &str,
        a1: Vec<u8>,
        a2: Vec<u8>,
        a3: Vec<u8>,
        start_encryption: u64,
        end_encryption: u64,
        rd: u32,
    ) -> (String, String) {

        let mut ab = [0u32; 80];
        let set = |ab: &mut [u32; 80], k: usize, v: u64| ab[k] = (v & 0xFFFF_FFFF) as u32;

        set(&mut ab, 8, 3);
        set(&mut ab, 18, 44);
        set(&mut ab, 66, 0);
        set(&mut ab, 69, 0);
        set(&mut ab, 70, 0);
        set(&mut ab, 71, 0);
        // 索引 19 与 15 不在 SORT_INDEX 中，不会被读取，保持为 0。

        // 加密开始时间（未掩码的 5/6 字节位，可能 >255）
        set(&mut ab, 20, (start_encryption >> 24) & 255);
        set(&mut ab, 21, (start_encryption >> 16) & 255);
        set(&mut ab, 22, (start_encryption >> 8) & 255);
        set(&mut ab, 23, start_encryption & 255);
        set(&mut ab, 24, start_encryption / 256 / 256 / 256 / 256);
        set(&mut ab, 25, start_encryption / 256 / 256 / 256 / 256 / 256);

        // options
        let o = self.options;
        set(&mut ab, 26, ((o[0] >> 24) & 255) as u64);
        set(&mut ab, 27, ((o[0] >> 16) & 255) as u64);
        set(&mut ab, 28, ((o[0] >> 8) & 255) as u64);
        set(&mut ab, 29, (o[0] & 255) as u64);
        set(&mut ab, 30, ((o[1] / 256) & 255) as u64);
        set(&mut ab, 31, ((o[1] % 256) & 255) as u64);
        set(&mut ab, 32, ((o[1] >> 24) & 255) as u64);
        set(&mut ab, 33, ((o[1] >> 16) & 255) as u64);
        set(&mut ab, 34, ((o[2] >> 24) & 255) as u64);
        set(&mut ab, 35, ((o[2] >> 16) & 255) as u64);
        set(&mut ab, 36, ((o[2] >> 8) & 255) as u64);
        set(&mut ab, 37, (o[2] & 255) as u64);

        // 参数 / body / UA 摘要片段
        set(&mut ab, 38, a1[21] as u64);
        set(&mut ab, 39, a1[22] as u64);
        set(&mut ab, 40, a2[21] as u64);
        set(&mut ab, 41, a2[22] as u64);
        set(&mut ab, 42, a3[23] as u64);
        set(&mut ab, 43, a3[24] as u64);

        // 开始与结束时间戳各取一次，由调用方注入
        set(&mut ab, 44, (end_encryption >> 24) & 255);
        set(&mut ab, 45, (end_encryption >> 16) & 255);
        set(&mut ab, 46, (end_encryption >> 8) & 255);
        set(&mut ab, 47, end_encryption & 255);
        set(&mut ab, 48, 3);
        set(&mut ab, 49, end_encryption / 256 / 256 / 256 / 256);
        set(&mut ab, 50, end_encryption / 256 / 256 / 256 / 256 / 256);

        // pageId (=0) 与 aid (=6383，未掩码)
        set(&mut ab, 51, 0);
        set(&mut ab, 52, 0);
        set(&mut ab, 53, 0);
        set(&mut ab, 54, 0);
        set(&mut ab, 55, 0);
        set(&mut ab, 56, 6383);
        set(&mut ab, 57, 6383 & 255);
        set(&mut ab, 58, (6383 >> 8) & 255);
        set(&mut ab, 59, (6383 >> 16) & 255);
        set(&mut ab, 60, (6383 >> 24) & 255);

        let fp_cps: Vec<u32> = self.browser_fp.chars().map(|c| c as u32).collect();
        let fp_len = fp_cps.len() as u64;
        set(&mut ab, 64, fp_len);
        set(&mut ab, 65, fp_len);

        let mut values: Vec<u32> = SORT_INDEX.iter().map(|&i| ab[i]).collect();

        let mut ab_xor: u32 = 0;
        for index in 0..SORT_INDEX_2.len() - 1 {
            if index == 0 {
                ab_xor = ab[SORT_INDEX_2[index]];
            }
            ab_xor ^= ab[SORT_INDEX_2[index + 1]];
        }

        values.extend_from_slice(&fp_cps);
        values.push(ab_xor);

        #[cfg(test)]
        {
            self.last_sorted = values.clone();
            self.last_ab = ab;
        }

        let mut raw = random_bytes_from_rd(rd);
        let transformed = transform_bytes(&mut self.big_array, &values);
        #[cfg(test)]
        {
            self.last_transformed = transformed.clone();
        }
        raw.extend_from_slice(&transformed);

        let abogus = abogus_encode(&raw, CHARACTER);
        (format!("{params}&a_bogus={abogus}"), abogus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const PARAMS: &str = "device_platform=webapp&aid=6383&channel=channel_pc_web&sec_user_id=MS4wLjABAAAArDVBosPJF3eIWVEFp0szuJ-e1V_-rK0ieJeWwpE77E8&max_cursor=0&locate_query=false&show_live_replay_strategy=1&need_time_list=1&time_list_query=0&whale_cut_token=&cut_version=1&count=18&publish_video_strategy_type=2&from_user_page=1&update_version_code=170400&pc_client_type=1&pc_libra_divert=Windows&support_h265=1&support_dash=0&version_code=290100&version_name=29.1.0&cookie_enabled=true&screen_width=1920&screen_height=1080&browser_language=zh-CN&browser_platform=Win32&browser_name=Edge&browser_version=131.0.0.0&browser_online=true&engine_name=Blink&engine_version=131.0.0.0&os_name=Windows&os_version=10&cpu_core_num=12&device_memory=8&platform=PC&downlink=10&effective_type=4g&round_trip_time=50";

    fn fixed_fp() -> String {
        "1920|1080|1950|1170|0|0|0|0|1920|1080|1920|1080|1920|1080|24|24|Win32".to_string()
    }

    #[test]
    fn golden_matches_python() {
        let text = include_str!("../../../tests/golden_abogus.json");
        let g: serde_json::Value = serde_json::from_str(text).unwrap();
        let params = g["inputs"]["params"].as_str().unwrap();
        let ua = g["inputs"]["user_agent"].as_str().unwrap();
        let fp = g["inputs"]["browser_fp"].as_str().unwrap();
        let ms = g["inputs"]["fixed_time_ms"].as_u64().unwrap();
        let rd = (g["inputs"]["fixed_random"].as_f64().unwrap() * 10000.0) as u32;

        let mut ab = ABogus::new(Some(ua.to_string()), Some(fp.to_string()));
        let (_, sig) = ab.generate_deterministic(params, "", ms, rd);

        let (sorted, transformed, dir) = ab.debug_intermediates();
        let expect_sorted: Vec<u32> = g["sorted_values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let expect_transformed: Vec<u32> = g["transformed_codepoints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();

        // 逐段定位第一个不一致点，便于排查
        for (i, (a, b)) in sorted.iter().zip(expect_sorted.iter()).enumerate() {
            assert_eq!(a, b, "sorted_values[{i}] 不一致");
        }
        assert_eq!(sorted.len(), expect_sorted.len());

        for (i, (a, b)) in transformed.iter().zip(expect_transformed.iter()).enumerate() {
            assert_eq!(a, b, "transformed[{i}] 不一致");
        }

        // ab_dir 关键槽位
        let expect_dir = &g["ab_dir_int"];
        for key in ["8", "18", "24", "38", "42", "44", "48", "56", "64"] {
            let k: usize = key.parse().unwrap();
            let v = expect_dir[key].as_u64().unwrap() as u32;
            assert_eq!(dir[k], v, "ab_dir[{k}] 不一致");
        }

        assert_eq!(sig, g["abogus"].as_str().unwrap());
    }

    #[test]
    fn abogus_charset_length_and_param_passthrough() {
        let mut ab = ABogus::new(None, Some(fixed_fp()));
        let (full, sig) = ab.generate(PARAMS, "");

        // 长度可精确推算：values = 44 + fp 码点数 + 1(ab_xor)；raw = 12(随机) + values
        let fp_len = fixed_fp().chars().count();
        let raw_len = 12 + 44 + fp_len + 1;
        let full_groups = raw_len / 3;
        let rem = raw_len % 3;
        // 末组按截断规则少输出字符：余 1 字节 → 2 字符，余 2 字节 → 3 字符
        let tail = match rem {
            0 => 0,
            1 => 2,
            _ => 3,
        };
        let chars = full_groups * 4 + tail;
        let pad = (4 - chars % 4) % 4;
        assert_eq!(sig.chars().count(), chars + pad, "sig={sig}");

        let allowed: HashSet<char> = CHARACTER.chars().chain(std::iter::once('=')).collect();
        assert!(sig.chars().all(|c| allowed.contains(&c)), "非法字符: {sig}");
        assert!(full.starts_with(PARAMS));
        assert!(full.ends_with(&format!("&a_bogus={sig}")));
    }

    #[test]
    fn abogus_is_nondeterministic() {
        let mut ab = ABogus::new(None, Some(fixed_fp()));
        let a = ab.generate(PARAMS, "").1;
        std::thread::sleep(std::time::Duration::from_millis(5));
        let b = ab.generate(PARAMS, "").1;
        assert_ne!(a, b, "含随机填充与时间戳，两次不应相同");
    }

    #[test]
    fn abogus_stable_length_across_calls() {
        let mut ab = ABogus::new(None, Some(fixed_fp()));
        let lens: HashSet<usize> = (0..20).map(|_| ab.generate(PARAMS, "").1.len()).collect();
        assert_eq!(lens.len(), 1, "签名长度应恒定: {lens:?}");
    }

    #[test]
    fn sm3_known_vector() {
        // GB/T 32905-2016: SM3("abc")
        let hex: String = sm3_bytes(b"abc").iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
        );
    }

    #[test]
    fn rc4_known_vector() {
        // RC4("Key", "Plaintext") = BBF316E8D940AF0AD3
        let hex: String = rc4_encrypt(b"Key", b"Plaintext")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hex, "bbf316e8d940af0ad3");
    }

    #[test]
    fn base64_custom_alphabet_shape() {
        // 'h' = 0x68 = 0b01101000 -> 前 6 位 011010 = 26 -> CHARACTER2[26]
        let enc = base64_encode_std(b"hello", CHARACTER2);
        assert_eq!(enc.chars().count() % 4, 0);
        let expect_first = CHARACTER2.chars().nth(26).unwrap();
        assert_eq!(enc.chars().next().unwrap(), expect_first);
    }

    #[test]
    fn fingerprint_shape() {
        let fp = generate_fingerprint("Win32");
        assert_eq!(fp.matches('|').count(), 16);
        assert!(fp.ends_with("|24|24|Win32"));
    }
}
