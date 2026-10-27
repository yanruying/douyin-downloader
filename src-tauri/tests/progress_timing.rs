//! 下载进度推送的时序验证。
//!
//! 验证进度按任务完成即时推送：慢速服务端配合多任务时，
//! 必须出现中间态，而非结束后一次性跳到 100%。

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use douyin_downloader_lib::download::{run_queue, DownloadCtx};
use douyin_downloader_lib::models::TaskRow;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const BODY: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// 每个响应延迟 `delay_ms`，让任务真的有「进行中」的时段。
async fn slow_server(delay_ms: u64) -> (SocketAddr, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = hits.clone();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let hits = h2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 2048];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let seq = hits.fetch_add(1, Ordering::Relaxed) as usize;

                // 每个请求返回不同长度，避免「大小预检」把任务判为同一作品，
                // 让本测试专注于进度推送时序
                let body_len = BODY.len() + (seq % 97);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body_len
                );
                if req.starts_with("HEAD") {
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.flush().await;
                    return;
                }
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(&vec![b'x'; body_len]).await;
                let _ = sock.flush().await;
            });
        }
    });
    (addr, hits)
}

fn task(i: usize, url: String) -> TaskRow {
    TaskRow {
        id: format!("t{i}"),
        aweme_id: format!("a{i}"),
        owner_nickname: String::new(),
        owner_unique_id: String::new(),
        owner_sec_uid: String::new(),
date: "2025-01-01".into(),
        desc: format!("任务{i}"),
        mix: String::new(),
        kind: "video".into(),
        url,
        play_url: String::new(),
        ext: ".mp4".into(),
        is_image: false,
        url_hash: format!("h{i}"),
    }
}

#[tokio::test]
async fn progress_is_emitted_incrementally_not_all_at_end() {
    let (addr, hits) = slow_server(160).await;
    let base = std::env::temp_dir().join("dy_progress_inc");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();

    let rows: Vec<TaskRow> = (0..8)
        .map(|i| task(i, format!("http://{addr}/f{i}.mp4")))
        .collect();

    let ctx = Arc::new(DownloadCtx {
        client: reqwest::Client::new(),
        cookie: String::new(),
        referer: String::new(),
        concurrency: 2,
        include_date: false,
        use_mix_folder: false,
        root: base.clone(),
        folder_suffix: String::new(),
    });

    let events: Arc<Mutex<Vec<(u32, u32, Instant)>>> = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    let started = Instant::now();

    let summary = run_queue(rows, ctx, || false, move |p| {
        ev.lock()
            .unwrap()
            .push((p.completed, p.total, Instant::now()));
    })
    .await
    .unwrap();

    let elapsed = started.elapsed();
    // 这个测试服务端对所有 URL 返回同一份 BODY，大小恒定，
    // 因此「大小预检」会把后续任务判为同一作品并复用。
    // 本测试关心的是**进度推送时序**，不关心实际下载了几次，所以只断言成功数。
    assert_eq!(summary.ok, 8, "8 条都应成功处理: {summary:?}");
    println!(
        "实际 GET {} 次，复用 {} 条",
        hits.load(Ordering::Relaxed),
        summary.skipped
    );

    let evs = events.lock().unwrap().clone();
    println!("共 {} 个进度事件，耗时 {:?}", evs.len(), elapsed);
    for (c, t, at) in &evs {
        println!("  {c}/{t} @ +{:?}", at.duration_since(started));
    }

    // 首事件 0/N
    assert_eq!(evs.first().map(|e| (e.0, e.1)), Some((0, 8)));
    // 末事件 N/N
    assert_eq!(evs.last().map(|e| (e.0, e.1)), Some((8, 8)));

    // 关键：中间必须出现「部分完成」的事件，且最后一个中间事件
    // 不能与结束事件挤在同一瞬间 —— 那说明进度是最后一次性刷出来的
    let mids: Vec<_> = evs.iter().filter(|e| e.0 > 0 && e.0 < 8).collect();
    assert!(
        !mids.is_empty(),
        "没有中间进度事件，说明进度是最后一次性推送的: {evs:?}"
    );

    let last_mid = mids.last().unwrap().2;
    let finish = evs.last().unwrap().2;
    let gap = finish.duration_since(last_mid);
    println!("最后一个中间事件到结束的间隔: {gap:?}");
    assert!(
        gap > Duration::from_millis(100),
        "进度在结束前就应持续更新，实际间隔仅 {gap:?}"
    );

    // 单调不减
    let mut prev = 0;
    for (c, _, _) in &evs {
        assert!(*c >= prev, "进度回退: {evs:?}");
        prev = *c;
    }

    let _ = std::fs::remove_dir_all(&base);
}

#[tokio::test]
async fn progress_events_are_throttled() {
    let (addr, _) = slow_server(0).await;
    let base = std::env::temp_dir().join("dy_progress_throttle");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();

    let rows: Vec<TaskRow> = (0..40)
        .map(|i| task(i, format!("http://{addr}/f{i}.mp4")))
        .collect();

    let ctx = Arc::new(DownloadCtx {
        client: reqwest::Client::new(),
        cookie: String::new(),
        referer: String::new(),
        concurrency: 4,
        include_date: false,
        use_mix_folder: false,
        root: base.clone(),
        folder_suffix: String::new(),
    });

    let count = Arc::new(AtomicU32::new(0));
    let c2 = count.clone();

    let summary = run_queue(rows, ctx, || false, move |_| {
        c2.fetch_add(1, Ordering::Relaxed);
    })
    .await
    .unwrap();

    assert_eq!(summary.ok, 40);
    let n = count.load(Ordering::Relaxed);
    println!("40 个任务产生 {n} 个进度事件");
    // 300ms 节流 + 收尾一次：事件数应远小于任务数
    assert!(n <= 10, "节流失效，事件数 {n} 过多");

    let _ = std::fs::remove_dir_all(&base);
}

#[tokio::test]
async fn stop_marks_summary_as_stopped() {
    let (addr, _) = slow_server(120).await;
    let base = std::env::temp_dir().join("dy_progress_stop");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();

    let rows: Vec<TaskRow> = (0..20)
        .map(|i| task(i, format!("http://{addr}/f{i}.mp4")))
        .collect();

    let ctx = Arc::new(DownloadCtx {
        client: reqwest::Client::new(),
        cookie: String::new(),
        referer: String::new(),
        concurrency: 2,
        include_date: false,
        use_mix_folder: false,
        root: base.clone(),
        folder_suffix: String::new(),
    });

    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let f2 = flag.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        f2.store(true, Ordering::Relaxed);
    });

    let summary = run_queue(
        rows, ctx,
        move || flag.load(Ordering::Relaxed),
        |_| {},
    )
    .await
    .unwrap();

    println!("停止后: {summary:?}");
    assert!(summary.stopped, "停止标志未反映到结果里");
    assert!(summary.ok < 20, "停止后不应全部完成");

    let _ = std::fs::remove_dir_all(&base);
}
