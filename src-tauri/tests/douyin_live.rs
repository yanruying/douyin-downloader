//! 真实接口联调：用已保存的 cookie 抓取一个公开主页。
//!
//! 默认 `#[ignore]`（需要网络 + 有效 cookie），手动运行：
//!   cargo test --test douyin_live -- --ignored --nocapture
//!
//! 目的：验证「a_bogus 签名能被抖音接受」这一唯一无法用单元测试覆盖的点。
//! 签名算法已与 Python 版逐字节一致；本测试确认服务端也认可。

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
async fn fetch_real_profile_succeeds() {
    let Some(cookie) = saved_cookie() else {
        panic!("未找到已保存的 cookie，请先在应用内登录");
    };
    println!("cookie 长度: {}", cookie.len());

    let client = DouyinClient::new().expect("构建 client");

    // 先用一个公开的抖音官方账号做验证（不依赖用户自己的账号）
    // 公开账号示例 ID
    let url = "https://www.douyin.com/user/MS4wLjABAAAArDVBosPJF3eIWVEFp0szuJ-e1V_-rK0ieJeWwpE77E8";

    let status = client.check_cookie(&cookie).await;
    println!("cookie 状态: {status:?}");

    match client.fetch_all(url, &cookie, FetchMode::Post, || false, |_| {}, |page, list| {
        println!("第 {page} 页: {} 条", list.len());
    }).await {
        Ok((p, list)) => {
            println!("昵称={} unique_id={} 作品数={}", p.nickname, p.unique_id, list.len());
            assert!(!list.is_empty(), "应至少拿到一条作品");
        }
        Err(e) => {
            panic!("抓取失败（若为 status_code != 0，说明签名或指纹被拒）: {e}");
        }
    }
}

#[tokio::test]
#[ignore = "需要真实网络与有效 cookie"]
async fn signature_accepted_by_server() {
    // 最小验证：只打一页，确认不是 403 / status_code != 0
    let Some(cookie) = saved_cookie() else {
        panic!("未找到 cookie");
    };
    let client = DouyinClient::new().unwrap();
    let sec = "MS4wLjABAAAArDVBosPJF3eIWVEFp0szuJ-e1V_-rK0ieJeWwpE77E8";
    match douyin_downloader_lib::douyin::api::fetch_profile_info(&client.client, sec, &cookie).await {
        Ok(p) => println!("profile OK: {} / {}", p.nickname, p.unique_id),
        Err(e) => panic!("profile 失败: {e}"),
    }
}
