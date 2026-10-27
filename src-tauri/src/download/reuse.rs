//! 大小预检复用：不发 GET，只比对 Content-Length。
//!
//! 为什么不读内容哈希：一个 50MB 视频读一遍要几十毫秒到几百毫秒，
//! 几百个作品累积起来很可观；而抖音不同作品大小精确到字节相同的概率极低，
//! 同一作品无论怎么改设置大小恒定。所以「同大小视为同一文件」足够可靠。
//!
//! 命中后的行为按当前设置执行：目标位置不同就移动（等价于改名/换目录），
//! 这才是「开关合集目录 / 日期前缀后不重新下载」的实现方式。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use reqwest::Client;

/// 只取远端文件大小。HEAD 不被支持时返回 None（调用方会正常下载）。
pub async fn remote_size(client: &Client, url: &str, cookie: &str) -> Option<u64> {
    let mut req = client
        .head(url)
        .header("User-Agent", crate::douyin::api::UA)
        .header("Referer", "https://www.douyin.com/");
    if !cookie.is_empty() {
        req = req.header("Cookie", cookie);
    }
    // HEAD 失败或没给长度都是常见情况，静默回退到正常下载即可
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.headers()
        .get(reqwest::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
}

/// 复用索引：记忆「本次已见过的 (目录, 大小) → 路径」。
pub struct ReuseIndex {
    /// (用户目录, 文件大小) → 路径
    by_size: HashMap<(PathBuf, u64), PathBuf>,
    /// 任务描述 → 落盘路径（同一批次内后续判断）
    by_desc: HashMap<String, PathBuf>,
}

impl ReuseIndex {
    pub fn new() -> Self {
        Self {
            by_size: HashMap::new(),
            by_desc: HashMap::new(),
        }
    }

    /// 在指定目录（含子目录）里找大小完全相同的文件。
    ///
    /// 只看已登记的条目 + 首次访问时扫描一次磁盘，
    /// 避免每个任务都递归遍历目录（几百个任务会变成 O(n²)）。
    /// 认领一个同大小文件：命中后立即从索引移除。
    ///
    /// 并发任务可能同时判定「这个文件可复用」。若不认领，
    /// 后到的任务会移动一个已被移走的路径，报「找不到文件」并转为重新下载
    /// 否则并发任务会争抢同一个源文件，后到者移动失败并退化为重新下载。
    pub fn claim_same_size(&mut self, user_dir: &Path, size: u64) -> Option<PathBuf> {
        let key = (user_dir.to_path_buf(), size);

        // 先查已登记的
        if let Some(p) = self.by_size.get(&key).cloned() {
            if p.exists() {
                self.by_size.remove(&key);
                return Some(p);
            }
            self.by_size.remove(&key);
        }

        // 首次见到这个目录：扫一遍登记
        let scan_key = format!("scanned:{}", user_dir.display());
        if !self.by_desc.contains_key(&scan_key) {
            self.scan(user_dir);
            self.by_desc.insert(scan_key, PathBuf::new());
        }
        // 认领：取出即删除
        let hit = self.by_size.get(&key).cloned();
        if hit.is_some() {
            self.by_size.remove(&key);
        }
        hit.filter(|p| p.exists())
    }

    fn scan(&mut self, root: &Path) {
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                // 半成品与标记文件不参与复用
                if name.ends_with(".tmp") || name.starts_with('.') {
                    continue;
                }
                if let Ok(meta) = std::fs::metadata(&p) {
                    if meta.len() > 0 {
                        self.by_size
                            .insert((root.to_path_buf(), meta.len()), p.clone());
                    }
                }
            }
        }
    }

    /// 登记一个新落盘的文件。
    pub fn register(&mut self, desc: &str, path: &Path, size: u64, user_dir: &Path) {
        // 登记到用户目录键下，供同批次后续任务复用
        self.by_size
            .insert((user_dir.to_path_buf(), size), path.to_path_buf());
        self.by_desc.insert(desc.to_string(), path.to_path_buf());
    }

    /// 复用后更新映射，避免同批次后续任务又去找旧路径。
    pub fn rekey(&mut self, desc: &str, new_path: &Path) {
        self.by_desc.insert(desc.to_string(), new_path.to_path_buf());
    }
}

impl Default for ReuseIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// 移动文件；跨盘时退回「复制 + 删除」。
pub fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if from == to {
        return Ok(());
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dy_reuse_{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn finds_file_with_same_size_in_subdir() {
        let dir = tmp("sub");
        let sub = dir.join("某个合集");
        std::fs::create_dir_all(&sub).unwrap();
        let f = sub.join("2025-01-01_作品.mp4");
        std::fs::write(&f, vec![b'x'; 1234]).unwrap();

        let mut idx = ReuseIndex::new();
        let hit = idx.claim_same_size(&dir, 1234);
        assert_eq!(hit, Some(f), "子目录里同大小文件应被找到");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn different_size_is_not_matched() {
        let dir = tmp("neg");
        std::fs::write(dir.join("a.mp4"), vec![b'x'; 100]).unwrap();
        let mut idx = ReuseIndex::new();
        assert!(idx.claim_same_size(&dir, 101).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignores_tmp_and_dotfiles() {
        let dir = tmp("skip");
        std::fs::write(dir.join("a.mp4.tmp"), vec![b'x'; 500]).unwrap();
        std::fs::write(dir.join(".sec_uid_X"), vec![b'x'; 500]).unwrap();
        let mut idx = ReuseIndex::new();
        assert!(
            idx.claim_same_size(&dir, 500).is_none(),
            "半成品与标记文件不应参与复用"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn move_file_renames_within_same_dir() {
        let dir = tmp("move");
        let a = dir.join("旧名.mp4");
        let b = dir.join("新名.mp4");
        std::fs::write(&a, b"data").unwrap();
        move_file(&a, &b).unwrap();
        assert!(!a.exists());
        assert!(b.exists());
        assert_eq!(std::fs::read(&b).unwrap(), b"data");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn move_file_creates_target_dir() {
        let dir = tmp("move_dir");
        let a = dir.join("x.mp4");
        std::fs::write(&a, b"data").unwrap();
        let b = dir.join("新合集").join("x.mp4");
        move_file(&a, &b).unwrap();
        assert!(b.exists(), "应自动创建目标目录");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
