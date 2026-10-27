//! 系统托盘与窗口关闭策略。
//!
//! 关闭行为由前端统一处理（需要弹确认框、读取「最小化到托盘」设置）：
//! 窗口的 `closeRequested` 一律先 `preventDefault` 并 emit 事件给前端，
//! 由前端决定是隐藏到托盘还是真正退出。这样判定逻辑只有一处。

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

/// 真正退出时置位，用于区分「隐藏窗口」与「结束进程」。
pub static QUITTING: AtomicBool = AtomicBool::new(false);

/// 托盘图标是否已创建。
///
/// 托盘只在用户勾选「最小化到任务栏」后才出现 —— 没打算用托盘的人
/// 不该在通知区多一个图标。所以这里做惰性创建，勾选时调 `ensure_tray`。
static TRAY_READY: AtomicBool = AtomicBool::new(false);

/// 按需创建托盘图标（幂等）。
pub fn ensure_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if TRAY_READY.load(Ordering::Relaxed) {
        return Ok(());
    }
    create_tray(app)?;
    TRAY_READY.store(true, Ordering::Relaxed);
    Ok(())
}

/// 移除托盘图标（用户取消勾选时）。
pub fn remove_tray<R: Runtime>(app: &AppHandle<R>) {
    if !TRAY_READY.load(Ordering::Relaxed) {
        return;
    }
    // Tauri 2 用 id 移除
    let _ = app.remove_tray_by_id("main-tray");
    TRAY_READY.store(false, Ordering::Relaxed);
}

fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("抖音作品下载")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => quit_app(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

/// 显示并聚焦主窗口。
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 真正结束进程。
pub fn quit_app<R: Runtime>(app: &AppHandle<R>) {
    QUITTING.store(true, Ordering::Relaxed);
    app.exit(0);
}

/// 隐藏到托盘（不结束进程）。
pub fn hide_to_tray<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

/// 通知前端「用户点了关闭按钮」，由前端决定后续动作。
pub fn emit_close_request<R: Runtime>(app: &AppHandle<R>, busy: bool, downloading: bool) {
    let _ = app.emit(
        "window:close-request",
        serde_json::json!({ "busy": busy, "downloading": downloading }),
    );
    // 弹窗期间把窗口拉到前台，否则用户可能看不到询问框
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_focus();
    }
}
