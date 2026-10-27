//! 登录窗口 + Cookie 提取。
//!
//! 使用 Tauri 子 WebviewWindow 打开 douyin.com，
//! 由 Rust 侧读 cookie 存储（sessionid 是 HttpOnly，`document.cookie` 读不到）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{AppError, AppResult};

pub const LOGIN_LABEL: &str = "login";
pub const LOGIN_URL: &str = "https://www.douyin.com/";

/// 登录窗口尺寸：模拟一个正常浏览器窗，扫码页需要足够宽。
const LOGIN_W: f64 = 1000.0;
const LOGIN_H: f64 = 720.0;

pub struct LoginState {
    pub polling: Arc<AtomicBool>,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            polling: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// 创建（或聚焦）登录窗口。
pub fn open_login_window(app: &AppHandle) -> AppResult<()> {
    if let Some(w) = app.get_webview_window(LOGIN_LABEL) {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }

    let url = Url::parse(LOGIN_URL).map_err(|e| AppError::msg(format!("URL 非法: {e}")))?;

    WebviewWindowBuilder::new(app, LOGIN_LABEL, WebviewUrl::External(url))
        .title("抖音登录 - 扫码后自动获取 Cookie")
        .inner_size(LOGIN_W, LOGIN_H)
        .min_inner_size(800.0, 600.0)
        .center()
        .user_agent(crate::douyin::api::UA)
        .build()
        .map_err(AppError::from)?;

    Ok(())
}

/// 从窗口 cookie 存储读取并拼成 `k=v; k=v` 串。
pub fn read_cookie_string(window: &WebviewWindow) -> AppResult<String> {
    let url = Url::parse(LOGIN_URL).map_err(|e| AppError::msg(format!("URL 非法: {e}")))?;
    let cookies = window.cookies_for_url(url).map_err(AppError::from)?;

    let mut parts: Vec<String> = cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect();
    // 稳定输出，便于日志比对
    parts.sort();
    Ok(parts.join("; "))
}

pub fn has_session(cookie: &str) -> bool {
    cookie.split(';').any(|kv| {
        let k = kv.split('=').next().unwrap_or("").trim();
        k == "sessionid" || k == "sessionid_ss"
    })
}

/// 轮询登录窗口直到命中 sessionid 或超时。
/// 返回 `Some(cookie)` 表示拿到，`None` 表示超时/用户关闭。
pub async fn wait_for_login(
    app: AppHandle,
    timeout: Duration,
    should_cancel: impl Fn() -> bool,
) -> AppResult<Option<String>> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::time::Instant::now() > deadline {
            return Ok(None);
        }
        if should_cancel() {
            return Ok(None);
        }
        tokio::time::sleep(Duration::from_millis(1200)).await;

        let Some(win) = app.get_webview_window(LOGIN_LABEL) else {
            // 窗口被用户关了：最后再试一次读 cookie（已拿到就返回）
            return Ok(None);
        };
        if let Ok(cookie) = read_cookie_string(&win) {
            if has_session(&cookie) {
                let _ = win.close();
                return Ok(Some(cookie));
            }
        }
    }
}

/// 关闭登录窗口（用户取消时调用）。
pub fn close_login_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LOGIN_LABEL) {
        let _ = w.close();
    }
}

/// 登录成功后确保主窗口回到前台。
pub fn focus_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 显式存活性检查，防止编译器把 `Ordering` 优化掉的同时给调用方一个清晰的语义。
pub fn is_polling(state: &LoginState) -> bool {
    state.polling.load(Ordering::Relaxed)
}
