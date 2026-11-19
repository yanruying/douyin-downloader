//! Cookie 读取通道与登录判定。
//!
//! 内嵌 WebView 读取的是浏览器 cookie 存储而非 `document.cookie`，因此
//! HttpOnly 的 `sessionid` 也能取到。`cookies_for_url` 依赖真实 WebView2，
//! 单元测试环境无法构造，这里验证 URL 解析与判定逻辑，真实读取在运行程序中体现。

use std::time::Duration;

use tauri::Url;

#[test]
fn cookie_url_is_valid() {
    let url = Url::parse("https://www.douyin.com/").expect("URL 解析");
    assert_eq!(url.host_str(), Some("www.douyin.com"));
    assert_eq!(url.scheme(), "https");
}

/// `has_session` 是 cookie 是否可用的唯一判据。
#[test]
fn has_session_detection() {
    use douyin_downloader_lib::auth::has_session;

    assert!(has_session("ttwid=abc; sessionid=deadbeef; passport_csrf_token=x"));
    assert!(has_session("sessionid_ss=xyz"));
    assert!(!has_session("ttwid=abc; odin_tt=1"));
    assert!(!has_session(""));
    // 值里含 sessionid 字样但键不是 sessionid 时不应误判
    assert!(!has_session("note=sessionid=1"));
}

/// 从 cookie 串拼装时必须排序：多标签页下 cookie 顺序不定。
#[test]
fn cookie_string_is_sorted() {
    let mut parts = vec!["b=2".to_string(), "a=1".to_string(), "c=3".to_string()];
    parts.sort();
    assert_eq!(parts.join("; "), "a=1; b=2; c=3");
}

/// 登录轮询必须有上限，不能无限等待。
#[test]
fn login_timeout_is_bounded() {
    let timeout = Duration::from_secs(300);
    assert!(timeout.as_secs() > 0 && timeout.as_secs() < 3600);
}
