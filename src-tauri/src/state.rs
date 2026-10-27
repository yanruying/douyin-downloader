//! 全局状态。所有跨命令共享的东西都在这里，命令层只读不构造。
//!
//! 两个关键设计：
//!
//! 1. **任务与进度都在 Rust 进程内**，不放 Vue 组件。组件卸载/路由切换不影响后端。
//! 2. **每个取数模式各有一份独立状态**（`ModeState`）：主页/点赞/收藏/单作品
//!    四页的列表、进度、运行标志互不干扰，切换页面看到的是各自的列表。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use crate::douyin::DouyinClient;
use crate::models::{FetchMode, ModeState, ProfileInfo, Settings};

pub struct AppState {
    pub douyin: DouyinClient,
    pub settings: RwLock<Settings>,
    pub app_dir: PathBuf,
    /// 每个模式一份状态（唯一数据源，前端表格只是它的投影）
    pub modes: RwLock<HashMap<String, ModeState>>,
    /// 抓取/下载各自的停止标志，按模式分开
    pub fetch_stop: RwLock<HashMap<String, Arc<AtomicBool>>>,
    pub download_stop: RwLock<HashMap<String, Arc<AtomicBool>>>,
    pub profile: RwLock<ProfileInfo>,
    pub login_stop: Arc<AtomicBool>,
    pub speed_stop: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(app_dir: PathBuf) -> Self {
        let settings = crate::config::load(&app_dir);
        let profile = ProfileInfo {
            nickname: settings.nickname.clone(),
            unique_id: settings.unique_id.clone(),
            avatar_url: settings.avatar_url.clone(),
            sec_uid: settings.sec_uid.clone(),
        };

        let mut modes = HashMap::new();
        let mut fetch_stop = HashMap::new();
        let mut download_stop = HashMap::new();
        for m in FetchMode::all() {
            let slug = m.slug().to_string();
            modes.insert(slug.clone(), ModeState::new(m));
            fetch_stop.insert(slug.clone(), Arc::new(AtomicBool::new(false)));
            download_stop.insert(slug, Arc::new(AtomicBool::new(false)));
        }

        Self {
            douyin: DouyinClient::new().expect("build douyin client"),
            settings: RwLock::new(settings),
            app_dir,
            modes: RwLock::new(modes),
            fetch_stop: RwLock::new(fetch_stop),
            download_stop: RwLock::new(download_stop),
            profile: RwLock::new(profile),
            login_stop: Arc::new(AtomicBool::new(false)),
            speed_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings.read().expect("settings poisoned").clone()
    }

    pub fn save_settings(&self, s: Settings) -> crate::error::AppResult<()> {
        crate::config::save(&self.app_dir, &s)?;
        *self.settings.write().expect("settings poisoned") = s;
        Ok(())
    }

    /// 某个模式的快照。
    pub fn mode(&self, mode: FetchMode) -> ModeState {
        self.modes
            .read()
            .expect("modes poisoned")
            .get(mode.slug())
            .cloned()
            .unwrap_or_else(|| ModeState::new(mode))
    }

    pub fn all_modes(&self) -> Vec<ModeState> {
        let g = self.modes.read().expect("modes poisoned");
        FetchMode::all()
            .iter()
            .filter_map(|m| g.get(m.slug()).cloned())
            .collect()
    }

    /// 修改某模式的状态。
    pub fn with_mode<R>(&self, mode: FetchMode, f: impl FnOnce(&mut ModeState) -> R) -> R {
        let mut g = self.modes.write().expect("modes poisoned");
        let st = g
            .entry(mode.slug().to_string())
            .or_insert_with(|| ModeState::new(mode));
        f(st)
    }

    /// 清空某模式的列表与进度（不动其它模式）。
    pub fn reset_mode(&self, mode: FetchMode) {
        self.with_mode(mode, |st| {
            st.tasks.clear();
            st.summary = Default::default();
            st.progress.mode = mode.slug().to_string();
            st.progress.completed = 0;
            st.progress.total = 0;
            st.fetching = false;
        });
        self.fetch_flag(mode).store(false, Ordering::Relaxed);
    }

    pub fn fetch_flag(&self, mode: FetchMode) -> Arc<AtomicBool> {
        self.fetch_stop
            .read()
            .expect("fetch_stop poisoned")
            .get(mode.slug())
            .cloned()
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)))
    }

    pub fn download_flag(&self, mode: FetchMode) -> Arc<AtomicBool> {
        self.download_stop
            .read()
            .expect("download_stop poisoned")
            .get(mode.slug())
            .cloned()
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)))
    }

    /// 是否有任意模式正在抓取或下载（关闭窗口/退出前要检查）。
    pub fn any_busy(&self) -> bool {
        self.modes
            .read()
            .expect("modes poisoned")
            .values()
            .any(|m| m.fetching || m.downloading)
    }

    pub fn downloading_count(&self) -> usize {
        self.modes
            .read()
            .expect("modes poisoned")
            .values()
            .filter(|m| m.downloading)
            .count()
    }

    /// 停止所有模式的任务（退出前调用）。
    pub fn stop_all(&self) {
        for m in FetchMode::all() {
            self.fetch_flag(m).store(true, Ordering::Relaxed);
            self.download_flag(m).store(true, Ordering::Relaxed);
        }
    }

    pub fn profile_snapshot(&self) -> ProfileInfo {
        self.profile.read().expect("profile poisoned").clone()
    }

    /// 缓存登录用户信息到内存与配置。
    pub fn set_profile(&self, p: ProfileInfo) {
        *self.profile.write().expect("profile poisoned") = p.clone();
        let mut s = self.settings_snapshot();
        s.nickname = p.nickname;
        s.unique_id = p.unique_id;
        s.avatar_url = p.avatar_url;
        s.sec_uid = p.sec_uid;
        let _ = crate::config::save(&self.app_dir, &s);
        *self.settings.write().expect("settings poisoned") = s;
    }

    pub fn clear_profile(&self) {
        self.set_profile(ProfileInfo::default());
    }

    /// 按 id 子集取某模式的任务（前端勾选后下载）。
    pub fn tasks_by_ids(&self, mode: FetchMode, ids: &[String]) -> Vec<crate::models::TaskRow> {
        let st = self.mode(mode);
        if ids.is_empty() {
            return st.tasks;
        }
        let set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        st.tasks
            .into_iter()
            .filter(|t| set.contains(t.id.as_str()))
            .collect()
    }
}
