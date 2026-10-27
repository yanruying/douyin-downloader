//! 配置持久化。新格式为应用数据目录下的 `settings.json`（原子写），
//! 并支持从历史版本的 `config.ini` 一次性导入。

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::models::{Settings, UserEntry};

pub const LEGACY_CONFIG: &str = "config.ini";

fn settings_path(app_dir: &Path) -> PathBuf {
    app_dir.join("settings.json")
}

pub fn load(app_dir: &Path) -> Settings {
    let p = settings_path(app_dir);
    match std::fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// 原子写：先写 `.tmp` 再 rename，避免掉电写坏配置。
pub fn save(app_dir: &Path, cfg: &Settings) -> AppResult<()> {
    std::fs::create_dir_all(app_dir)?;
    let p = settings_path(app_dir);
    let tmp = p.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

/// 历史版本 `config.ini` 的一次性导入。返回 None 表示无旧配置。
///
/// 搜索顺序刻意包含「上一个版本的 settings.json 所在目录」，
/// 这样把配置目录从文档迁到程序目录后，老用户升级仍然找得到自己的 Cookie。
pub fn import_legacy(app_dir: &Path, legacy_dir: Option<&Path>) -> Option<Settings> {
    // 先试上版本的 settings.json（迁移），再退回更早的 config.ini
    let prev_json: Option<Settings> = {
        let p = settings_path(&legacy_documents_dir());
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
    };
    if let Some(s) = prev_json {
        if !s.cookie.is_empty() || !s.users.is_empty() {
            return Some(s);
        }
    }

    let candidates: Vec<PathBuf> = [
        legacy_dir.map(|d| d.join(LEGACY_CONFIG)),
        Some(app_dir.join(LEGACY_CONFIG)),
        Some(legacy_documents_dir().join(LEGACY_CONFIG)),
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join(LEGACY_CONFIG))),
        std::env::current_dir().ok().map(|d| d.join(LEGACY_CONFIG)),
    ]
    .into_iter()
    .flatten()
    .collect();

    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Some(s) = parse_ini(&text) {
                return Some(s);
            }
        }
    }
    None
}

/// 极简 INI 解析（只处理 `[main]` 与 `[users]` 两节）。
pub fn parse_ini(text: &str) -> Option<Settings> {
    let mut cfg = Settings::default();
    let mut section = String::new();
    let mut found_main = false;
    let mut users: Vec<UserEntry> = Vec::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_lowercase();
            if section == "main" {
                found_main = true;
            }
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim().to_lowercase();
        let val = v.trim().to_string();

        match section.as_str() {
            "main" => match key.as_str() {
                "path" => cfg.path = val,
                "cookie" => cfg.cookie = val,
                "use_mix_folder" => cfg.use_mix_folder = parse_bool(&val, true),
                "include_date_in_filename" => {
                    cfg.include_date_in_filename = parse_bool(&val, true)
                }
                "auto_select_after_fetch" => {
                    cfg.auto_select_after_fetch = parse_bool(&val, true)
                }
                "add_title_when_export_urls" => {
                    cfg.add_title_when_export_urls = parse_bool(&val, false)
                }
                "threads" => {
                    if let Ok(n) = val.parse::<u32>() {
                        cfg.threads = n.clamp(1, 32);
                    }
                }
                _ => {}
            },
            "users" => {
                if key.starts_with("user") {
                    if let Some((name, url)) = val.split_once(',') {
                        // 旧配置存的是整条 URL，这里抽出 sec_uid 后归一化
                        if let Some(sec) = extract_sec_uid(url.trim()) {
                            users.push(UserEntry {
                                username: name.trim().to_string(),
                                sec_uid: sec,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if !found_main && users.is_empty() {
        return None;
    }
    cfg.users = users;
    Some(cfg)
}

/// 从主页链接里抽 sec_uid（`/user/MS4wLjAB...` 或裸 ID）。
pub fn extract_sec_uid(url: &str) -> Option<String> {
    if let Some(pos) = url.find("/user/") {
        let rest = &url[pos + 6..];
        let end = rest.find(['?', '/', '&', '#']).unwrap_or(rest.len());
        let id = &rest[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    // 裸 ID 形式
    if let Some(pos) = url.find("MS4wLjAB") {
        let rest = &url[pos..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
            .unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

fn parse_bool(s: &str, default: bool) -> bool {
    match s.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

/// 程序自身所在目录。默认下载路径就用它——绿色软件语义，下载文件落在 exe 旁边。
pub fn exe_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 默认下载根：程序所在目录下的 `作品下载`。
///
/// 装在 `Program Files` 这类无写权限的位置时，`exe_dir()` 不可写，
/// 此时必须回退到文档目录——否则每一次下载都会以「拒绝访问」失败，
/// 而用户看到的只是一片莫名其妙的下载失败。
pub fn default_download_base() -> PathBuf {
    let exe = exe_dir();
    if is_writable(&exe) {
        return exe;
    }
    documents_dir().join("DouyinDownloader")
}

fn documents_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("DouyinDownloader")
}

/// 用真实写一个探针文件来判断可写性：只看权限位在 Windows 上不可靠
/// （ACL、UAC 虚拟化、只读介质都会让判断失真）。
fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".dy_write_probe");
    match std::fs::write(&probe, b"1") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 配置目录：与下载目录同源——程序所在目录（绿色软件语义）。
///
/// 程序目录不可写（例如装在 `Program Files`）时回退到文档目录，
/// 否则配置存不下、每次启动都像失忆。
pub fn default_app_dir() -> PathBuf {
    let dir = default_download_base();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 历史版本将配置存放在文档目录。此处保留只读入口，
/// 用于一次性的迁移读取——老用户升级后配置不会丢。
pub fn legacy_documents_dir() -> PathBuf {
    documents_dir()
}

fn base_of(settings: &Settings) -> PathBuf {
    let p = settings.path.trim();
    if p.is_empty() {
        default_download_base()
    } else {
        PathBuf::from(p)
    }
}

/// 下载根目录：`<基础路径>/作品下载`。
pub fn download_root(settings: &Settings) -> PathBuf {
    base_of(settings).join("作品下载")
}

pub fn excel_root(settings: &Settings) -> PathBuf {
    base_of(settings).join("作品数据Excel")
}

pub fn urls_root(settings: &Settings) -> PathBuf {
    base_of(settings).join("视频直链")
}

/// 用户子目录名：昵称-unique_id，再按模式加后缀。
///
/// 点赞与收藏必须各自独立成目录，否则同一个用户的三种列表会互相覆盖
/// —— 文件名相同但内容不同，覆盖后用户完全察觉不到。
pub fn user_folder_name(settings_id: &str, unique_id: &str, mode: &str) -> String {
    let mut name = if unique_id.is_empty() {
        settings_id.to_string()
    } else {
        format!("{settings_id}-{unique_id}")
    };
    match mode {
        "favorite" => name.push_str("-like"),
        "collect" => name.push_str("-collect"),
        _ => {}
    }
    name
}

pub fn ensure(app_dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(app_dir).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
[main]
path = D:\\Downloads\\Douyin
use_mix_folder = True
include_date_in_filename = True
auto_select_after_fetch = False
add_title_when_export_urls = True
threads = 8
cookie = sessionid=abc123; ttwid=xyz

[users]
user1 = 张三,https://www.douyin.com/user/MS4wLjABAAAA1
user2 = 李四,https://www.douyin.com/user/MS4wLjABAAAA2
";

    #[test]
    fn parses_legacy_ini() {
        let s = parse_ini(SAMPLE).expect("should parse");
        assert_eq!(s.path, "D:\\Downloads\\Douyin");
        assert!(s.use_mix_folder);
        assert!(s.include_date_in_filename);
        assert!(!s.auto_select_after_fetch);
        assert!(s.add_title_when_export_urls);
        assert_eq!(s.threads, 8);
        assert!(s.cookie.contains("sessionid="));
        assert_eq!(s.users.len(), 2);
        assert_eq!(s.users[0].username, "张三");
    }

    #[test]
    fn rejects_non_ini() {
        assert!(parse_ini("random text").is_none());
        assert!(parse_ini("").is_none());
    }

    #[test]
    fn bool_forms() {
        assert!(parse_bool("True", false));
        assert!(!parse_bool("false", true));
        assert!(parse_bool("garbage", true));
    }

    #[test]
    fn folder_name_rules() {
        assert_eq!(user_folder_name("张三", "abc", "post"), "张三-abc");
        assert_eq!(user_folder_name("张三", "", "post"), "张三");
        assert_eq!(user_folder_name("张三", "abc", "favorite"), "张三-abc-like");
        assert_eq!(user_folder_name("张三", "abc", "collect"), "张三-abc-collect");
        assert_eq!(user_folder_name("张三", "abc", "single"), "张三-abc");
    }

    #[test]
    fn app_dir_equals_exe_dir_when_writable() {
        // 配置与下载同源：都落在程序目录（绿色软件语义）
        assert_eq!(default_app_dir(), exe_dir());
    }

    #[test]
    fn legacy_documents_dir_is_distinct() {
        // 迁移来源目录仍然指向文档，避免与当前配置目录混为一谈
        let legacy = legacy_documents_dir();
        assert!(legacy.ends_with("DouyinDownloader"));
    }

    #[test]
    fn default_base_is_exe_dir_when_writable() {
        // 测试进程所在目录（target/debug/deps）必然可写 → 默认基目录就是它
        let base = default_download_base();
        assert_eq!(base, exe_dir(), "可写时应落在程序目录");
        assert!(is_writable(&base));
    }

    #[test]
    fn empty_path_falls_back_to_exe_dir() {
        let s = Settings::default();
        assert!(s.path.is_empty());
        assert_eq!(download_root(&s), exe_dir().join("作品下载"));
        assert_eq!(excel_root(&s), exe_dir().join("作品数据Excel"));
        assert_eq!(urls_root(&s), exe_dir().join("视频直链"));
    }

    #[test]
    fn explicit_path_wins() {
        let mut s = Settings::default();
        s.path = r"D:\我的下载".into();
        assert_eq!(download_root(&s), PathBuf::from(r"D:\我的下载").join("作品下载"));
    }

    #[test]
    fn readonly_dir_is_not_writable() {
        // 用一个不存在的深层路径验证探测函数不会误判为可写
        let ghost = std::env::temp_dir().join("dy_no_such_dir_xyz").join("deeper");
        assert!(!is_writable(&ghost));
    }

    #[test]
    fn roundtrip_save_load() {        let dir = std::env::temp_dir().join("dy_cfg_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = Settings::default();
        s.threads = 6;
        s.cookie = "sessionid=zzz".into();
        save(&dir, &s).unwrap();
        let back = load(&dir);
        assert_eq!(back.threads, 6);
        assert_eq!(back.cookie, "sessionid=zzz");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
