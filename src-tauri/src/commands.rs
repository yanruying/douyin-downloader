//! 所有 `#[tauri::command]`。纯薄壳：参数转换 → 调业务模块 → 返回/emit。
//!
//! 设计要点：
//! - 长任务（fetch / download）在 `spawn` 里跑，命令立即返回 → 切页不影响任务。
//! - 每个模式（single/post/favorite/collect）有独立状态与独立停止标志。
//! - 事件都带 `mode` 字段，前端据此把增量投递到对应页面。

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::download::DownloadCtx;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;

// ---------- 事件名（前端 events.ts 必须与此一致） ----------
pub const EV_PROGRESS: &str = "progress:update";
pub const EV_SPEED: &str = "speed:update";
pub const EV_FETCH_PAGE: &str = "fetch:page";
pub const EV_LOG: &str = "fetch:log";
pub const EV_FETCH_DONE: &str = "fetch:done";
/// 抓取开始后、分页之前就推送用户名：状态栏需要它
pub const EV_PROFILE_FETCHING: &str = "fetch:profile";
pub const EV_DOWNLOAD_DONE: &str = "download:done";
pub const EV_COOKIE: &str = "cookie:status";
pub const EV_PROFILE: &str = "auth:profile";

fn log(app: &AppHandle, level: &str, text: impl Into<String>) {
    let _ = app.emit(
        EV_LOG,
        LogEvent {
            level: level.into(),
            text: text.into(),
        },
    );
}

fn parse_mode(slug: &str) -> AppResult<FetchMode> {
    FetchMode::from_slug(slug).ok_or_else(|| AppError::msg(format!("未知模式: {slug}")))
}

// ---------------- 启动 / 设置 ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitPayload {
    pub settings: Settings,
    pub app_dir: String,
    pub download_base: String,
    pub legacy_imported: bool,
    pub cookie_status: CookieStatus,
    pub profile: ProfileInfo,
    /// 四个模式各自的状态（各自独立的列表与进度）
    pub modes: Vec<ModeState>,
}

#[tauri::command]
pub async fn init_app(app: AppHandle, state: State<'_, AppState>) -> AppResult<InitPayload> {
    let imported = crate::config::import_legacy(&state.app_dir, None);
    let mut legacy_imported = false;
    if let Some(legacy) = imported {
        let current = state.settings_snapshot();
        if current.cookie.is_empty() && current.path.is_empty() {
            let mut merged = legacy;
            merged.nickname = current.nickname;
            merged.unique_id = current.unique_id;
            merged.avatar_url = current.avatar_url;
            merged.sec_uid = current.sec_uid;
            merged.close_to_tray = current.close_to_tray;
            crate::config::save(&state.app_dir, &merged)?;
            *state.settings.write().unwrap() = merged;
            legacy_imported = true;
        }
    }

    let settings = state.settings_snapshot();
    let cookie_status = if settings.cookie.is_empty() {
        CookieStatus::Empty
    } else {
        state.douyin.check_cookie(&settings.cookie).await
    };

    // Cookie 失效就把头像一起清掉，让侧栏回到登录按钮
    if cookie_status == CookieStatus::Expired || cookie_status == CookieStatus::Empty {
        if !state.profile_snapshot().avatar_url.is_empty() {
            state.clear_profile();
        }
    }

    // Cookie 有效但本地没缓存头像 → 启动时补拉一次。
    // 否则通过手动配置 Cookie 登录的用户看不到头像。
    if cookie_status == CookieStatus::Ok && state.profile_snapshot().avatar_url.is_empty() {
        match state.douyin.self_profile(&settings.cookie).await {
            Ok(p) => {
                if !p.avatar_url.is_empty() {
                    state.set_profile(p);
                }
            }
            Err(e) => log(&app, "warn", format!("获取账号信息失败: {e}")),
        }
    }

    let _ = app.emit(EV_COOKIE, cookie_status);
    let profile = state.profile_snapshot();
    let _ = app.emit(EV_PROFILE, profile.clone());

    Ok(InitPayload {
        settings,
        app_dir: state.app_dir.to_string_lossy().to_string(),
        download_base: crate::config::default_download_base()
            .to_string_lossy()
            .to_string(),
        legacy_imported,
        cookie_status,
        profile,
        modes: state.all_modes(),
    })
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings_snapshot()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<()> {
    let mut s = settings;
    s.threads = s.threads.clamp(1, 8);
    let want_tray = s.close_to_tray;
    state.save_settings(s)?;

    // 托盘随设置增删：勾选才出现，取消勾选就移除
    if want_tray {
        if let Err(e) = crate::tray::ensure_tray(&app) {
            log(&app, "warn", format!("创建托盘失败: {e}"));
        }
    } else {
        crate::tray::remove_tray(&app);
    }
    Ok(())
}

// ---------------- Cookie / 登录 ----------------

#[tauri::command]
pub async fn open_login(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    crate::auth::open_login_window(&app)?;
    state.login_stop.store(false, Ordering::Relaxed);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub cookie: String,
    pub status: CookieStatus,
    pub profile: ProfileInfo,
}

#[tauri::command]
pub async fn wait_login(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<LoginResult>> {
    let stop = state.login_stop.clone();
    let cookie = crate::auth::wait_for_login(
        app.clone(),
        std::time::Duration::from_secs(300),
        move || stop.load(Ordering::Relaxed),
    )
    .await?;

    let Some(c) = cookie else { return Ok(None) };

    let mut s = state.settings_snapshot();
    s.cookie = c.clone();
    state.save_settings(s)?;

    let status = state.douyin.check_cookie(&c).await;
    let profile = if status == CookieStatus::Ok {
        let p = state.douyin.self_profile(&c).await.unwrap_or_default();
        state.set_profile(p.clone());
        p
    } else {
        ProfileInfo::default()
    };

    let _ = app.emit(EV_COOKIE, status);
    let _ = app.emit(EV_PROFILE, profile.clone());
    crate::auth::focus_main(&app);
    Ok(Some(LoginResult {
        cookie: c,
        status,
        profile,
    }))
}

#[tauri::command]
pub fn cancel_login(app: AppHandle, state: State<'_, AppState>) {
    state.login_stop.store(true, Ordering::Relaxed);
    crate::auth::close_login_window(&app);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieState {
    pub status: CookieStatus,
    pub profile: ProfileInfo,
}

#[tauri::command]
pub async fn check_cookie(app: AppHandle, state: State<'_, AppState>) -> AppResult<CookieState> {
    let s = state.settings_snapshot();
    if s.cookie.is_empty() {
        return Ok(CookieState {
            status: CookieStatus::Empty,
            profile: state.profile_snapshot(),
        });
    }
    let status = state.douyin.check_cookie(&s.cookie).await;

    let mut profile = state.profile_snapshot();
    if status == CookieStatus::Expired {
        if !profile.avatar_url.is_empty() {
            state.clear_profile();
            profile = ProfileInfo::default();
        }
    } else if status == CookieStatus::Ok && profile.avatar_url.is_empty() {
        if let Ok(p) = state.douyin.self_profile(&s.cookie).await {
            state.set_profile(p.clone());
            profile = p;
        }
    }
    let _ = app.emit(EV_COOKIE, status);
    let _ = app.emit(EV_PROFILE, profile.clone());
    Ok(CookieState { status, profile })
}

#[tauri::command]
pub async fn set_cookie(
    app: AppHandle,
    state: State<'_, AppState>,
    cookie: String,
) -> AppResult<CookieState> {
    let mut s = state.settings_snapshot();
    s.cookie = cookie.trim().to_string();
    state.save_settings(s.clone())?;

    let status = state.douyin.check_cookie(&s.cookie).await;
    let profile = if status == CookieStatus::Ok {
        let p = state.douyin.self_profile(&s.cookie).await.unwrap_or_default();
        state.set_profile(p.clone());
        p
    } else {
        state.clear_profile();
        ProfileInfo::default()
    };
    let _ = app.emit(EV_COOKIE, status);
    let _ = app.emit(EV_PROFILE, profile.clone());
    Ok(CookieState { status, profile })
}

/// 按 sec_uid 查账号资料（用户列表添加链接时自动填最新用户名）。
#[tauri::command]
pub async fn lookup_profile(
    state: State<'_, AppState>,
    sec_uid: String,
) -> AppResult<ProfileInfo> {
    let s = state.settings_snapshot();
    if s.cookie.is_empty() {
        return Err(AppError::msg("请先登录，查询用户信息需要 Cookie"));
    }
    let id = sec_uid.trim();
    if id.is_empty() {
        return Err(AppError::msg("sec_user_id 为空"));
    }
    state.douyin.profile_by_sec_uid(id, &s.cookie).await
}

#[tauri::command]
pub fn logout(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let mut s = state.settings_snapshot();
    s.cookie.clear();
    state.save_settings(s)?;
    state.clear_profile();
    let _ = app.emit(EV_COOKIE, CookieStatus::Empty);
    let _ = app.emit(EV_PROFILE, ProfileInfo::default());
    Ok(())
}

// ---------------- 抓取 ----------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchReq {
    pub url: String,
    pub mode: FetchMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartAck {
    pub started: bool,
}

/// 启动抓取后立即返回，进度通过事件推送。`mode` 决定写到哪一页。
#[tauri::command]
pub async fn start_fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    req: FetchReq,
) -> AppResult<StartAck> {
    let settings = state.settings_snapshot();
    if settings.cookie.is_empty() {
        return Err(AppError::msg("请先登录获取 Cookie"));
    }
    if req.url.trim().is_empty() && req.mode == FetchMode::Single {
        return Err(AppError::msg("请粘贴作品链接"));
    }
    if state.mode(req.mode).fetching {
        return Err(AppError::msg("该页面已在获取中，请先停止"));
    }
    // 同一时刻只允许一个页面在抓取：并发请求不同接口更容易触发风控
    if state.all_modes().iter().any(|m| m.fetching) {
        return Err(AppError::msg("已有一个页面正在获取，请等待完成或先停止"));
    }

    state.reset_mode(req.mode);
    let mode = req.mode;
    let slug = mode.slug().to_string();
    state.with_mode(mode, |st| {
        st.fetching = true;
        st.url = req.url.clone();
    });
    state.fetch_flag(mode).store(false, Ordering::Relaxed);

    let douyin = state.douyin.clone();
    let stop = state.fetch_flag(mode);
    let app2 = app.clone();
    let url = req.url.clone();
    let cookie = settings.cookie.clone();

    tauri::async_runtime::spawn(async move {
        let stop_cb = stop.clone();
        let app_cb = app2.clone();
        let slug_cb = slug.clone();
        let app_prof = app2.clone();
        let mode_prof = mode;
        // 分页回调发生在 profile 之后，用一个共享槽把 owner 传进去
        let owner_slot: Arc<std::sync::Mutex<crate::douyin::parser::Owner>> =
            Arc::new(std::sync::Mutex::new(crate::douyin::parser::Owner::default()));
        let owner_for_page = owner_slot.clone();
        let prof_cb = move |p: &ProfileInfo| {
            // 先把昵称写进该页面的 summary，前端立刻能显示
            if let Some(st) = app_prof.try_state::<AppState>() {
                st.with_mode(mode_prof, |s| {
                    s.summary.nickname = p.nickname.clone();
                    s.summary.unique_id = p.unique_id.clone();
                    s.summary.mode = mode_prof.slug().to_string();
                });
            }
            if let Ok(mut o) = owner_slot.lock() {
                *o = crate::douyin::parser::Owner::from_profile(p);
            }
            let _ = app_prof.emit(EV_PROFILE_FETCHING, serde_json::json!({
                "mode": mode_prof.slug(),
                "nickname": p.nickname,
                "uniqueId": p.unique_id,
            }));
        };

        let result = douyin
            .fetch_all(&url, &cookie, mode, move || stop_cb.load(Ordering::Relaxed), prof_cb, |page, list| {
                let owner = owner_for_page
                    .lock()
                    .map(|o| o.clone())
                    .unwrap_or_default();
                let rows = crate::douyin::parser::parse_all(list, &owner);
                let added = rows.len() as u32;
                let _ = app_cb.emit(
                    EV_FETCH_PAGE,
                    FetchPageEvent {
                        mode: slug_cb.clone(),
                        page,
                        added,
                        total: 0,
                        tasks: rows,
                    },
                );
            })
            .await;

        let st = app2.state::<AppState>();
        match result {
            Ok((profile, awemes)) => {
                // 权威列表以 awemes 为准；每页已增量推送过，这里覆盖而非追加
                let owner = crate::douyin::parser::Owner::from_profile(&profile);
                let tasks = crate::douyin::parser::parse_all(&awemes, &owner);
                let stats = crate::douyin::parser::stats_all(&awemes);
                let count = tasks.len();
                let summary = FetchSummary {
                    nickname: profile.nickname.clone(),
                    unique_id: profile.unique_id.clone(),
                    sec_uid: profile.sec_uid.clone(),
                    count,
                    mode: slug.clone(),
                };
                st.with_mode(mode, |s| {
                    s.tasks = tasks;
                    s.stats = stats;
                    s.summary = summary.clone();
                    s.fetching = false;
                });

                // 把刚抓到的用户记进用户列表，下次可直接点开
                remember_user(&app2, &st, &profile, &url, mode);
                log(
                    &app2,
                    "info",
                    format!("{}完成：{} 个文件", mode.label(), count),
                );
                let _ = app2.emit(EV_FETCH_DONE, summary);
            }
            Err(e) => {
                st.with_mode(mode, |s| s.fetching = false);
                let stopped = stop.load(Ordering::Relaxed);
                if stopped {
                    log(&app2, "info", "抓取已停止");
                } else {
                    log(&app2, "error", format!("抓取失败: {e}"));
                }
                let _ = app2.emit(
                    EV_FETCH_DONE,
                    serde_json::json!({
                        "mode": slug,
                        "error": e.to_string(),
                        "stopped": stopped
                    }),
                );
            }
        }
    });

    Ok(StartAck { started: true })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchFetchReq {
    pub urls: Vec<String>,
    pub mode: FetchMode,
}

/// 批量抓取多个主页：串行执行，结果合并到同一页面。
#[tauri::command]
pub async fn batch_fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    req: BatchFetchReq,
) -> AppResult<StartAck> {
    let settings = state.settings_snapshot();
    if settings.cookie.is_empty() {
        return Err(AppError::msg("请先登录获取 Cookie"));
    }
    if req.urls.is_empty() {
        return Err(AppError::msg("请先勾选要批量获取的用户"));
    }
    if req.mode == FetchMode::Single {
        return Err(AppError::msg("批量获取不支持单作品模式"));
    }
    if state.all_modes().iter().any(|m| m.fetching) {
        return Err(AppError::msg("已有抓取任务在进行中"));
    }

    let mode = req.mode;
    let slug = mode.slug().to_string();
    state.reset_mode(mode);
    state.with_mode(mode, |st| st.fetching = true);
    state.fetch_flag(mode).store(false, Ordering::Relaxed);

    let douyin = state.douyin.clone();
    let stop = state.fetch_flag(mode);
    let app2 = app.clone();
    let cookie = settings.cookie.clone();
    let urls = req.urls.clone();

    tauri::async_runtime::spawn(async move {
        let mut total_count = 0usize;
        let mut ok_users = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for (i, u) in urls.iter().enumerate() {
            if stop.load(Ordering::Relaxed) {
                log(&app2, "info", "批量获取已停止");
                break;
            }
            log(
                &app2,
                "info",
                format!("批量获取 {}/{}：{}", i + 1, urls.len(), u),
            );

            let app_cb = app2.clone();
            let slug_cb = slug.clone();
            let stop_cb = stop.clone();
            // 每个用户单独一份 owner：作品必须带上自己的归属，否则批量下载会全堆一个目录
            let owner_slot: Arc<std::sync::Mutex<crate::douyin::parser::Owner>> =
                Arc::new(std::sync::Mutex::new(crate::douyin::parser::Owner::default()));
            let owner_for_prof = owner_slot.clone();
            let owner_for_page = owner_slot.clone();
            let result = douyin
                .fetch_all(
                    u,
                    &cookie,
                    mode,
                    move || stop_cb.load(Ordering::Relaxed),
                    move |p: &ProfileInfo| {
                        if let Ok(mut o) = owner_for_prof.lock() {
                            *o = crate::douyin::parser::Owner::from_profile(p);
                        }
                    },
                    |page, list| {
                        let owner = owner_for_page
                            .lock()
                            .map(|o| o.clone())
                            .unwrap_or_default();
                        let rows = crate::douyin::parser::parse_all(list, &owner);
                        let _ = app_cb.emit(
                            EV_FETCH_PAGE,
                            FetchPageEvent {
                                mode: slug_cb.clone(),
                                page,
                                added: rows.len() as u32,
                                total: 0,
                                tasks: rows,
                            },
                        );
                    },
                )
                .await;

            match result {
                Ok((profile, awemes)) => {
                    let owner = crate::douyin::parser::Owner::from_profile(&profile);
                    let rows = crate::douyin::parser::parse_all(&awemes, &owner);
                    let stats = crate::douyin::parser::stats_all(&awemes);
                    total_count += rows.len();
                    ok_users += 1;
                    let st = app2.state::<AppState>();
                    st.with_mode(mode, |s| {
                        s.tasks.extend(rows);
                        s.stats.extend(stats);
                        if s.summary.nickname.is_empty() {
                            s.summary.nickname = profile.nickname.clone();
                            s.summary.unique_id = profile.unique_id.clone();
                            s.summary.sec_uid = profile.sec_uid.clone();
                        }
                    });
                    let app_ref = app2.clone();
                    remember_user(&app_ref, &app2.state::<AppState>(), &profile, u, mode);
                }
                Err(e) => {
                    log(&app2, "error", format!("{u} 获取失败: {e}"));
                    failures.push(format!("{u}: {e}"));
                }
            }
            // 用户之间留间隔，降低风控概率
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }

        let st = app2.state::<AppState>();
        st.with_mode(mode, |s| {
            s.fetching = false;
            s.summary.count = total_count;
        });
        log(
            &app2,
            "info",
            format!(
                "批量获取完成：{ok_users}/{} 个用户成功，共 {total_count} 个文件",
                urls.len()
            ),
        );
        let _ = app2.emit(
            EV_FETCH_DONE,
            serde_json::json!({
                "mode": slug,
                "count": total_count,
                "nickname": "",
                "uniqueId": "",
            }),
        );
    });

    Ok(StartAck { started: true })
}

#[tauri::command]
pub fn stop_fetch(app: AppHandle, state: State<'_, AppState>, mode: String) {
    if let Ok(m) = parse_mode(&mode) {
        state.fetch_flag(m).store(true, Ordering::Relaxed);
        log(&app, "info", format!("已请求停止获取{}", m.label()));
    }
}

#[tauri::command]
pub fn clear_tasks(state: State<'_, AppState>, mode: String) -> AppResult<()> {
    let m = parse_mode(&mode)?;
    state.reset_mode(m);
    Ok(())
}

/// 查询后端当前状态（前端切页回来 / 热重载时用它恢复视图）。
#[tauri::command]
pub fn get_runtime(state: State<'_, AppState>) -> Vec<ModeState> {
    state.all_modes()
}

#[tauri::command]
pub fn get_tasks(state: State<'_, AppState>, mode: String) -> AppResult<Vec<TaskRow>> {
    let m = parse_mode(&mode)?;
    Ok(state.mode(m).tasks)
}

// ---------------- 下载 ----------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadReq {
    pub mode: FetchMode,
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusyState {
    pub busy: bool,
    pub downloading: bool,
    pub fetching: bool,
}

/// 启动下载后立即返回；进度与结果通过事件推送。
#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    req: DownloadReq,
) -> AppResult<BusyState> {
    let mode = req.mode;
    if state.all_modes().iter().any(|m| m.downloading) {
        return Err(AppError::msg("已有下载任务在进行中，请等待完成"));
    }
    if state.mode(mode).fetching {
        return Err(AppError::msg("正在获取作品，请先等待获取完成"));
    }

    let settings = state.settings_snapshot();
    let summary = state.mode(mode).summary;
    let selected = state.tasks_by_ids(mode, &req.ids);
    if selected.is_empty() {
        return Err(AppError::msg("请先选择要下载的作品"));
    }

    let root = crate::config::download_root(&settings);

    // 目录由每个任务自己的 owner 决定：批量获取多个用户时各进各的文件夹。
    // summary 只用于兜底（单作品模式可能拿不到 owner）。
    let fallback_owner = crate::douyin::parser::Owner {
        nickname: summary.nickname.clone(),
        unique_id: summary.unique_id.clone(),
        sec_uid: summary.sec_uid.clone(),
    };
    let selected: Vec<TaskRow> = selected
        .into_iter()
        .map(|mut t| {
            if t.owner_nickname.is_empty() {
                t.owner_nickname = fallback_owner.nickname.clone();
                t.owner_unique_id = fallback_owner.unique_id.clone();
                t.owner_sec_uid = fallback_owner.sec_uid.clone();
            }
            t
        })
        .collect();

    let ctx = Arc::new(DownloadCtx {
        client: state.douyin.client.clone(),
        cookie: settings.cookie.clone(),
        referer: "https://www.douyin.com/".into(),
        concurrency: settings.threads as usize,
        include_date: settings.include_date_in_filename,
        use_mix_folder: settings.use_mix_folder,
        root,
        folder_suffix: mode.folder_suffix().to_string(),
    });

    let (todo, already) = crate::download::plan_targets(&selected, &ctx);
    let skipped = already.len() as u32;
    // 跳过只报总数：一次几百个作品的批次里逐条打印会把真正的失败信息埋掉
    if skipped > 0 {
        log(&app, "info", format!("跳过 {skipped} 个已存在的文件"));
    }

    let slug = mode.slug().to_string();

    if todo.is_empty() {
        let done = DownloadSummary {
            mode: slug.clone(),
            ok: 0,
            skipped,
            failed: 0,
            stopped: false,
            failures: Vec::new(),
        };
        state.with_mode(mode, |s| s.last_download = Some(done.clone()));
        log(&app, "info", format!("全部已存在，跳过 {skipped} 个"));
        let _ = app.emit(EV_DOWNLOAD_DONE, done);
        return Ok(BusyState {
            busy: false,
            downloading: false,
            fetching: false,
        });
    }

    let total = todo.len() as u32;
    state.download_flag(mode).store(false, Ordering::Relaxed);
    state.with_mode(mode, |s| {
        s.downloading = true;
        s.progress = ProgressEvent {
            mode: slug.clone(),
            completed: 0,
            total,
        };
    });

    let app2 = app.clone();
    let stop = state.download_flag(mode);
    let slug2 = slug.clone();

    tauri::async_runtime::spawn(async move {
        let app_ev = app2.clone();
        let slug_ev = slug2.clone();
        let stop_q = stop.clone();

        let result = crate::download::run_queue(
            todo,
            ctx,
            move || stop_q.load(Ordering::Relaxed),
            move |p| {
                if let Some(st) = app_ev.try_state::<AppState>() {
                    st.with_mode(mode, |s| {
                        s.progress.completed = p.completed;
                        s.progress.total = p.total;
                    });
                }
                let _ = app_ev.emit(
                    EV_PROGRESS,
                    ProgressEvent {
                        mode: slug_ev.clone(),
                        completed: p.completed,
                        total: p.total,
                    },
                );
            },
        )
        .await;

        let st = app2.state::<AppState>();
        st.with_mode(mode, |s| s.downloading = false);

        match result {
            Ok(mut s) => {
                s.skipped = skipped;
                s.mode = slug2.clone();
                st.with_mode(mode, |st2| st2.last_download = Some(s.clone()));
                let msg = if s.stopped {
                    format!("下载已停止：成功 {}，失败 {}，跳过 {}", s.ok, s.failed, skipped)
                } else {
                    format!("下载完成：成功 {}，失败 {}，跳过 {}", s.ok, s.failed, skipped)
                };
                log(&app2, if s.failed > 0 { "warn" } else { "info" }, msg);

                // 失败逐个列出：这里的信息才有排查价值，必须完整
                for (desc, reason) in &s.failures {
                    let short: String = desc.chars().take(40).collect();
                    log(&app2, "error", format!("失败：{short} —— {reason}"));
                }
                let _ = app2.emit(EV_DOWNLOAD_DONE, s);
            }
            Err(e) => {
                log(&app2, "error", format!("下载异常: {e}"));
                let _ = app2.emit(
                    EV_DOWNLOAD_DONE,
                    DownloadSummary {
                        mode: slug2,
                        ok: 0,
                        skipped,
                        failed: total,
                        stopped: true,
                        failures: Vec::new(),
                    },
                );
            }
        }
    });

    Ok(BusyState {
        busy: true,
        downloading: true,
        fetching: false,
    })
}

#[tauri::command]
pub fn stop_download(app: AppHandle, state: State<'_, AppState>, mode: String) {
    if let Ok(m) = parse_mode(&mode) {
        state.download_flag(m).store(true, Ordering::Relaxed);
        log(&app, "info", "已请求停止下载，正在收尾");
    }
}

/// 供关闭窗口前查询：是否还有任务在跑。
#[tauri::command]
pub fn query_busy(state: State<'_, AppState>) -> BusyState {
    let downloading = state.all_modes().iter().any(|m| m.downloading);
    let fetching = state.all_modes().iter().any(|m| m.fetching);
    BusyState {
        busy: downloading || fetching,
        downloading,
        fetching,
    }
}

/// 抓取成功后把该用户记进用户列表，下次可直接从抽屉点开。
///
/// 规则：
/// - 只记「有主页链接」的抓取（单作品模式抓到的是作者，链接指向作品页，跳过）
/// - 按 sec_uid / url 去重，重复抓取不会堆积
/// - 最多保留 50 条，超出丢弃最旧的，避免配置无限膨胀
fn remember_user(app: &AppHandle, state: &AppState, profile: &ProfileInfo, url: &str, mode: FetchMode) {
    if mode == FetchMode::Single || url.trim().is_empty() || profile.nickname.is_empty() {
        return;
    }
    // 只以 sec_uid 作为身份：同一个人的短链、带 query 的主页链接都应去重到一条
    let sec = profile.sec_uid.clone();
    if sec.is_empty() {
        return;
    }
    let mut s = state.settings_snapshot();

    // 已存在则更新昵称而不是跳过：用户改名后，旧昵称会一直留在列表里，
    // 而昵称是列表里唯一能区分用户的字段。
    if let Some(existing) = s.users.iter_mut().find(|u| u.sec_uid == sec) {
        if existing.username != profile.nickname {
            existing.username = profile.nickname.clone();
            if let Err(e) = state.save_settings(s) {
                log(app, "warn", format!("更新用户昵称失败: {e}"));
            }
        }
        return;
    }

    s.users.push(UserEntry {
        username: profile.nickname.clone(),
        sec_uid: sec,
    });
    if s.users.len() > 50 {
        let drop_n = s.users.len() - 50;
        s.users.drain(0..drop_n);
    }
    if let Err(e) = state.save_settings(s) {
        log(app, "warn", format!("保存用户列表失败: {e}"));
    }
}

/// 退出前停止全部任务（前端点「是」时调用）。
#[tauri::command]
pub fn stop_all(state: State<'_, AppState>) {
    state.stop_all();
}

/// 隐藏主窗口到系统托盘（保持进程存活）。
#[tauri::command]
pub fn hide_to_tray(app: AppHandle) {
    crate::tray::hide_to_tray(&app);
}

/// 真正退出进程。置 QUITTING 后窗口的 closeRequested 不再被拦截。
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::tray::quit_app(&app);
}

/// 从托盘恢复主窗口（前端也可主动调用）。
#[tauri::command]
pub fn show_main(app: AppHandle) {
    crate::tray::show_main(&app);
}

// ---------------- 导出 ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
}

#[tauri::command]
pub fn export_urls(state: State<'_, AppState>, mode: String) -> AppResult<ExportResult> {
    let m = parse_mode(&mode)?;
    let settings = state.settings_snapshot();
    let st = state.mode(m);
    if st.tasks.is_empty() {
        return Err(AppError::msg("没有作品数据可以导出"));
    }
    let folder = crate::config::urls_root(&settings);
    let base = crate::config::user_folder_name(
        if st.summary.nickname.is_empty() {
            "抖音用户"
        } else {
            &st.summary.nickname
        },
        &st.summary.unique_id,
        m.slug(),
    );
    let path = crate::export::write_urls(
        &folder,
        &base,
        &st.tasks,
        settings.add_title_when_export_urls,
    )?;
    Ok(ExportResult { path })
}

#[tauri::command]
pub fn export_excel(state: State<'_, AppState>, mode: String) -> AppResult<ExportResult> {
    let m = parse_mode(&mode)?;
    let settings = state.settings_snapshot();
    let st = state.mode(m);
    if st.tasks.is_empty() {
        return Err(AppError::msg("没有作品数据可以导出"));
    }
    // 有统计就用真实互动数据；老版本残留的空 stats 则退回只导出可下载字段
    let rows = if st.stats.is_empty() {
        build_excel_rows(&st.tasks)
    } else {
        st.stats.iter().map(stat_to_row).collect()
    };
    let folder = crate::config::excel_root(&settings);
    let base = crate::config::user_folder_name(
        if st.summary.nickname.is_empty() {
            "抖音用户"
        } else {
            &st.summary.nickname
        },
        &st.summary.unique_id,
        m.slug(),
    );
    let path = crate::export::write_xlsx(&folder, &base, &rows)?;
    Ok(ExportResult { path })
}

/// 把精简统计转成 Excel 行。
fn stat_to_row(s: &AwemeStat) -> crate::export::ExcelRow {
    let publish_time = chrono::DateTime::from_timestamp(s.create_time, 0)
        .map(|d| {
            use chrono::{Local, TimeZone};
            Local
                .timestamp_opt(d.timestamp(), 0)
                .single()
                .map(|l| l.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let link = if s.aweme_id.is_empty() {
        String::new()
    } else if s.is_album {
        format!("https://www.douyin.com/note/{}", s.aweme_id)
    } else {
        format!("https://www.douyin.com/video/{}", s.aweme_id)
    };

    crate::export::ExcelRow {
        kind: if s.is_album { "图集".into() } else { "视频".into() },
        publish_time,
        desc: s.desc.clone(),
        mix: s.mix.clone(),
        digg: s.digg,
        comment: s.comment,
        collect: s.collect,
        share: s.share,
        recommend: s.recommend,
        duration: crate::export::duration_text(s.duration),
        link,
    }
}

/// 兜底：只有 TaskRow 时导出的行（互动字段为 0）。
///
/// 由 TaskRow 生成 Excel 行（统计字段缺失时的兜底路径）。
fn build_excel_rows(tasks: &[TaskRow]) -> Vec<crate::export::ExcelRow> {
    tasks
        .iter()
        .map(|t| crate::export::ExcelRow {
            kind: match t.kind.as_str() {
                "image" => "图集".to_string(),
                "live" => "实况".to_string(),
                _ => "视频".to_string(),
            },
            publish_time: t.date.clone(),
            desc: t.desc.clone(),
            mix: t.mix.clone(),
            digg: 0,
            comment: 0,
            collect: 0,
            share: 0,
            recommend: 0,
            duration: String::new(),
            link: if t.aweme_id.is_empty() {
                String::new()
            } else if t.is_image {
                format!("https://www.douyin.com/note/{}", t.aweme_id)
            } else {
                format!("https://www.douyin.com/video/{}", t.aweme_id)
            },
        })
        .collect()
}

// ---------------- 杂项 ----------------

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let target = std::path::PathBuf::from(&path);
    let dir = if target.is_dir() {
        target
    } else {
        target.parent().map(|p| p.to_path_buf()).unwrap_or(target)
    };
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::msg(format!("打开目录失败: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| AppError::msg(format!("打开失败: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn download_base() -> String {
    crate::config::default_download_base()
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn open_download_root(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let settings = state.settings_snapshot();
    let root = crate::config::download_root(&settings);
    std::fs::create_dir_all(&root)?;
    app.opener()
        .open_path(root.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::msg(format!("打开目录失败: {e}")))?;
    Ok(())
}

/// 诊断：读登录窗口 cookie 存储，返回 (数量, 是否含 sessionid)。
#[tauri::command]
pub fn debug_cookie_probe(app: AppHandle) -> AppResult<(usize, bool, Vec<String>)> {
    let win = app
        .get_webview_window(crate::auth::LOGIN_LABEL)
        .ok_or_else(|| AppError::msg("登录窗口未打开"))?;
    let cookie = crate::auth::read_cookie_string(&win)?;
    let names: Vec<String> = cookie
        .split(';')
        .filter_map(|kv| kv.split('=').next())
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();
    let has = crate::auth::has_session(&cookie);
    Ok((names.len(), has, names))
}
