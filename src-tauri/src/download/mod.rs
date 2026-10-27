//! 下载队列编排：Semaphore 限并发 + JoinSet + 进度节流 + 取消。
//!
//! 关键设计：
//! - **按作品归属分目录**：`TaskRow` 自带 owner 信息，批量获取多个用户时
//!   各进各的文件夹，互不混放。
//! - **重复检测用大小预检**：不发 GET，只发 HEAD 比对 Content-Length，
//!   命中即复用已有文件并按当前设置移动/改名。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

pub mod marker;
pub mod reuse;
pub mod single;

use crate::download::single::{download_with_retry, DownloadOpts};
use crate::error::AppResult;
use crate::models::{DownloadSummary, ProgressEvent, TaskRow};
use crate::paths::{build_expected_filename, ensure_dir, sanitize_filename};

pub const PROGRESS_THROTTLE: Duration = Duration::from_millis(300);

pub struct DownloadCtx {
    pub client: reqwest::Client,
    pub cookie: String,
    pub referer: String,
    pub concurrency: usize,
    pub include_date: bool,
    pub use_mix_folder: bool,
    /// 下载根目录（`<设置路径>/作品下载`）
    pub root: PathBuf,
    /// 点赞/收藏后缀，用于目录名
    pub folder_suffix: String,
}

impl DownloadCtx {
    /// 该任务所属的用户目录：`<root>/<昵称>-<抖音号><后缀>`。
    ///
    /// 昵称为空时退回 `Douyin_User`，保证永远有可写目录。
    pub fn user_dir(&self, row: &TaskRow) -> PathBuf {
        let nick = if row.owner_nickname.is_empty() {
            "Douyin_User"
        } else {
            &row.owner_nickname
        };
        let mut name = if row.owner_unique_id.is_empty() {
            nick.to_string()
        } else {
            format!("{nick}-{}", row.owner_unique_id)
        };
        name.push_str(&self.folder_suffix);
        self.root.join(sanitize_filename(&name, 150))
    }

    /// 任务在用户目录内的相对路径。
    pub fn rel_path(&self, row: &TaskRow) -> PathBuf {
        let mix = if self.use_mix_folder && !row.mix.is_empty() {
            Some(row.mix.as_str())
        } else {
            None
        };
        build_expected_filename(
            &row.desc,
            &row.ext,
            row.is_image,
            mix,
            &row.date,
            self.include_date,
        )
    }

    /// 完整目标路径。
    pub fn target_path(&self, row: &TaskRow) -> PathBuf {
        self.user_dir(row).join(self.rel_path(row))
    }
}

/// 拆分「已存在（跳过）」与「待下载」。按任务各自的用户目录判断。
pub fn plan_targets(rows: &[TaskRow], ctx: &DownloadCtx) -> (Vec<TaskRow>, Vec<TaskRow>) {
    let mut todo = Vec::new();
    let mut already = Vec::new();
    for r in rows {
        if ctx.target_path(r).exists() {
            already.push(r.clone());
        } else {
            todo.push(r.clone());
        }
    }
    (todo, already)
}

struct TaskOutcome {
    ok: bool,
    desc: String,
    reason: Option<String>,
    /// true = 复用了已有文件（含移动/改名），没有实际下载
    reused: bool,
}

async fn download_task(
    row: TaskRow,
    ctx: Arc<DownloadCtx>,
    stop: Arc<AtomicBool>,
    reuse_index: Arc<Mutex<reuse::ReuseIndex>>,
) -> TaskOutcome {
    let label = row.desc.clone();
    let user_dir = ctx.user_dir(&row);
    let want = user_dir.join(ctx.rel_path(&row));

    // ---- 第一步：目标路径已存在 → 跳过 ----
    if want.exists() {
        return TaskOutcome {
            ok: true,
            desc: label,
            reason: None,
            reused: false,
        };
    }

    // ---- 第二步：大小预检。同目录下大小相同 → 视为同一作品 ----
    // 只发 HEAD（几 KB），不读文件内容，也不发 GET。
    if let Some(size) = reuse::remote_size(&ctx.client, &row.url, &ctx.cookie).await {
        let hit = {
            let mut idx = reuse_index.lock().expect("reuse poisoned");
            idx.claim_same_size(&user_dir, size)
        };
        if let Some(old) = hit {
            if let Err(e) = ensure_dir(&want.parent().unwrap_or(&user_dir)).await {
                eprintln!("[reuse] 建目录失败: {e}");
            }
            match reuse::move_file(&old, &want) {
                Ok(()) => {
                    reuse_index
                        .lock()
                        .expect("reuse poisoned")
                        .rekey(&row.desc, &want);
                    return TaskOutcome {
                        ok: true,
                        desc: label,
                        reason: None,
                        reused: true,
                    };
                }
                Err(e) => {
                    // 移动失败就正常下载，不因为复用失败而丢文件
                    eprintln!("[reuse] 移动失败，改为重新下载: {e}");
                }
            }
        }
    }

    // ---- 第三步：真正下载 ----
    let folder = want.parent().map(|p| p.to_path_buf()).unwrap_or(user_dir.clone());
    if let Err(e) = ensure_dir(&folder).await {
        return TaskOutcome {
            ok: false,
            desc: label,
            reason: Some(format!("创建目录失败: {e}")),
            reused: false,
        };
    }

    // 文件名主干：与 rel_path 保持一致（合集/日期都由 rel_path 决定）
    let file_stem = want
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| row.desc.clone());

    let opts = DownloadOpts {
        url: &row.url,
        folder: &folder,
        base_name: &file_stem,
        ext: &row.ext,
        url_hash: &row.url_hash,
        referer: &ctx.referer,
        cookie: &ctx.cookie,
        exact_path: Some(&want),
    };

    let stop_flag = stop.clone();
    let should_stop = move || stop_flag.load(Ordering::Relaxed);

    match download_with_retry(&ctx.client, &opts, &[], 3, &should_stop).await {
        Ok(Some(downloaded)) => {
            let size = std::fs::metadata(&downloaded).map(|m| m.len()).unwrap_or(0);
            reuse_index
                .lock()
                .expect("reuse poisoned")
                .register(&row.desc, &downloaded, size, &user_dir);
            TaskOutcome {
                ok: true,
                desc: label,
                reason: None,
                reused: false,
            }
        }
        // 用户中止不算失败，也不该进失败明细
        Ok(None) => TaskOutcome {
            ok: false,
            desc: label,
            reason: None,
            reused: false,
        },
        Err(e) => TaskOutcome {
            ok: false,
            desc: label,
            reason: Some(e.to_string()),
            reused: false,
        },
    }
}

/// 执行下载队列。
pub async fn run_queue<F, S>(
    rows: Vec<TaskRow>,
    ctx: Arc<DownloadCtx>,
    should_stop: S,
    emit_progress: F,
) -> AppResult<DownloadSummary>
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
    S: Fn() -> bool + Send + Sync + Clone + 'static,
{
    let total = rows.len() as u32;
    emit_progress(ProgressEvent {
        mode: String::new(),
        completed: 0,
        total,
    });

    // 身份标记：每个用户目录一个以 sec_user_id 命名的空文件
    // （昵称会变，目录名跟着变，靠标记认回同一账号）
    {
        let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        for r in &rows {
            let dir = ctx.user_dir(r);
            if seen.insert(dir.clone()) {
                if let Err(e) = marker::ensure(&dir, &r.owner_sec_uid) {
                    eprintln!("[marker] 写身份标记失败: {e}");
                }
            }
        }
    }

    let reuse_index = Arc::new(Mutex::new(reuse::ReuseIndex::new()));
    let sem = Arc::new(Semaphore::new(ctx.concurrency.max(1)));
    let completed = Arc::new(AtomicU32::new(0));
    let ok = Arc::new(AtomicU32::new(0));
    let failed = Arc::new(AtomicU32::new(0));
    let reused = Arc::new(AtomicU32::new(0));
    let cancelled = Arc::new(AtomicBool::new(false));

    let failures: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    let emit = Arc::new(emit_progress);
    let last_emit = Arc::new(Mutex::new(Instant::now() - PROGRESS_THROTTLE));

    let bump = {
        let completed = completed.clone();
        let ok = ok.clone();
        let failed = failed.clone();
        let reused = reused.clone();
        let emit = emit.clone();
        let last_emit = last_emit.clone();
        Arc::new(move |success: bool, was_reused: bool| {
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            if success {
                ok.fetch_add(1, Ordering::Relaxed);
                if was_reused {
                    reused.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                failed.fetch_add(1, Ordering::Relaxed);
            }
            let should = {
                let mut last = last_emit.lock().expect("throttle poisoned");
                if last.elapsed() >= PROGRESS_THROTTLE || done == total {
                    *last = Instant::now();
                    true
                } else {
                    false
                }
            };
            if should {
                emit(ProgressEvent {
                    mode: String::new(),
                    completed: done,
                    total,
                });
            }
        })
    };

    let mut set: JoinSet<()> = JoinSet::new();
    let stop_flag = Arc::new(AtomicBool::new(false));

    for row in rows {
        if should_stop() {
            cancelled.store(true, Ordering::Relaxed);
            break;
        }
        let permit = match sem.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => break,
        };
        if should_stop() {
            cancelled.store(true, Ordering::Relaxed);
            break;
        }
        let ctx = ctx.clone();
        let bump = bump.clone();
        let stop_flag = stop_flag.clone();
        let failures = failures.clone();
        let reuse_index = reuse_index.clone();
        set.spawn(async move {
            let _permit = permit;
            let out = download_task(row, ctx, stop_flag, reuse_index).await;
            if let Some(reason) = &out.reason {
                failures
                    .lock()
                    .expect("failures poisoned")
                    .push((out.desc.clone(), reason.clone()));
            }
            bump(out.ok, out.reused);
        });
    }

    let watcher = {
        let stop_flag = stop_flag.clone();
        let cancelled = cancelled.clone();
        tokio::spawn(async move {
            loop {
                if should_stop() {
                    stop_flag.store(true, Ordering::Relaxed);
                    cancelled.store(true, Ordering::Relaxed);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(120)).await;
            }
        })
    };

    while set.join_next().await.is_some() {}

    watcher.abort();

    let done = completed.load(Ordering::Relaxed);
    emit(ProgressEvent {
        mode: String::new(),
        completed: done,
        total,
    });

    let failures = failures.lock().expect("failures poisoned").clone();

    Ok(DownloadSummary {
        mode: String::new(),
        ok: ok.load(Ordering::Relaxed),
        skipped: reused.load(Ordering::Relaxed),
        failed: failed.load(Ordering::Relaxed),
        stopped: cancelled.load(Ordering::Relaxed),
        failures,
    })
}
