//! 本人账号抓取：链接留空时应抓取自己。
//!
//! `cargo test --test self_fetch_live -- --ignored --nocapture`

use douyin_downloader_lib::douyin::DouyinClient;
use douyin_downloader_lib::models::FetchMode;

/// 测试二进制的目录是 target/debug/deps，配置在上一级 target/debug。
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
    v
}

fn saved_cookie() -> Option<String> {
    for p in config_candidates() {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(c) = v["cookie"].as_str() {
                    if !c.is_empty() {
                        return Some(c.to_string());
                    }
                }
            }
        }
    }
    None
}

#[tokio::test]
#[ignore = "需要真实网络与有效 cookie"]
async fn empty_url_fetches_my_own_works() {
    let Some(cookie) = saved_cookie() else {
        panic!("未找到 cookie，请先在应用内登录")
    };
    let client = DouyinClient::new().unwrap();

    // 先确认能拿到自己的 sec_uid
    let me = client.self_profile(&cookie).await.expect("self_profile");
    println!(
        "本人：{} / {} / sec_uid={}",
        me.nickname, me.unique_id, me.sec_uid
    );
    assert!(!me.sec_uid.is_empty(), "应拿到 sec_uid");

    // 留空 URL + 主页模式 → 抓自己
    let (p, list) = client
        .fetch_all("", &cookie, FetchMode::Post, || false, |_| {}, |page, l| {
            println!("  第 {page} 页: {} 条", l.len());
        })
        .await
        .expect("fetch self");

    println!("抓取到 {} 个作品，nickname={}", list.len(), p.nickname);
    assert!(!list.is_empty(), "自己账号应至少有一个作品");
    assert_eq!(p.sec_uid, me.sec_uid, "应是同一个账号");
}

#[tokio::test]
#[ignore = "需要真实网络与有效 cookie"]
async fn empty_url_works_for_favorite_and_collect() {
    let Some(cookie) = saved_cookie() else {
        panic!("未找到 cookie")
    };
    let client = DouyinClient::new().unwrap();

    for mode in [FetchMode::Favorite, FetchMode::Collect] {
        match client.fetch_all("", &cookie, mode, || false, |_| {}, |_, _| {}).await {
            Ok((_, list)) => println!("{:?} 留空抓自己: {} 条", mode, list.len()),
            Err(e) => println!("{:?} 失败（可能接口需要特定权限）: {e}", mode),
        }
    }
}

#[tokio::test]
#[ignore = "需要真实网络"]
async fn single_mode_requires_url() {
    // 单作品模式没有「自己的作品」这种默认，必须给链接
    let client = DouyinClient::new().unwrap();
    let err = client
        .fetch_all("", "dummy", FetchMode::Single, || false, |_| {}, |_, _| {})
        .await
        .unwrap_err();
    println!("预期错误: {err}");
    // 短链会先尝试跟随跳转，失败时给出具体引导
    assert!(err.to_string().contains("作品 ID"), "实际: {err}");
}
