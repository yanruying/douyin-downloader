//! aweme JSON → 扁平 `TaskRow` 的解析层。
//!
//! 媒体提取规则：`images[].video.bit_rate` 存在时视为实况图（只取 mp4），
//! 否则取 `url_list` 末项（最高分辨率静态图）。

use crate::douyin::model::{Aweme, MixInfo, VideoInfo};
use crate::models::{AwemeStat, TaskRow};
use crate::paths::{extension_from_url, sanitize_filename};
use md5::{Digest, Md5};


fn md5_short(s: &str) -> String {
    let mut h = Md5::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())[..8].to_string()
}

/// 从 bit_rate 的 url_list 里挑出「可复用的 play 链接」。
///
/// 抖音的 url_list 一般是三条：前两条是 CDN 直链（会过期、带一堆签名参数），
/// 最后一条形如 `https://www.douyin.com/aweme/v1/play/?video_id=…&file_id=…`，
/// 只有它含 video_id / file_id —— 导出直链必须用这一条。
/// 优先取「第一个含 video_id 的」而非固定下标：该列表长度不固定。
fn best_play_url(v: &VideoInfo) -> Option<String> {
    let best = v.bit_rate.iter().max_by_key(|b| b.bit_rate)?;
    // 优先带 video_id 的
    if let Some(u) = best
        .play_addr
        .url_list
        .iter()
        .find(|u| u.contains("video_id=") && u.contains("file_id="))
    {
        return Some(u.clone());
    }
    // 退而求其次：取最后一条
    best.play_addr.url_list.last().cloned()
}

fn best_bitrate_url(v: &VideoInfo) -> Option<String> {
    if let Some(best) = v.bit_rate.iter().max_by_key(|b| b.bit_rate) {
        if let Some(u) = best.play_addr.url_list.first() {
            return Some(u.clone());
        }
    }
    // 兜底只在确实没有任何码率信息时使用。
    // 注意：图集作品的 video 字段里带的往往是背景音乐（.mp3），
    // 直接取 play_addr 会把音乐当成视频下载。
    None
}

/// 这个 video 字段是不是「真视频」。
///
/// 图集作品的 `video` 常常只装了背景音乐（ext=mp3、duration 很短或为 0），
/// 而真正的实况图在 `images[].video` 里。用它把音乐挡在任务列表之外。
fn is_real_video(v: &VideoInfo, url: &str) -> bool {
    if v.bit_rate.is_empty() {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains(".mp3") || lower.contains(".m4a") || lower.contains(".aac") {
        return false;
    }
    true
}

fn mix_of(a: &Aweme) -> Option<String> {
    if let Some(MixInfo {
        mix_name,
        mix_name_str,
    }) = &a.mix_info
    {
        if let Some(n) = mix_name.as_ref().filter(|s| !s.is_empty()) {
            return Some(n.clone());
        }
        if let Some(n) = mix_name_str.as_ref().filter(|s| !s.is_empty()) {
            return Some(n.clone());
        }
    }
    a.mix_name
        .as_ref()
        .or(a.mix_name_str.as_ref())
        .filter(|s| !s.is_empty())
        .cloned()
}

fn date_of(a: &Aweme) -> String {
    // 时间戳按本地时区格式化
    a.create_time
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
        .map(|d| {
            use chrono::{Datelike, Local};
            let l = d.with_timezone(&Local);
            format!("{:04}-{:02}-{:02}", l.year(), l.month(), l.day())
        })
        .unwrap_or_default()
}

/// 描述不再截断。
///
/// 描述完整保留，不在此处截断：列表以单行省略号展示，落盘时由 `paths`
/// 按 Windows 文件名长度上限（255 字节）自动截断。
fn normalize_desc(desc: &str) -> String {
    desc.trim().to_string()
}

/// 把一条 aweme 拆成 0..n 个 TaskRow。
///
/// `owner` 是作品所属账号：批量获取多个用户时，靠它把作品分到各自目录。
pub fn parse_aweme(value: &serde_json::Value, owner: &Owner) -> Vec<TaskRow> {
    let a: Aweme = match serde_json::from_value(value.clone()) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let aweme_id = a.aweme_id.clone().unwrap_or_default();
    let raw_desc = a
        .desc
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if aweme_id.is_empty() {
                "no_desc".to_string()
            } else {
                aweme_id.clone()
            }
        });
    let desc = normalize_desc(&raw_desc);
    let date = date_of(&a);
    let mix = mix_of(&a);
    let mix_clean = mix.as_ref().map(|m| sanitize_filename(m, 100));

    let mut rows = Vec::new();
    let mut seq = 0usize;
    let make_id = |base: &str, seq: &mut usize| {
        *seq += 1;
        format!("{}-{}", base, seq)
    };

    // 1. 普通视频。只有图集作品才需要判断「是不是音乐」——
    //    图集的 images 非空就说明这是图集，其顶层 video 是背景音乐。
    let is_album = a.images.as_ref().is_some_and(|v| !v.is_empty());
    if let Some(v) = &a.video {
        if let Some(url) = best_bitrate_url(v) {
            // 图集的 video 字段装的是 BGM，不能当视频下载
            let play_url = best_play_url(v).unwrap_or_else(|| url.clone());
            if !is_album || is_real_video(v, &url) {
                let ext = extension_from_url(&url, ".mp4");
                rows.push(TaskRow {
                    id: make_id(&aweme_id, &mut seq),
                    aweme_id: aweme_id.clone(),
                    owner_nickname: owner.nickname.clone(),
                    owner_unique_id: owner.unique_id.clone(),
                    owner_sec_uid: owner.sec_uid.clone(),
                    date: date.clone(),
                    desc: desc.clone(),
                    mix: mix_clean.clone().unwrap_or_default(),
                    kind: "video".into(),
                    url_hash: md5_short(&url),
                    url,
                    play_url,
                    ext,
                    is_image: false,
                });
            }
        }
    }

    // 2. 图集 / 实况图
    if let Some(images) = &a.images {
        let mut idx = 0usize;
        let mut live_idx = 0usize;
        for img in images {
            // 2a. 实况图：有 video.bit_rate
            if let Some(v) = &img.video {
                if !v.bit_rate.is_empty() {
                    if let Some(url) = best_bitrate_url(v) {
                        live_idx += 1;
                        let ext = extension_from_url(&url, ".mp4");
                        rows.push(TaskRow {
                            id: make_id(&aweme_id, &mut seq),
                            aweme_id: aweme_id.clone(),
                            owner_nickname: owner.nickname.clone(),
                            owner_unique_id: owner.unique_id.clone(),
                            owner_sec_uid: owner.sec_uid.clone(),
                            date: date.clone(),
                            desc: format!("{desc}_live{live_idx}"),
                            mix: mix_clean.clone().unwrap_or_default(),
                            kind: "live".into(),
                            url_hash: md5_short(&url),
                            play_url: url.clone(),
                            url,
                            ext,
                            is_image: true,
                        });
                    }
                    continue; // 互斥：有实况就不取静态图
                }
            }
            // 2b. 普通图片：取 url_list 最后一个（最高分辨率）
            if let Some(url) = img.url_list.last() {
                if !url.is_empty() {
                    idx += 1;
                    let ext = extension_from_url(url, ".jpg");
                    rows.push(TaskRow {
                        id: make_id(&aweme_id, &mut seq),
                        aweme_id: aweme_id.clone(),
                        owner_nickname: owner.nickname.clone(),
                        owner_unique_id: owner.unique_id.clone(),
                        owner_sec_uid: owner.sec_uid.clone(),
                        date: date.clone(),
                        desc: format!("{desc}_p{idx}"),
                        mix: mix_clean.clone().unwrap_or_default(),
                        kind: "image".into(),
                        url_hash: md5_short(url),
                        play_url: url.clone(),
                        url: url.clone(),
                        ext,
                        is_image: true,
                    });
                }
            }
        }
    }

    rows
}

/// 从一条 aweme 抽出 Excel 需要的统计字段。
/// 单独成函数：解析任务行与抽统计互不影响，任一失败都不拖累另一个。
pub fn stat_of(value: &serde_json::Value) -> Option<AwemeStat> {
    let a: Aweme = serde_json::from_value(value.clone()).ok()?;
    let st = a.statistics.clone().unwrap_or_default();
    let mix = if let Some(m) = &a.mix_info {
        m.mix_name
            .clone()
            .or(m.mix_name_str.clone())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
    .or_else(|| {
        a.mix_name
            .clone()
            .or(a.mix_name_str.clone())
            .filter(|s| !s.is_empty())
    })
    .unwrap_or_default();

    Some(AwemeStat {
        aweme_id: a.aweme_id.clone().unwrap_or_default(),
        desc: a.desc.clone().unwrap_or_default(),
        create_time: a.create_time.unwrap_or(0),
        mix,
        is_album: a.images.as_ref().is_some_and(|v| !v.is_empty()),
        digg: st.digg_count,
        comment: st.comment_count,
        collect: st.collect_count,
        share: st.share_count,
        recommend: st.recommend_count,
        duration: a.video.as_ref().map(|v| v.duration).unwrap_or(0),
    })
}

/// 批量抽统计。
pub fn stats_all(awemes: &[serde_json::Value]) -> Vec<AwemeStat> {
    awemes.iter().filter_map(stat_of).collect()
}

/// 作品归属账号。批量抓取时每个用户一份。
#[derive(Debug, Clone, Default)]
pub struct Owner {
    pub nickname: String,
    pub unique_id: String,
    pub sec_uid: String,
}

impl Owner {
    pub fn from_profile(p: &crate::models::ProfileInfo) -> Self {
        Self {
            nickname: p.nickname.clone(),
            unique_id: p.unique_id.clone(),
            sec_uid: p.sec_uid.clone(),
        }
    }
}

pub fn parse_all(awemes: &[serde_json::Value], owner: &Owner) -> Vec<TaskRow> {
    let mut out = Vec::new();
    for v in awemes {
        out.extend(parse_aweme(v, owner));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_fixture() -> serde_json::Value {
        serde_json::json!({
            "aweme_id": "7300000000000000001",
            "desc": "测试视频标题",
            "create_time": 1704153600i64,
            "video": {
                "duration": 15000,
                "bit_rate": [
                    {"bit_rate": 500, "play_addr": {"url_list": ["https://cdn/low.mp4"]}},
                    {"bit_rate": 2000, "play_addr": {"url_list": ["https://cdn/high.mp4", "https://cdn/high2.mp4"]}}
                ]
            },
            "mix_info": {"mix_name": "合集/第一季"},
            "statistics": {"digg_count": 10}
        })
    }

    fn album_fixture() -> serde_json::Value {
        serde_json::json!({
            "aweme_id": "7300000000000000002",
            "desc": "图集",
            "create_time": 1704153600i64,
            "images": [
                {"url_list": ["https://cdn/a.jpg", "https://cdn/b.jpg"]},
                {"url_list": ["https://cdn/c.jpg"], "video": {"bit_rate": [{"bit_rate": 900, "play_addr": {"url_list": ["https://cdn/live.mp4"]}}]}}
            ]
        })
    }

    #[test]
    fn video_picks_highest_bitrate_and_first_url() {
        let rows = parse_aweme(&video_fixture(), &Owner::default());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "https://cdn/high.mp4");
        assert_eq!(rows[0].kind, "video");
        assert!(!rows[0].is_image);
        assert_eq!(rows[0].ext, ".mp4");
        assert_eq!(rows[0].mix, "合集_第一季");
        // 日期按本地时区格式化，只断言形状（避免依赖运行机器时区）
        assert_eq!(rows[0].date.len(), 10, "date={}", rows[0].date);
        assert_eq!(rows[0].date.matches('-').count(), 2);
    }

    #[test]
    fn album_with_null_bit_rate_still_parses() {
        // 列表接口返回的图集作品，顶层 video.bit_rate 为 null。
        // 若结构体只写 #[serde(default)]，显式 null 会导致整条作品反序列化失败，
        // 表现为「只有视频没有图集」。
        let v = serde_json::json!({
            "aweme_id": "7694506535757577061",
            "desc": "图集作品",
            "create_time": 1704153600i64,
            "video": { "bit_rate": null, "duration": 0 },
            "images": [
                { "url_list": ["https://cdn/a.jpg", "https://cdn/b.jpg"] },
                { "url_list": ["https://cdn/c.jpg"] }
            ]
        });
        let rows = parse_aweme(&v, &Owner::default());
        assert_eq!(rows.len(), 2, "图集应解析出 2 张图片，实际 {}", rows.len());
        assert!(rows.iter().all(|r| r.kind == "image"));
        assert_eq!(rows[0].url, "https://cdn/b.jpg");
    }

    #[test]
    fn video_with_null_bit_rate_does_not_panic() {
        // 视频作品若 bit_rate 为 null，应解析为空而不是崩溃
        let v = serde_json::json!({
            "aweme_id": "1",
            "desc": "无码率视频",
            "video": { "bit_rate": null, "play_addr": { "url_list": ["https://cdn/v.mp4"] } }
        });
        let rows = parse_aweme(&v, &Owner::default());
        assert!(rows.is_empty(), "无码率信息时不应产出视频任务");
    }

    #[test]
    fn album_splits_static_and_live_with_exclusion() {
        let rows = parse_aweme(&album_fixture(), &Owner::default());
        // 1 张静态（取 url_list 最后一个）+ 1 张实况
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].url, "https://cdn/b.jpg");
        assert_eq!(rows[0].kind, "image");
        assert_eq!(rows[0].desc, "图集_p1");
        assert_eq!(rows[1].url, "https://cdn/live.mp4");
        assert_eq!(rows[1].kind, "live");
        assert_eq!(rows[1].desc, "图集_live1");
    }

    #[test]
    fn malformed_aweme_does_not_panic() {
        assert!(parse_aweme(&serde_json::json!({}), &Owner::default()).is_empty());
        assert!(parse_aweme(&serde_json::json!({"images": "not-an-array"}), &Owner::default()).is_empty());
    }

    #[test]
    fn stat_extracts_interaction_counts() {
        let f = video_fixture();
        let st = stat_of(&f).expect("should extract");
        assert_eq!(st.aweme_id, "7300000000000000001");
        assert_eq!(st.digg, 10);
        assert_eq!(st.mix, "合集/第一季");
        assert!(!st.is_album);
        assert!(st.create_time > 0);
    }

    #[test]
    fn stat_marks_album_and_reads_duration() {
        let st = stat_of(&album_fixture()).expect("should extract");
        assert!(st.is_album, "有 images 应标为图集");
    }

    #[test]
    fn stat_handles_missing_statistics() {
        // statistics 缺失时不能 panic，互动数应为 0
        let v = serde_json::json!({"aweme_id": "1", "desc": "x"});
        let st = stat_of(&v).expect("should extract");
        assert_eq!(st.digg, 0);
        assert_eq!(st.comment, 0);
    }

    #[test]
    fn stats_all_skips_unparseable() {
        let list = vec![video_fixture(), serde_json::json!({"images": "bad"})];
        let stats = stats_all(&list);
        // 畸形条目被跳过，好条目保留
        assert_eq!(stats.len(), 1);
    }

    #[test]
    fn desc_is_not_truncated() {
        // 长文案必须完整保留：截断会让长标题在列表里无法区分
        let mut f = video_fixture();
        let long: String = "字".repeat(200);
        f["desc"] = serde_json::json!(long);
        let rows = parse_aweme(&f, &Owner::default());
        assert_eq!(rows[0].desc.chars().count(), 200);
        assert!(!rows[0].desc.contains("......"));
    }
}
