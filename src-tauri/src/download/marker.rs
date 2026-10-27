//! 用户身份标记文件。
//!
//! 目的：用户目录名是 `<昵称>-<抖音号>`，**昵称会变**（改名后目录名也就变了）。
//! 在目录里放一个以 **sec_user_id** 命名的空文件，就能在昵称变更后认出
//! 「这个目录属于同一个账号」，从而沿用已下载的文件。
//!
//! 文件名形如 `.sec_uid_MS4wLjABAAAA...`。
//! sec_user_id 就是主页 URL 里那串 `MS4wLjAB…`，只含 `[A-Za-z0-9_-]`，
//! 天然是合法文件名，不需要额外转义。

use std::path::{Path, PathBuf};

pub const MARKER_PREFIX: &str = ".sec_uid_";

/// 标记文件路径。
///
/// sec_user_id 为空时不写标记（避免产生 `.sec_uid_` 这种无意义文件）。
pub fn path_for(user_dir: &Path, sec_uid: &str) -> Option<PathBuf> {
    let id = sec_uid.trim();
    if id.is_empty() {
        return None;
    }
    // 只保留合法字符，防止拼接出越界路径
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Some(user_dir.join(format!("{MARKER_PREFIX}{safe}")))
}

/// 写入标记文件。已存在则跳过（幂等）。
pub fn ensure(user_dir: &Path, sec_uid: &str) -> std::io::Result<()> {
    let Some(p) = path_for(user_dir, sec_uid) else {
        return Ok(());
    };
    if p.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(user_dir)?;
    std::fs::write(&p, b"")
}

/// 该目录是否属于此 sec_user_id。
pub fn belongs_to(user_dir: &Path, sec_uid: &str) -> bool {
    path_for(user_dir, sec_uid).is_some_and(|p| p.exists())
}

/// 在 root 下的同级目录里找出属于该 sec_user_id 的那个。
///
/// 用于「昵称变了、按新目录名找不到旧目录」：
/// 逐个扫描同级目录的标记文件，命中就复用旧目录。
pub fn find_user_dir(root: &Path, sec_uid: &str) -> Option<PathBuf> {
    let target = path_for(Path::new(""), sec_uid)?;
    let marker_name = target.file_name()?.to_os_string();

    let rd = std::fs::read_dir(root).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && p.join(&marker_name).exists() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dy_marker_{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const SEC: &str = "MS4wLjABAAAAexample_sec_uid_for_test_only";

    #[test]
    fn marker_uses_sec_user_id() {
        let d = tmp("name");
        ensure(&d, SEC).unwrap();
        assert!(d.join(format!(".sec_uid_{SEC}")).exists());
        // 不应再出现按抖音号命名的旧形态
        assert!(!d.join(".douyin_id_58203799496").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn marker_is_empty_and_idempotent() {
        let d = tmp("empty");
        ensure(&d, SEC).unwrap();
        ensure(&d, SEC).unwrap();
        let p = path_for(&d, SEC).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().len(), 0, "标记文件应为空");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn empty_id_writes_nothing() {
        let d = tmp("noid");
        ensure(&d, "").unwrap();
        ensure(&d, "   ").unwrap();
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn belongs_to_matches_exact_id() {
        let d = tmp("belongs");
        ensure(&d, SEC).unwrap();
        assert!(belongs_to(&d, SEC));
        assert!(!belongs_to(&d, "MS4wLjABOTHER"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn finds_dir_after_nickname_change() {
        let root = tmp("find");
        let old = root.join("旧昵称-58203799496");
        std::fs::create_dir_all(&old).unwrap();
        ensure(&old, SEC).unwrap();

        let hit = find_user_dir(&root, SEC);
        assert_eq!(hit, Some(old), "昵称变更后应靠标记找回旧目录");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_path_separators_in_id() {
        let d = tmp("safe");
        // 异常字符不应逃出目录
        let p = path_for(&d, "a/b\\c").unwrap();
        assert_eq!(p.parent(), Some(d.as_path()));
        let _ = std::fs::remove_dir_all(&d);
    }
}
