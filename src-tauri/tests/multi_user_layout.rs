//! 批量下载的目录归属验证。
//!
//! 验证多用户下载的目录归属：`TaskRow` 自带 owner 信息，
//! 每个作品写入各自账号的目录，不会混放。

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use douyin_downloader_lib::download::{run_queue, DownloadCtx};
use douyin_downloader_lib::models::TaskRow;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const BODY: &[u8] = b"MULTI-USER-PAYLOAD-0123456789";

async fn server() -> (SocketAddr, Arc<AtomicU32>) {
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
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    BODY.len()
                );
                if req.starts_with("GET") {
                    hits.fetch_add(1, Ordering::Relaxed);
                }
                let _ = sock.write_all(head.as_bytes()).await;
                if req.starts_with("GET") {
                    let _ = sock.write_all(BODY).await;
                }
                let _ = sock.flush().await;
            });
        }
    });
    (addr, hits)
}

fn task(desc: &str, url: String, owner_nick: &str, owner_uid: &str, owner_sec: &str) -> TaskRow {
    TaskRow {
        id: format!("id-{desc}"),
        aweme_id: format!("aw-{desc}"),
        owner_nickname: owner_nick.into(),
        owner_unique_id: owner_uid.into(),
        owner_sec_uid: owner_sec.into(),
        date: "2025-05-06".into(),
        desc: desc.into(),
        mix: String::new(),
        kind: "video".into(),
        url,
        play_url: String::new(),
        ext: ".mp4".into(),
        is_image: false,
        url_hash: format!("h-{desc}"),
    }
}

fn fresh(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("dy_multiuser_{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ctx(root: std::path::PathBuf) -> DownloadCtx {
    DownloadCtx {
        client: reqwest::Client::new(),
        cookie: String::new(),
        referer: String::new(),
        concurrency: 2,
        include_date: true,
        use_mix_folder: false,
        root,
        folder_suffix: String::new(),
    }
}

/// 列出相对路径（排除标记文件）。
fn tree(root: &std::path::Path) -> Vec<String> {
    fn walk(p: &std::path::Path, base: &std::path::Path, out: &mut Vec<String>) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    walk(&path, base, out);
                } else {
                    let rel = path
                        .strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    if rel.contains(".sec_uid_") {
                        continue;
                    }
                    out.push(rel);
                }
            }
        }
    }
    let mut v = Vec::new();
    walk(root, root, &mut v);
    v.sort();
    v
}

#[tokio::test]
async fn each_user_gets_own_folder() {
    let (addr, _) = server().await;
    let dir = fresh("folders");

    let rows = vec![
        task("作品甲", format!("http://{addr}/a.mp4"), "张三", "111", "MS4wLjABS1"),
        task("作品乙", format!("http://{addr}/b.mp4"), "李四", "222", "MS4wLjABS2"),
        task("作品丙", format!("http://{addr}/c.mp4"), "张三", "111", "MS4wLjABS1"),
    ];

    run_queue(rows, Arc::new(ctx(dir.clone())), || false, |_| {})
        .await
        .unwrap();

    let files = tree(&dir);
    println!("落盘结构: {files:?}");

    // 张三两个文件在自己的目录，李四一个
    assert!(
        files.iter().any(|f| f.starts_with("张三-111/")),
        "张三的文件应在 张三-111/ 下: {files:?}"
    );
    assert!(
        files.iter().any(|f| f.starts_with("李四-222/")),
        "李四的文件应在 李四-222/ 下: {files:?}"
    );
    // 不应出现「所有文件都在同一个目录」的旧行为
    assert!(
        !files.iter().all(|f| !f.contains('/')),
        "所有文件都在根目录，说明没有按用户分目录: {files:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn marker_written_per_user_folder() {
    let (addr, _) = server().await;
    let dir = fresh("markers");

    let rows = vec![
        task("甲", format!("http://{addr}/a.mp4"), "张三", "111", "MS4wLjABS1"),
        task("乙", format!("http://{addr}/b.mp4"), "李四", "222", "MS4wLjABS2"),
    ];

    run_queue(rows, Arc::new(ctx(dir.clone())), || false, |_| {})
        .await
        .unwrap();

    // 每个用户目录里都应有「以其 sec_user_id 命名」的空标记
    assert!(
        dir.join("张三-111").join(".sec_uid_MS4wLjABS1").exists(),
        "张三目录应写自己的 sec_uid 标记"
    );
    assert!(
        dir.join("李四-222").join(".sec_uid_MS4wLjABS2").exists(),
        "李四目录应写自己的 sec_uid 标记"
    );
    // 不应把别人的标记写进来
    assert!(!dir.join("张三-111").join(".sec_uid_MS4wLjABS2").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn favorite_suffix_keeps_lists_separate() {
    let (addr, _) = server().await;
    let dir = fresh("suffix");

    let mut c = ctx(dir.clone());
    c.folder_suffix = "-like".into();

    let rows = vec![task("点赞作品", format!("http://{addr}/a.mp4"), "张三", "111", "MS4wLjABS1")];
    run_queue(rows, Arc::new(c), || false, |_| {}).await.unwrap();

    assert!(
        dir.join("张三-111-like").exists(),
        "点赞模式的目录应带 -like 后缀: {:?}",
        tree(&dir)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn mix_folder_moves_existing_file_instead_of_redownloading() {
    let (addr, hits) = server().await;
    let dir = fresh("mixmove");

    // 第一次：不开合集目录
    let mut c1 = ctx(dir.clone());
    c1.use_mix_folder = false;
    let rows = vec![TaskRow {
        mix: "某合集".into(),
        ..task("作品A", format!("http://{addr}/a.mp4"), "张三", "111", "MS4wLjABS1")
    }];
    run_queue(rows.clone(), Arc::new(c1), || false, |_| {}).await.unwrap();
    let after_first = hits.load(Ordering::Relaxed);
    assert!(dir.join("张三-111").join("2025-05-06_作品A.mp4").exists());

    // 第二次：开启合集目录。大小预检应命中并把文件移进合集目录，
    // 不再发一次 GET。
    let mut c2 = ctx(dir.clone());
    c2.use_mix_folder = true;
    let summary = run_queue(rows, Arc::new(c2), || false, |_| {}).await.unwrap();
    let after_second = hits.load(Ordering::Relaxed);

    let files = tree(&dir);
    println!("第二次后结构: {files:?}，GET 次数 {} -> {}，{summary:?}",
        after_first, after_second);

    assert!(
        files.iter().any(|f| f.starts_with("张三-111/某合集/")),
        "文件应被移入合集目录: {files:?}"
    );
    assert_eq!(
        after_first, after_second,
        "复用不应产生新的 GET 请求"
    );
    assert_eq!(summary.skipped, 1, "应计入复用: {summary:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn date_prefix_toggle_moves_instead_of_redownloading() {
    let (addr, hits) = server().await;
    let dir = fresh("datemove");

    let mut c1 = ctx(dir.clone());
    c1.include_date = true;
    let rows = vec![task("作品A", format!("http://{addr}/a.mp4"), "张三", "111", "MS4wLjABS1")];
    run_queue(rows.clone(), Arc::new(c1), || false, |_| {}).await.unwrap();
    let after_first = hits.load(Ordering::Relaxed);
    assert!(dir.join("张三-111").join("2025-05-06_作品A.mp4").exists());

    // 关闭日期前缀：应改名为「作品A.mp4」，不重新下载
    let mut c2 = ctx(dir.clone());
    c2.include_date = false;
    let summary = run_queue(rows, Arc::new(c2), || false, |_| {}).await.unwrap();
    let after_second = hits.load(Ordering::Relaxed);

    let files = tree(&dir);
    println!("关闭日期前缀后: {files:?}，GET {} -> {}", after_first, after_second);

    assert!(
        files.contains(&"张三-111/作品A.mp4".to_string()),
        "应改名为不带日期的文件: {files:?}"
    );
    assert_eq!(after_first, after_second, "复用不应产生新的 GET");
    assert_eq!(summary.skipped, 1);

    let _ = std::fs::remove_dir_all(&dir);
}
