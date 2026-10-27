//! 导出：视频直链 txt 与作品数据 xlsx。

use std::path::Path;

use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::error::{AppError, AppResult};
use crate::models::TaskRow;
use crate::paths::sanitize_filename;

fn grab_param(url: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=");
    let start = url.find(&pat)? + pat.len();
    let rest = &url[start..];
    let end = rest.find('&').unwrap_or(rest.len());
    let v = &rest[..end];
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// 从长直链里抽出 `video_id` / `file_id` 拼成简短可复用链接。
pub fn simplify_play_url(url: &str) -> Option<String> {
    let vid = grab_param(url, "video_id")?;
    let fid = grab_param(url, "file_id")?;
    Some(format!(
        "https://www.douyin.com/aweme/v1/play/?video_id={vid}&file_id={fid}"
    ))
}

/// 写直链文件。返回落盘路径。
pub fn write_urls(
    folder: &Path,
    base_name: &str,
    rows: &[TaskRow],
    with_desc: bool,
) -> AppResult<String> {
    std::fs::create_dir_all(folder)?;
    let filename = format!("{}.txt", sanitize_filename(base_name, 150));
    let path = folder.join(filename);

    let mut lines = String::new();
    let mut idx = 0usize;
    for r in rows {
        // 仅导出视频作品，图集与实况不计入直链文件
        if r.kind != "video" {
            continue;
        }
        // 用 play_url（含 video_id/file_id 的那条）而不是下载用的 CDN 直链：
        // CDN 直链会过期且带签名参数，复用到别处通常直接失效。
        let source = if r.play_url.is_empty() { &r.url } else { &r.play_url };
        let url = simplify_play_url(source).unwrap_or_else(|| source.clone());
        if with_desc {
            idx += 1;
            lines.push_str(&format!("{idx}.{}\n{}\n\n", r.desc, url));
        } else {
            lines.push_str(&url);
            lines.push('\n');
        }
    }

    std::fs::write(&path, lines)?;
    Ok(path.to_string_lossy().to_string())
}

/// 一行作品数据（Excel）。
pub struct ExcelRow {
    pub kind: String,
    pub publish_time: String,
    pub desc: String,
    pub mix: String,
    pub digg: i64,
    pub comment: i64,
    pub collect: i64,
    pub share: i64,
    pub recommend: i64,
    pub duration: String,
    pub link: String,
}

pub fn duration_text(ms: i64) -> String {
    if ms <= 0 {
        return String::new();
    }
    let total = ms / 1000;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}小时{m}分钟{s}秒")
    } else if m > 0 {
        format!("{m}分钟{s}秒")
    } else {
        format!("{s}秒")
    }
}

pub fn write_xlsx(folder: &Path, base_name: &str, rows: &[ExcelRow]) -> AppResult<String> {
    std::fs::create_dir_all(folder)?;
    let filename = format!("{}.xlsx", sanitize_filename(base_name, 150));
    let path = folder.join(filename);

    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("作品数据").map_err(map_xlsx)?;

    let wrap = Format::new().set_text_wrap();
    let header_fmt = Format::new().set_bold();

    let headers = [
        "类型",
        "发布时间",
        "文案",
        "合集",
        "点赞数",
        "评论数",
        "收藏数",
        "分享数",
        "推荐次数",
        "视频时长",
        "作品链接",
    ];
    for (col, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, col as u16, *h, &header_fmt)
            .map_err(map_xlsx)?;
    }

    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        for (col, v) in [&r.kind, &r.publish_time, &r.desc, &r.mix, &String::new()]
            .iter()
            .enumerate()
        {
            ws.write_string_with_format(row, col as u16, v.as_str(), &wrap)
                .map_err(map_xlsx)?;
        }
        for (offset, n) in [r.digg, r.comment, r.collect, r.share, r.recommend]
            .iter()
            .enumerate()
        {
            ws.write_number_with_format(row, (4 + offset) as u16, *n as f64, &wrap)
                .map_err(map_xlsx)?;
        }
        ws.write_string_with_format(row, 9, &r.duration, &wrap)
            .map_err(map_xlsx)?;
        ws.write_string_with_format(row, 10, &r.link, &wrap)
            .map_err(map_xlsx)?;
    }

    ws.set_column_width(0, 5.0).map_err(map_xlsx)?;
    ws.set_column_width(1, 20.0).map_err(map_xlsx)?;
    ws.set_column_width(2, 80.0).map_err(map_xlsx)?;
    ws.set_column_width(3, 12.0).map_err(map_xlsx)?;
    for c in 4..=8u16 {
        ws.set_column_width(c, 10.0).map_err(map_xlsx)?;
    }
    ws.set_column_width(9, 12.0).map_err(map_xlsx)?;
    ws.set_column_width(10, 50.0).map_err(map_xlsx)?;

    wb.save(&path).map_err(map_xlsx)?;
    Ok(path.to_string_lossy().to_string())
}

fn map_xlsx(e: XlsxError) -> AppError {
    AppError::msg(format!("Excel 写入失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: &str, url: &str, desc: &str) -> TaskRow {
        TaskRow {
            id: "1".into(),
            aweme_id: "a".into(),
            owner_nickname: String::new(),
            owner_unique_id: String::new(),
            owner_sec_uid: String::new(),
date: "2025-01-01".into(),
            desc: desc.into(),
            mix: String::new(),
            kind: kind.into(),
            url: url.into(),
            play_url: String::new(),
            ext: ".mp4".into(),
            is_image: kind != "video",
            url_hash: "abcd1234".into(),
        }
    }

    #[test]
    fn simplify_extracts_ids() {
        let u = "https://v3-web.douyinvod.com/x/video.mp4?video_id=v123&file_id=f456&a=1";
        assert_eq!(
            simplify_play_url(u).unwrap(),
            "https://www.douyin.com/aweme/v1/play/?video_id=v123&file_id=f456"
        );
        assert!(simplify_play_url("https://x/y.mp4").is_none());
    }

    #[test]
    fn export_prefers_play_url_over_cdn() {
        // 下载用 CDN 直链（会过期、无 video_id），导出要用带 video_id 的 play 链接
        let mut r = row("video", "https://v26-web.douyinvod.com/abc/video.mp4?x=1", "标题");
        r.play_url =
            "https://www.douyin.com/aweme/v1/play/?video_id=V123&file_id=F456".into();

        let dir = std::env::temp_dir().join("dy_export_play");
        let _ = std::fs::remove_dir_all(&dir);
        let p = write_urls(&dir, "t", &[r], false).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();

        assert!(
            text.contains("video_id=V123&file_id=F456"),
            "应导出 play 链接: {text}"
        );
        assert!(!text.contains("douyinvod.com"), "不应导出会过期的 CDN 直链");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_falls_back_to_cdn_when_play_url_missing() {
        // play_url 为空（老数据/图片）时退回原 url，不能导出空行
        let r = row("video", "https://v26-web.douyinvod.com/abc/video.mp4", "标题");
        assert!(r.play_url.is_empty());

        let dir = std::env::temp_dir().join("dy_export_fallback");
        let _ = std::fs::remove_dir_all(&dir);
        let p = write_urls(&dir, "t", &[r], false).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("douyinvod.com"), "无 play_url 时应退回 CDN: {text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn urls_file_only_videos() {
        let dir = std::env::temp_dir().join("dy_urls_test");
        let _ = std::fs::remove_dir_all(&dir);
        let rows = vec![
            row("video", "https://x/v.mp4?video_id=v1&file_id=f1", "标题甲"),
            row("image", "https://x/i.jpg", "图集"),
            row("video", "https://x/plain.mp4", "标题乙"),
        ];
        let p = write_urls(&dir, "张三-abc", &rows, false).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("video_id=v1&file_id=f1"));
        assert!(text.contains("https://x/plain.mp4"));
        assert!(!text.contains("i.jpg"), "图集不应出现在直链文件");
        assert_eq!(text.lines().filter(|l| !l.is_empty()).count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn urls_file_with_desc() {
        let dir = std::env::temp_dir().join("dy_urls_test2");
        let _ = std::fs::remove_dir_all(&dir);
        let rows = vec![row("video", "https://x/plain.mp4", "标题乙")];
        let p = write_urls(&dir, "n", &rows, true).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.starts_with("1.标题乙\nhttps://x/plain.mp4"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(duration_text(0), "");
        assert_eq!(duration_text(15_000), "15秒");
        assert_eq!(duration_text(90_000), "1分钟30秒");
        assert_eq!(duration_text(3_661_000), "1小时1分钟1秒");
    }

    #[test]
    fn xlsx_is_written() {
        let dir = std::env::temp_dir().join("dy_xlsx_test");
        let _ = std::fs::remove_dir_all(&dir);
        let rows = vec![ExcelRow {
            kind: "视频".into(),
            publish_time: "2025-01-01 10:00:00".into(),
            desc: "文案".into(),
            mix: "合集".into(),
            digg: 1,
            comment: 2,
            collect: 3,
            share: 4,
            recommend: 5,
            duration: "15秒".into(),
            link: "https://www.douyin.com/video/1".into(),
        }];
        let p = write_xlsx(&dir, "张三", &rows).unwrap();
        assert!(std::fs::metadata(&p).unwrap().len() > 1000);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
