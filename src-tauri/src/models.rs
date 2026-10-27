use serde::{Deserialize, Serialize};

/// 单个作品行，前端 el-table 直接消费的扁平结构。
/// 抖音原始 JSON 不出 `douyin` 模块。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    pub id: String,
    pub aweme_id: String,
    /// 所属账号：批量获取多个用户时，必须按作品归属分目录，
    /// 否则所有用户的作品会堆进同一个文件夹。
    #[serde(default)]
    pub owner_nickname: String,
    #[serde(default)]
    pub owner_unique_id: String,
    #[serde(default)]
    pub owner_sec_uid: String,
    pub date: String,
    pub desc: String,
    pub mix: String,
    /// "video" | "image" | "live"
    pub kind: String,
    /// 实际下载地址（CDN 直链，url_list[0]）
    pub url: String,
    /// 导出直链用的简短形态（url_list 里带 video_id/file_id 的那条，
    /// 通常是最后一个）。与 `url` 分开存：下载要 CDN 直链，导出要可复用的 play 链接。
    #[serde(default)]
    pub play_url: String,
    pub ext: String,
    pub is_image: bool,
    pub url_hash: String,
}

/// 四种取数模式。`Single` 走 aweme/detail，另外三种走列表接口分页。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FetchMode {
    Single,
    Post,
    Favorite,
    Collect,
}

impl FetchMode {
    pub fn label(self) -> &'static str {
        match self {
            FetchMode::Single => "单作品",
            FetchMode::Post => "主页作品",
            FetchMode::Favorite => "点赞作品",
            FetchMode::Collect => "收藏作品",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            FetchMode::Single => "single",
            FetchMode::Post => "post",
            FetchMode::Favorite => "favorite",
            FetchMode::Collect => "collect",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "single" => Some(FetchMode::Single),
            "post" => Some(FetchMode::Post),
            "favorite" => Some(FetchMode::Favorite),
            "collect" => Some(FetchMode::Collect),
            _ => None,
        }
    }

    pub fn all() -> [FetchMode; 4] {
        [
            FetchMode::Single,
            FetchMode::Post,
            FetchMode::Favorite,
            FetchMode::Collect,
        ]
    }

    /// 点赞/收藏的目录名要带后缀，避免同一个用户三种列表互相覆盖。
    pub fn folder_suffix(self) -> &'static str {
        match self {
            FetchMode::Favorite => "-like",
            FetchMode::Collect => "-collect",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEntry {
    pub username: String,
    /// 只存 sec_uid，链接在需要时拼出来，避免脏 URL 进配置
    #[serde(default)]
    pub sec_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub path: String,
    pub cookie: String,
    pub use_mix_folder: bool,
    pub include_date_in_filename: bool,
    pub auto_select_after_fetch: bool,
    pub add_title_when_export_urls: bool,
    pub threads: u32,
    pub users: Vec<UserEntry>,
    /// 登录用户缓存，用于侧栏头像与昵称，避免每次启动都请求接口。
    pub nickname: String,
    pub unique_id: String,
    pub avatar_url: String,
    pub sec_uid: String,
    /// 关闭窗口时最小化到托盘（勾选后不再弹确认框）
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: String::new(),
            cookie: String::new(),
            use_mix_folder: true,
            include_date_in_filename: true,
            auto_select_after_fetch: true,
            add_title_when_export_urls: false,
            threads: 4,
            users: Vec::new(),
            nickname: String::new(),
            unique_id: String::new(),
            avatar_url: String::new(),
            sec_uid: String::new(),
            close_to_tray: false,
        }
    }
}

/// 一次抓取的元信息，供后续下载目录名与导出使用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchSummary {
    pub nickname: String,
    pub unique_id: String,
    /// 作品所属账号的 sec_user_id。下载时用它写身份标记文件，
    /// 保证「谁的目录里就该有谁的标记」。
    #[serde(default)]
    pub sec_uid: String,
    pub count: usize,
    pub mode: String,
}

/// 单页增量事件：带上本页解析好的行，前端可以边抓边显示。
/// `mode` 让前端知道该追加到哪个页面。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchPageEvent {
    pub mode: String,
    pub page: u32,
    pub added: u32,
    /// 累计已解析条数
    pub total: u32,
    pub tasks: Vec<TaskRow>,
}

/// 进度事件按模式区分：每个页面有自己的进度条。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub mode: String,
    pub completed: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedEvent {
    pub bytes_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub level: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSummary {
    pub mode: String,
    pub ok: u32,
    pub skipped: u32,
    pub failed: u32,
    pub stopped: bool,
    /// 失败明细：(作品描述, 失败原因)。成功/跳过不记，避免日志刷屏。
    #[serde(default)]
    pub failures: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CookieStatus {
    Empty,
    Ok,
    Expired,
}

/// 登录用户信息（昵称 / unique_id / 头像），侧栏头像用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub nickname: String,
    pub unique_id: String,
    pub avatar_url: String,
    /// 自己的 sec_uid：不填链接时用它抓取本人作品
    pub sec_uid: String,
}

/// Excel 导出需要的统计字段。
///
/// 不保留完整 aweme JSON（几十 MB），只抽这 7 个数字/字符串。
/// 单条约 80 字节，435 条约 35KB —— 相对完整 JSON 可以忽略不计。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwemeStat {
    pub aweme_id: String,
    pub desc: String,
    pub create_time: i64,
    pub mix: String,
    /// 是否图集（决定链接用 /note/ 还是 /video/）
    pub is_album: bool,
    pub digg: i64,
    pub comment: i64,
    pub collect: i64,
    pub share: i64,
    pub recommend: i64,
    /// 视频时长（毫秒）
    pub duration: i64,
}

/// 单个页面的完整状态（每个 mode 一份，互不干扰）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeState {
    pub mode: String,
    pub tasks: Vec<TaskRow>,
    pub summary: FetchSummary,
    pub fetching: bool,
    pub downloading: bool,
    pub progress: ProgressEvent,
    pub last_download: Option<DownloadSummary>,
    pub url: String,
    /// 供 Excel 导出用的精简统计（与 tasks 等长、一一对应）
    #[serde(default)]
    pub stats: Vec<AwemeStat>,
}

impl ModeState {
    pub fn new(mode: FetchMode) -> Self {
        Self {
            mode: mode.slug().to_string(),
            progress: ProgressEvent {
                mode: mode.slug().to_string(),
                completed: 0,
                total: 0,
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_slug_roundtrip() {
        for m in FetchMode::all() {
            assert_eq!(FetchMode::from_slug(m.slug()), Some(m));
        }
        assert_eq!(FetchMode::from_slug("bogus"), None);
    }

    #[test]
    fn folder_suffixes_are_distinct() {
        // 三种列表必须各自独立目录，否则同名文件互相覆盖
        let mut s: Vec<&str> = [FetchMode::Post, FetchMode::Favorite, FetchMode::Collect]
            .iter()
            .map(|m| m.folder_suffix())
            .collect();
        let n = s.len();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), n);
    }

    #[test]
    fn mode_state_defaults_to_own_mode() {
        let st = ModeState::new(FetchMode::Favorite);
        assert_eq!(st.mode, "favorite");
        assert_eq!(st.progress.mode, "favorite");
        assert!(st.tasks.is_empty());
        assert!(!st.fetching);
    }
}
