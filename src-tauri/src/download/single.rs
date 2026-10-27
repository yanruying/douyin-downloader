//! 单文件下载：临时文件、断点续传与原子落盘。
//!
//! 行为逐条保留：`.tmp` 临时文件、`Range` 续传、`416` 视为已完成、
//! `206` 追加写 / `200` 重写、`rename` 原子落盘、失败保留 `.tmp`。

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};
use crate::paths::{ensure_dir, unique_path};

pub struct DownloadOpts<'a> {
    pub url: &'a str,
    pub folder: &'a Path,
    pub base_name: &'a str,
    pub ext: &'a str,
    pub url_hash: &'a str,
    pub referer: &'a str,
    pub cookie: &'a str,
    /// 指定落盘绝对路径。
    ///
    /// 去重逻辑需要在下载**前**就知道最终路径，否则 `unique_path` 会因
    /// 目录里已有同名文件而生成 `_1` 后缀，导致「移到期望位置」永远匹配不上。
    pub exact_path: Option<&'a Path>,
}

pub enum Outcome {
    Saved(PathBuf),
    /// 用户中止——`.tmp` 保留，下次可续传
    Stopped,
}

/// 单次下载尝试。
pub async fn download_one(
    client: &reqwest::Client,
    opts: &DownloadOpts<'_>,
    should_stop: &(dyn Fn() -> bool + Send + Sync),
) -> AppResult<Outcome> {
    ensure_dir(opts.folder).await?;
    let path = match opts.exact_path {
        Some(p) => p.to_path_buf(),
        None => unique_path(opts.folder, opts.base_name, opts.ext, opts.url_hash),
    };
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default()
    ));

    // 断点续传：已有 .tmp 就发 Range
    let mut existing: u64 = 0;
    if let Ok(meta) = tokio::fs::metadata(&tmp).await {
        existing = meta.len();
    }

    let mut req = client
        .get(opts.url)
        .header("Referer", opts.referer)
        .header("User-Agent", crate::douyin::api::UA);
    if !opts.cookie.is_empty() {
        req = req.header("Cookie", opts.cookie);
    }
    if existing > 0 {
        req = req.header("Range", format!("bytes={existing}-"));
    }

    let resp = req.send().await.map_err(AppError::from)?;
    let status = resp.status();

    // 416：请求范围不可满足 = 文件已完整
    if status.as_u16() == 416 {
        if tokio::fs::rename(&tmp, &path).await.is_ok() {
            return Ok(Outcome::Saved(path));
        }
        return Err(AppError::msg("416 但临时文件改名失败"));
    }
    if status.as_u16() != 200 && status.as_u16() != 206 {
        return Err(AppError::msg(format!("下载失败 HTTP {status}")));
    }

    // 206 续写，200 重写（服务器不支持 Range 时丢弃旧 .tmp）
    let append = status.as_u16() == 206;
    if !append && existing > 0 {
        let _ = tokio::fs::remove_file(&tmp).await;
        existing = 0;
    }

    let mut file = if append {
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&tmp)
            .await?
    } else {
        tokio::fs::File::create(&tmp).await?
    };

    let mut stream = resp.bytes_stream();
    let mut written = existing;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(AppError::from)?;
        if !bytes.is_empty() {
            file.write_all(&bytes).await?;
            written += bytes.len() as u64;
        }
        // 每块之后检查停止信号，让「停止下载」能及时生效
        if should_stop() {
            file.flush().await?;
            return Ok(Outcome::Stopped);
        }
    }
    file.flush().await?;
    drop(file);

    if written == 0 {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(AppError::msg("响应为空"));
    }

    tokio::fs::rename(&tmp, &path).await?;
    Ok(Outcome::Saved(path))
}

/// 带重试的单文件下载（对应 `_download_with_retry`）。
/// `alt_urls` 是同一作品的其它码率地址，第 n 次重试换下一个。
pub async fn download_with_retry(
    client: &reqwest::Client,
    opts: &DownloadOpts<'_>,
    alt_urls: &[String],
    max_retries: u32,
    should_stop: &(dyn Fn() -> bool + Send + Sync),
) -> AppResult<Option<PathBuf>> {
    let mut last_err: Option<AppError> = None;
    for attempt in 0..=max_retries {
        if should_stop() {
            return Ok(None);
        }
        let mut o = DownloadOpts {
            url: opts.url,
            folder: opts.folder,
            base_name: opts.base_name,
            ext: opts.ext,
            url_hash: opts.url_hash,
            referer: opts.referer,
            cookie: opts.cookie,
            exact_path: opts.exact_path,
        };
        if attempt > 0 && (attempt as usize) <= alt_urls.len() {
            o.url = &alt_urls[attempt as usize - 1];
        }

        match download_one(client, &o, should_stop).await {
            Ok(Outcome::Saved(p)) => return Ok(Some(p)),
            Ok(Outcome::Stopped) => return Ok(None),
            Err(e) => {
                last_err = Some(e);
                if attempt < max_retries {
                    let delay = (1u64 << attempt).min(10);
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| AppError::msg("下载失败")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmp_name_appends_suffix() {
        let p = Path::new("C:/x/a.mp4");
        let tmp = p.with_extension(format!(
            "{}.tmp",
            p.extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        assert_eq!(tmp.to_string_lossy().replace('\\', "/"), "C:/x/a.mp4.tmp");
    }
}
