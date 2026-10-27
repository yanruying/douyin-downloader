use std::path::{Path, PathBuf};

/// Windows 文件名非法字符集合：`\\ / * ? : " < > | #`。
const ILLEGAL: &[char] = &['\\', '/', '*', '?', ':', '"', '<', '>', '|', '#'];

fn unquote(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

/// 清理文件名，移除非法字符（对应 file_utils.sanitize_filename）。
/// 超长时保留首尾、中间以 "..." 代替。
pub fn sanitize_filename(name: &str, max_length: usize) -> String {
    let mut s = if name.is_empty() {
        "unknown".to_string()
    } else {
        name.to_string()
    };
    s = unquote(&s);
    s = s
        .chars()
        .map(|c| if ILLEGAL.contains(&c) { '_' } else { c })
        .collect();
    // \s+ -> 单个空格
    let mut squeezed = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                squeezed.push(' ');
            }
            prev_ws = true;
        } else {
            squeezed.push(c);
            prev_ws = false;
        }
    }
    let mut s = squeezed.trim().to_string();

    // 按字符计数，避免中文字节切坏
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_length {
        let head = max_length / 2 - 2;
        let tail = max_length / 2 - 1;
        let p: String = chars[..head].iter().collect();
        let q: String = chars[chars.len() - tail..].iter().collect();
        s = format!("{p}...{q}");
    }
    s
}

/// 从 URL 路径提取扩展名（对应 get_extension_from_url）。
pub fn extension_from_url(url: &str, default_ext: &str) -> String {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return default_ext.to_string(),
    };
    let path = parsed.path();
    match Path::new(path).extension() {
        Some(e) => {
            let e = e.to_string_lossy().to_string();
            if !e.is_empty() && e.len() > 1 && e.len() <= 6 {
                format!(".{e}")
            } else {
                default_ext.to_string()
            }
        }
        None => default_ext.to_string(),
    }
}

/// 预期落盘相对路径（对应 build_expected_filename），用于“已存在则跳过”。
pub fn build_expected_filename(
    desc: &str,
    ext: &str,
    is_image: bool,
    mix_name: Option<&str>,
    date_str: &str,
    include_date: bool,
) -> PathBuf {
    let base = if include_date && !date_str.is_empty() {
        format!("{date_str}_{desc}")
    } else {
        desc.to_string()
    };
    let clean = sanitize_filename(&base, usize::MAX);
    let filename = format!("{}{}", truncate_for_fs(&clean, ext), ext);
    let mut p = PathBuf::new();
    if let Some(mix) = mix_name {
        if !mix.is_empty() {
            p.push(sanitize_filename(mix, 100));
        }
    }
    if is_image {
        p.push("images");
    }
    p.push(filename);
    p
}

/// 生成不覆盖的唯一路径（对应 generate_unique_filename）。
pub fn unique_path(folder: &Path, base: &str, ext: &str, url_hash: &str) -> PathBuf {
    let base_clean = truncate_for_fs(&sanitize_filename(base, usize::MAX), ext);
    let mut path = folder.join(format!("{base_clean}{ext}"));

    let too_long = path.to_string_lossy().chars().count() > 240;
    if too_long || path.exists() {
        let ts = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let head: String = base_clean.chars().take(80).collect();
        path = folder.join(format!("{head}_{ts}_{url_hash}{ext}"));
    }

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let mut counter = 1;
    while path.exists() {
        path = folder.join(format!("{stem}_{counter}{ext}"));
        counter += 1;
        if counter > 200 {
            let ts = chrono::Local::now().timestamp();
            path = folder.join(format!("file_{ts}_{url_hash}{ext}"));
            break;
        }
    }
    path
}

/// Windows 单个文件名上限 255 个 UTF-16 码元，留出扩展名余量。
const MAX_NAME_BYTES: usize = 240;

/// 按 Windows 实际限制截断文件名主干（**字节**而非字符：
/// 中文在 UTF-16 里占 2 个码元，按字符算会超出上限导致创建失败）。
pub fn truncate_for_fs(name: &str, ext: &str) -> String {
    let ext_len = ext.len();
    let budget = MAX_NAME_BYTES.saturating_sub(ext_len);
    if name.len() <= budget {
        return name.to_string();
    }
    // 按字符边界回退，避免切坏 UTF-8
    let mut cut = budget;
    while cut > 0 && !name.is_char_boundary(cut) {
        cut -= 1;
    }
    let head = &name[..cut];
    // 保留尾部 8 个字符的区分度（很多文案前 50 字相同）
    let tail: String = name.chars().rev().take(12).collect::<Vec<_>>()
        .into_iter().rev().collect();
    let head_chars = budget.saturating_sub(tail.len() + 3);
    let mut hc = head_chars;
    while hc > 0 && !head.is_char_boundary(hc) {
        hc -= 1;
    }
    format!("{}...{}", &head[..hc], tail)
}

pub async fn ensure_dir(p: &Path) -> std::io::Result<()> {
    tokio::fs::create_dir_all(p).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_matches_python_rules() {
        assert_eq!(sanitize_filename("a/b:c*d", 150), "a_b_c_d");
        assert_eq!(sanitize_filename("  多  空格  ", 150), "多 空格");
        assert_eq!(sanitize_filename("", 150), "unknown");
        assert_eq!(sanitize_filename("a%20b", 150), "a b");
    }

    #[test]
    fn sanitize_truncates_with_ellipsis() {
        let s = "汉".repeat(200);
        let out = sanitize_filename(&s, 60);
        assert!(out.contains("..."));
        assert_eq!(out.chars().count(), 30 - 2 + 3 + 29);
    }

    #[test]
    fn ext_from_url() {
        assert_eq!(
            extension_from_url("https://x/a/b.mp4?x=1", ".mp4"),
            ".mp4"
        );
        assert_eq!(extension_from_url("https://x/a/", ".jpg"), ".jpg");
        assert_eq!(extension_from_url("not a url", ".mp4"), ".mp4");
    }

    #[test]
    fn expected_filename_layout() {
        let p = build_expected_filename("标题", ".mp4", false, Some("合集A"), "2025-01-02", true);
        assert_eq!(p.to_string_lossy().replace('\\', "/"), "合集A/2025-01-02_标题.mp4");
        let p2 = build_expected_filename("标题", ".jpg", true, None, "", true);
        assert_eq!(p2.to_string_lossy().replace('\\', "/"), "images/标题.jpg");
    }
}
