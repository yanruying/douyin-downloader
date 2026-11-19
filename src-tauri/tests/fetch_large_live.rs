//! 大列表真实联调：验证几千条作品能一次抓全。
//!
//! 默认 `#[ignore]`（需要网络 + 有效 cookie），手动运行：
//!   cargo test --test fetch_large_live -- --ignored --nocapture
//!
//! 关注的不是「能抓到」，而是「不会中途掉一半」：
//! - 逐页打印条数与累计，任何一页掉 0 条都会暴露出来
//! - 打印 has_more 与游标的实际取值形态，便于判断服务端何时撒谎
//! - 结束时打印 warning：非空说明确实没抓全，同时保留了已抓部分

use douyin_downloader_lib::douyin::DouyinClient;
use douyin_downloader_lib::models::FetchMode;

fn config_candidates() -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            v.push(d.join("settings.json"));
            if let Some(up) = d.parent() {
                v.push(up.join("settings.json"));
            }
        }
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        v.push(std::path::Path::new(&h).join("Documents/DouyinDownloader/settings.json"));
    }
    if let Ok(p) = std::env::var("SETTINGS_JSON") {
        v.push(std::path::PathBuf::from(p));
    }
    if let Ok(cwd) = std::env::current_dir() {
        v.push(cwd.join("settings.json"));
    }
    v
}

fn saved_config() -> Option<serde_json::Value> {
    for p in config_candidates() {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                return Some(v);
            }
        }
    }
    None
}

/// 用真实账号跑点赞列表，逐页打印。
#[tokio::test]
#[ignore = "需要真实网络与有效 cookie"]
async fn favorite_list_fetches_all_pages() {
    let cfg = saved_config().expect("未找到 settings.json");
    let cookie = cfg["cookie"].as_str().unwrap_or_default().to_string();
    let sec = cfg["secUid"].as_str().unwrap_or_default().to_string();
    assert!(!cookie.is_empty(), "cookie 为空");
    assert!(!sec.is_empty(), "secUid 为空");
    println!("账号 secUid={sec}");

    let client = DouyinClient::new().expect("构建 client");
    let mut pages = 0u32;

    let (profile, outcome) = client
        .fetch_all(
            "",
            &cookie,
            FetchMode::Favorite,
            || false,
            |_| {},
            |page, list| {
                pages = page;
                println!("  第 {page} 页 {} 条（累计到本页）", list.len());
            },
        )
        .await
        .expect("抓取失败");

    println!("=== {} / {} ===", profile.nickname, profile.unique_id);
    println!("共 {} 页，{} 条", pages, outcome.count);
    println!("warning = {:?}", outcome.warning);
    println!("stopped = {}", outcome.stopped);

    // 只要没有任何一页报错，就不该出现 warning
    assert!(
        outcome.warning.is_none(),
        "抓取不完整：{:?}",
        outcome.warning
    );
    // 已知该账号有 76 条收藏；至少说明空页终止没提前触发
    assert!(outcome.count >= 50, "条数过少，疑似中途停止");
}

/// 主页作品列表：用真实博主验证几千条不中断。
#[tokio::test]
#[ignore = "需要真实网络与有效 cookie"]
async fn post_list_fetches_all_pages() {
    let cfg = saved_config().expect("未找到 settings.json");
    let cookie = cfg["cookie"].as_str().unwrap_or_default().to_string();
    assert!(!cookie.is_empty(), "cookie 为空");

    // 作品数较多的公开账号；用 FETCH_URL 换成自己要验证的链接
    let url = std::env::var("FETCH_URL").unwrap_or_else(|_| {
        "https://www.douyin.com/user/MS4wLjABAAAAsample_user_sec_uid_replace_with_yours"
            .to_string()
    });
    println!("目标: {url}");

    let client = DouyinClient::new().expect("构建 client");
    let mut last = 0u32;

    let (profile, outcome) = client
        .fetch_all(
            &url,
            &cookie,
            FetchMode::Post,
            || false,
            |_| {},
            |page, list| {
                last = page;
                println!("  第 {page} 页 {} 条", list.len());
            },
        )
        .await
        .expect("抓取失败");

    println!("=== {} / {} ===", profile.nickname, profile.unique_id);
    println!("共 {last} 页，{} 条", outcome.count);
    println!("warning = {:?}", outcome.warning);

    assert!(
        outcome.warning.is_none(),
        "抓取不完整：{:?}",
        outcome.warning
    );
    assert!(outcome.count > 100, "条数异常偏少");
}
