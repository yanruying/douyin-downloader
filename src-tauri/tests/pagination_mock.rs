//! 分页终止条件的回归测试：本地起 mock 服务，不依赖外网。
//!
//! 锁住两条行为，它们直接决定「几千个作品能不能抓全」：
//! - 中途 has_more=0 **不能**当作终止条件（服务端在数据量大时会中途抛 0）
//! - 游标不前进必须停（否则同一页被反复抓取，越抓越慢且内存一直涨）

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use douyin_downloader_lib::douyin::api::{FetchCtx, PAGE_COUNT};
use douyin_downloader_lib::douyin::sign::AbogusProvider;
use douyin_downloader_lib::models::FetchMode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// 一页的应答脚本。`items` 条空作品，返回给定 has_more / max_cursor。
#[derive(Clone, Copy)]
struct Page {
    items: u32,
    has_more: u8,
    next_cursor: i64,
}

/// 起一个按 `max_cursor` 查询参数挑页的 mock 服务，返回 base URL。
async fn spawn_mock(pages: Vec<(i64, Page)>) -> (String, tokio::task::JoinHandle<()>, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicU32::new(0));
    let hits2 = hits.clone();
    let pages = Arc::new(pages);

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let pages = pages.clone();
            let hits = hits2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                hits.fetch_add(1, Ordering::Relaxed);

                let cursor = req
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|target| target.split_once('?'))
                    .map(|(_, q)| q)
                    .unwrap_or("")
                    .split('&')
                    .find_map(|kv| {
                        let (k, v) = kv.split_once('=')?;
                        (k == "max_cursor").then(|| v.parse::<i64>().ok())?
                    })
                    .unwrap_or(0);

                let page = pages
                    .iter()
                    .find(|(c, _)| *c == cursor)
                    .map(|(_, p)| *p)
                    .unwrap_or(Page {
                        items: 0,
                        has_more: 0,
                        next_cursor: cursor,
                    });

                let list: Vec<String> = (0..page.items)
                    .map(|i| format!("{{\"aweme_id\":\"{cursor}_{i}\"}}"))
                    .collect();
                let body = format!(
                    "{{\"status_code\":0,\"aweme_list\":[{}],\"max_cursor\":{},\"has_more\":{}}}",
                    list.join(","),
                    page.next_cursor,
                    page.has_more
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });

    (format!("http://{addr}"), handle, hits)
}

fn ctx(base: &str) -> FetchCtx {
    let mut c = FetchCtx::new(
        douyin_downloader_lib::douyin::api::client().unwrap(),
        Arc::new(AbogusProvider::new(None)),
        "sessionid=test".to_string(),
    );
    c.base_override = Some(base.to_string());
    c
}

/// 第 2 页起服务端就说 has_more=0，但游标仍在前进 —— 必须继续抓到空页。
#[tokio::test]
async fn keeps_paging_when_has_more_lies() {
    let (base, handle, _hits) = spawn_mock(vec![
        (0, Page { items: PAGE_COUNT, has_more: 0, next_cursor: 100 }),
        (100, Page { items: PAGE_COUNT, has_more: 0, next_cursor: 200 }),
        (200, Page { items: PAGE_COUNT, has_more: 1, next_cursor: 300 }),
    ])
    .await;

    let mut seen = 0u32;
    let outcome = ctx(&base)
        .fetch_all("SEC", FetchMode::Favorite, || false, |_, list| {
            seen += list.len() as u32
        })
        .await
        .expect("不应报错");

    handle.abort();
    // 三页全部落库：has_more=0 没有让循环提前退出
    assert_eq!(outcome.count, PAGE_COUNT * 3, "has_more=0 不应终止翻页");
    assert_eq!(seen, outcome.count, "回调累计数应与返回条数一致");
    assert!(outcome.warning.is_none(), "正常到底不该有 warning");
}

/// 游标不前进时必须停，否则同一页会被无限重抓。
#[tokio::test]
async fn stops_when_cursor_does_not_advance() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    // 无论入参是什么，都返回同一个游标 7
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let _ = sock.read(&mut buf).await;
                let body = format!(
                    "{{\"status_code\":0,\"aweme_list\":[{}],\"max_cursor\":7,\"has_more\":1}}",
                    vec!["{\"aweme_id\":\"a\"}"; PAGE_COUNT as usize].join(",")
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });

    let started = std::time::Instant::now();
    let outcome = ctx(&format!("http://{addr}"))
        .fetch_all("SEC", FetchMode::Favorite, || false, |_, _| {})
        .await
        .expect("不应报错");

    handle.abort();
    // 第 1 页 cursor 0→7 正常；第 2 页 7→7 触发停止。
    // count 只统计唯一作品，所以两页同内容时仍是 PAGE_COUNT 条。
    assert_eq!(
        outcome.count, PAGE_COUNT,
        "游标重复时应立刻停止，且重复内容不计入条数"
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "未及时停止，耗时 {:?}",
        started.elapsed()
    );
}

/// 游标一直前进但内容重复（服务端到底后仍喂旧数据）时必须停。
#[tokio::test]
async fn stops_when_content_repeats_despite_cursor() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let calls = Arc::new(AtomicU32::new(0));
    let calls2 = calls.clone();

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let calls = calls2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let _ = sock.read(&mut buf).await;
                // 每次游标都变，但 aweme_id 恒定 —— 服务端在重复喂同一批
                let next = calls.fetch_add(1, Ordering::Relaxed) as i64 * 100 + 100;
                let body = format!(
                    "{{\"status_code\":0,\"aweme_list\":[{}],\"max_cursor\":{next},\"has_more\":1}}",
                    vec!["{\"aweme_id\":\"same\"}"; 20].join(",")
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });

    let outcome = ctx(&format!("http://{addr}"))
        .fetch_all("SEC", FetchMode::Favorite, || false, |_, _| {})
        .await
        .expect("不应报错");

    handle.abort();
    // 第 1 页有 20 条新内容，之后连续 3 页全重复 → 停止
    assert_eq!(outcome.count, 20, "重复内容只算一次");
    assert!(
        calls.load(Ordering::Relaxed) <= 8,
        "重复页应及时刹车，实际请求 {} 次",
        calls.load(Ordering::Relaxed)
    );
}

/// 单页空**不**是到底信号：服务端会在分片边界吐单页空，后面还有数据。
///
/// 这是「几千个作品只抓到一百多个」的直接回归用例：真实账号在第 26 页
/// 返回空页，第 27 页又有数据，一直翻到第 66 页游标归零才结束。
#[tokio::test]
async fn keeps_paging_through_a_single_empty_page() {
    let (base, handle, _) = spawn_mock(vec![
        (0, Page { items: 30, has_more: 1, next_cursor: 100 }),
        (100, Page { items: 0, has_more: 1, next_cursor: 200 }), // 中途空页
        (200, Page { items: 30, has_more: 1, next_cursor: 300 }),
        (300, Page { items: 30, has_more: 0, next_cursor: 400 }),
        (400, Page { items: 0, has_more: 0, next_cursor: 500 }),
        (500, Page { items: 0, has_more: 0, next_cursor: 600 }),
        (600, Page { items: 0, has_more: 0, next_cursor: 700 }),
        (700, Page { items: 0, has_more: 0, next_cursor: 800 }),
        (800, Page { items: 0, has_more: 0, next_cursor: 900 }),
    ])
    .await;

    let outcome = ctx(&base)
        .fetch_all("SEC", FetchMode::Favorite, || false, |_, _| {})
        .await
        .expect("不应报错");

    handle.abort();
    assert_eq!(
        outcome.count,
        90,
        "空页之后的 2 页数据必须继续抓到，不能因单页空提前退出"
    );
    assert!(outcome.warning.is_none());
}

/// 连续空页达到阈值才认作到底。
#[tokio::test]
async fn stops_after_consecutive_empty_pages() {
    let mut pages = vec![(0, Page { items: 25, has_more: 1, next_cursor: 100 })];
    for i in 0..8i64 {
        pages.push((
            100 + i * 100,
            Page { items: 0, has_more: 0, next_cursor: 200 + i * 100 },
        ));
    }
    let (base, handle, hits) = spawn_mock(pages).await;

    let outcome = ctx(&base)
        .fetch_all("SEC", FetchMode::Favorite, || false, |_, _| {})
        .await
        .expect("不应报错");

    handle.abort();
    assert_eq!(outcome.count, 25);
    assert!(
        hits.load(Ordering::Relaxed) <= 7,
        "连续空页应及时刹车，实际请求 {} 次",
        hits.load(Ordering::Relaxed)
    );
}

/// 中途单页反序列化失败时，先重试同一页；重试成功就不该丢数据。
#[tokio::test]
async fn retries_broken_page_then_continues() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let calls = Arc::new(AtomicU32::new(0));
    let calls2 = calls.clone();

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let calls = calls2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let cursor = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|t| t.split_once('?'))
                    .map(|(_, q)| q)
                    .unwrap_or("")
                    .split('&')
                    .find_map(|kv| {
                        let (k, v) = kv.split_once('=')?;
                        (k == "max_cursor").then(|| v.parse::<i64>().ok())?
                    })
                    .unwrap_or(0);

                let nth = calls.fetch_add(1, Ordering::Relaxed);
                // 第 2 次请求（即第 2 页的首次尝试）返回坏 JSON
                let body = if nth == 1 && cursor == 100 {
                    "{\"status_code\":0,\"aweme_list\":".to_string()
                } else if cursor == 0 {
                    format!(
                        "{{\"status_code\":0,\"aweme_list\":[{}],\"max_cursor\":100,\"has_more\":1}}",
                        vec!["{\"aweme_id\":\"a\"}"; 10].join(",")
                    )
                } else if cursor == 100 {
                    format!(
                        "{{\"status_code\":0,\"aweme_list\":[{}],\"max_cursor\":200,\"has_more\":1}}",
                        vec!["{\"aweme_id\":\"b\"}"; 10].join(",")
                    )
                } else {
                    "{\"status_code\":0,\"aweme_list\":[],\"max_cursor\":200,\"has_more\":1}".to_string()
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });

    let outcome = ctx(&format!("http://{addr}"))
        .fetch_all("SEC", FetchMode::Favorite, || false, |_, _| {})
        .await
        .expect("不应报错");

    handle.abort();
    assert_eq!(outcome.count, 20, "坏页重试成功后应继续，数据不丢");
    assert!(outcome.warning.is_none(), "重试成功不该报 warning");
}
