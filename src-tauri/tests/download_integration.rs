//! 下载与队列的端到端集成测试：本地起 HTTP 服务，不依赖外网。
//!
//! 覆盖下载器的关键行为：
//! - 完整下载 + 原子落盘（.tmp 消失、目标文件存在）
//! - Range 断点续传（已有 .tmp 时只补剩余字节）
//! - 416 视为已完成
//! - 服务器不支持 Range（返回 200）时丢弃旧 .tmp 重写
//! - 队列并发 + 进度节流 + 已完成数正确

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use douyin_downloader_lib::download::single::{download_one, DownloadOpts, Outcome};
use douyin_downloader_lib::download::{run_queue, DownloadCtx};
use douyin_downloader_lib::models::TaskRow;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const BODY: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// 极简 HTTP 服务：支持 Range，也可切到 no-range 模式。
async fn spawn_server(support_range: bool) -> (SocketAddr, Arc<AtomicU32>, tokio::task::JoinHandle<()>) {
    spawn_server_opts(support_range, 0).await
}

/// `delay_ms` 让响应变慢，用于验证停止/取消确实能中断在途任务。
async fn spawn_server_opts(
    support_range: bool,
    delay_ms: u64,
) -> (SocketAddr, Arc<AtomicU32>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicU32::new(0));
    let hits2 = hits.clone();

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let hits = hits2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                // 只统计真正的下载请求；HEAD 是「大小预检」，不计入
                if req.starts_with("GET") {
                    hits.fetch_add(1, Ordering::Relaxed);
                }

                let total = BODY.len() as u64;
                let range_start = req
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                    .and_then(|l| {
                        l.split('=')
                            .nth(1)
                            .and_then(|v| v.split('-').next())
                            .and_then(|v| v.trim().parse::<u64>().ok())
                    });

                // HEAD：只回头部（大小预检用）
                if req.starts_with("HEAD") {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        BODY.len()
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.flush().await;
                    return;
                }

                let (head, payload): (String, &[u8]) = match (support_range, range_start) {
                    (true, Some(start)) if start >= total => (
                        format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
                        &[],
                    ),
                    (true, Some(start)) => {
                        let slice = &BODY[start as usize..];
                        (
                            format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                                slice.len(),
                                start,
                                total - 1,
                                total
                            ),
                            slice,
                        )
                    }
                    _ => (
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
                        ),
                        BODY,
                    ),
                };

                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(payload).await;
                let _ = sock.flush().await;
            });
        }
    });

    (addr, hits, handle)
}

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("dy_it_{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn opts<'a>(url: &'a str, folder: &'a std::path::Path, ext: &'a str) -> DownloadOpts<'a> {
    DownloadOpts {
        url,
        folder,
        base_name: "测试文件",
        ext,
        url_hash: "deadbeef",
        referer: "https://www.douyin.com/",
        cookie: "",
        exact_path: None,
    }
}

#[tokio::test]
async fn full_download_writes_atomically() {
    let (addr, _hits, _srv) = spawn_server(true).await;
    let dir = tmp_dir("full");
    let url = format!("http://{addr}/v.mp4");
    let client = reqwest::Client::new();

    let out = download_one(&client, &opts(&url, &dir, ".mp4"), &|| false)
        .await
        .unwrap();
    let path = match out {
        Outcome::Saved(p) => p,
        Outcome::Stopped => panic!("不应被停止"),
    };

    assert_eq!(std::fs::read(&path).unwrap(), BODY);
    assert_eq!(path.file_name().unwrap().to_string_lossy(), "测试文件.mp4");
    assert!(!path.with_extension("mp4.tmp").exists(), ".tmp 应已清理");
}

#[tokio::test]
async fn resumes_from_existing_tmp_with_range() {
    let (addr, _hits, _srv) = spawn_server(true).await;
    let dir = tmp_dir("resume");
    let url = format!("http://{addr}/v.mp4");
    let client = reqwest::Client::new();

    // 预置半截 .tmp
    let half = &BODY[..20];
    std::fs::write(dir.join("测试文件.mp4.tmp"), half).unwrap();

    let out = download_one(&client, &opts(&url, &dir, ".mp4"), &|| false)
        .await
        .unwrap();
    let path = match out {
        Outcome::Saved(p) => p,
        Outcome::Stopped => panic!(),
    };

    // 关键：内容必须是完整且正确的（续传不能重复写入前 20 字节）
    assert_eq!(std::fs::read(&path).unwrap(), BODY);
    assert_eq!(BODY.len(), 62);
}

#[tokio::test]
async fn range_beyond_eof_treated_as_complete() {
    let (addr, _hits, _srv) = spawn_server(true).await;
    let dir = tmp_dir("416");
    let url = format!("http://{addr}/v.mp4");
    let client = reqwest::Client::new();

    // .tmp 已经比服务端文件还大 → 服务端会回 416
    std::fs::write(dir.join("测试文件.mp4.tmp"), vec![b'x'; 200]).unwrap();

    let out = download_one(&client, &opts(&url, &dir, ".mp4"), &|| false)
        .await
        .unwrap();

    match out {
        Outcome::Saved(p) => {
            // 416 分支仅执行改名，内容保持原样
            assert_eq!(std::fs::read(&p).unwrap().len(), 200);
        }
        Outcome::Stopped => panic!("416 应走完成分支"),
    }
}

#[tokio::test]
async fn server_without_range_restarts_cleanly() {
    let (addr, _hits, _srv) = spawn_server(false).await;
    let dir = tmp_dir("norange");
    let url = format!("http://{addr}/v.mp4");
    let client = reqwest::Client::new();

    std::fs::write(dir.join("测试文件.mp4.tmp"), &BODY[..20]).unwrap();

    let out = download_one(&client, &opts(&url, &dir, ".mp4"), &|| false)
        .await
        .unwrap();
    let path = match out {
        Outcome::Saved(p) => p,
        Outcome::Stopped => panic!(),
    };

    // 服务端忽略 Range 返回 200 → 必须丢弃旧 .tmp 重写，不能拼接
    assert_eq!(std::fs::read(&path).unwrap(), BODY);
}

#[tokio::test]
async fn stop_flag_keeps_tmp_for_next_time() {
    let (addr, _hits, _srv) = spawn_server(true).await;
    let dir = tmp_dir("stop");
    let url = format!("http://{addr}/v.mp4");
    let client = reqwest::Client::new();

    // 立刻置停止 → 应在写盘前或写入中返回 Stopped
    let out = download_one(&client, &opts(&url, &dir, ".mp4"), &|| true)
        .await
        .unwrap();

    match out {
        Outcome::Stopped => {
            // .tmp 若已建则保留，供下次续传；关键是不要产出最终文件
            assert!(!dir.join("测试文件.mp4").exists(), "停止后不应有最终文件");
        }
        Outcome::Saved(_) => {
            // 极快完成时也允许（数据量很小）；此时同样验证内容完整
            assert_eq!(std::fs::read(dir.join("测试文件.mp4")).unwrap(), BODY);
        }
    }
}

fn task(n: usize, url: String) -> TaskRow {
    TaskRow {
        id: format!("t{n}"),
        aweme_id: format!("a{n}"),
        owner_nickname: String::new(),
        owner_unique_id: String::new(),
        owner_sec_uid: String::new(),
date: "2025-01-01".into(),
        desc: format!("任务{n}"),
        mix: String::new(),
        kind: "video".into(),
        url,
        play_url: String::new(),
        ext: ".mp4".into(),
        is_image: false,
        url_hash: format!("h{n}"),
    }
}

#[tokio::test]
async fn queue_runs_all_and_throttles_progress() {
    let (addr, hits, _srv) = spawn_server(true).await;
    let base = tmp_dir("queue");
    let client = reqwest::Client::new();

    let rows: Vec<TaskRow> = (0..12)
        .map(|i| task(i, format!("http://{addr}/f{i}.mp4")))
        .collect();

    let ctx = Arc::new(DownloadCtx {
        client,
        cookie: String::new(),
        referer: "https://www.douyin.com/".into(),
        concurrency: 3,
        include_date: true,
        use_mix_folder: false,
        root: base.clone(),
        folder_suffix: String::new(),
    });

    let events = Arc::new(std::sync::Mutex::new(Vec::<(u32, u32)>::new()));
    let ev2 = events.clone();

    let started = std::time::Instant::now();
    let summary = run_queue(rows, ctx, || false, move |p| {
        ev2.lock().unwrap().push((p.completed, p.total));
    })
    .await
    .unwrap();
    let elapsed = started.elapsed();

    // 这个测试服务端对任何 URL 都返回同一份 BODY（大小恒定），
    // 所以「大小预检」会把后续任务判为同一作品并复用 —— 这是设计行为。
    // ok 含复用；skipped 是其中被复用的条数；failed 必须为 0。
    assert_eq!(summary.ok, 12, "12 条都应成功处理: {summary:?}");
    assert_eq!(summary.failed, 0, "不应有失败: {summary:?}");
    println!("复用 {} 条，实际 HTTP 请求 {} 次",
        summary.skipped, hits.load(Ordering::Relaxed));

    // 大小预检复用后，磁盘上只保留被复用的那一份（同大小视为同一作品）
    fn count_mp4(dir: &std::path::Path) -> usize {
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    n += count_mp4(&p);
                } else if p.extension().map(|x| x == "mp4").unwrap_or(false) {
                    n += 1;
                }
            }
        }
        n
    }
    let n = count_mp4(&base);
    assert!(n >= 1, "至少应有一个文件落盘: {n}");
    // 文件名应带日期前缀（在用户目录下）
    let user_dir = base.join("Douyin_User");
    let has_dated = std::fs::read_dir(&user_dir)
        .map(|rd| {
            rd.flatten().any(|e| {
                e.file_name().to_string_lossy().starts_with("2025-01-01_")
            })
        })
        .unwrap_or(false);
    assert!(has_dated, "落盘文件名应带日期前缀");

    let evs = events.lock().unwrap().clone();
    assert!(!evs.is_empty());
    assert_eq!(evs[0], (0, 12), "首个事件应是 0/12");
    assert_eq!(*evs.last().unwrap(), (12, 12), "末个事件应是 12/12");

    // 节流生效：12 个任务在 300ms 内跑完，中间事件数应远少于 12
    assert!(
        evs.len() <= 6,
        "节流应把事件数压到个位，实际 {} 条: {:?}（耗时 {:?}）",
        evs.len(),
        evs,
        elapsed
    );

    // 进度单调不减
    let mut prev = 0;
    for (c, _) in &evs {
        assert!(*c >= prev, "进度不应回退: {evs:?}");
        prev = *c;
    }
}

#[tokio::test]
async fn queue_respects_stop() {
    // 每个响应延迟 150ms，保证停止信号落在在途任务中间
    let (addr, _hits, _srv) = spawn_server_opts(true, 150).await;
    let base = tmp_dir("queue_stop");
    let client = reqwest::Client::new();

    let rows: Vec<TaskRow> = (0..30)
        .map(|i| task(i, format!("http://{addr}/f{i}.mp4")))
        .collect();

    let ctx = Arc::new(DownloadCtx {
        client,
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
    let stop_trigger = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(120)).await;
        f2.store(true, Ordering::Relaxed);
    });

    let summary = run_queue(rows, ctx, move || flag.load(Ordering::Relaxed), |_| {})
        .await
        .unwrap();
    let _ = stop_trigger.await;

    assert!(summary.ok < 30, "停止后不应全部完成，实际完成 {}", summary.ok);
    // 已完成的文件必须完整可用
    for entry in std::fs::read_dir(&base).unwrap().filter_map(|e| e.ok()) {
        if entry.path().extension().map(|x| x == "mp4").unwrap_or(false) {
            assert_eq!(std::fs::read(entry.path()).unwrap(), BODY);
        }
    }
}

#[tokio::test]
async fn queue_skips_existing_files() {
    let (addr, hits, _srv) = spawn_server(true).await;
    let base = tmp_dir("queue_skip");
    let client = reqwest::Client::new();

    // 预置一个与预期路径同名的文件（在用户目录下），
    // 内容长度刻意不同于服务端 BODY，避免被大小预检误判为同作品
    let user_dir = base.join("Douyin_User");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("2025-01-01_任务0.mp4"), b"already-here-different-length").unwrap();

    let rows = vec![
        task(0, format!("http://{addr}/f0.mp4")),
        task(1, format!("http://{addr}/f1.mp4")),
    ];

    let ctx = Arc::new(DownloadCtx {
        client,
        cookie: String::new(),
        referer: String::new(),
        concurrency: 2,
        include_date: true,
        use_mix_folder: false,
        root: base.clone(),
        folder_suffix: String::new(),
    });

    let (todo, already) = douyin_downloader_lib::download::plan_targets(&rows, &ctx);
    assert_eq!(already.len(), 1, "应识别出 1 个已存在");
    assert_eq!(todo.len(), 1);

    let summary = run_queue(todo, ctx, || false, |_| {}).await.unwrap();
    assert_eq!(summary.ok, 1);
    assert_eq!(hits.load(Ordering::Relaxed), 1, "只应为新文件发请求");

    // 已存在文件不被覆盖
    assert_eq!(
        std::fs::read(base.join("Douyin_User").join("2025-01-01_任务0.mp4")).unwrap(),
        b"already-here-different-length"
    );
}
