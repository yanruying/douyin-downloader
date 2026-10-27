pub mod auth;
pub mod commands;
pub mod config;
pub mod douyin;
pub mod download;
pub mod error;
pub mod export;
pub mod models;
pub mod net;
pub mod paths;
pub mod state;
pub mod tray;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_dir = config::default_app_dir();
            config::ensure(&app_dir).ok();
            let state = AppState::new(app_dir);
            app.manage(state);

            // 托盘只在用户勾选了「最小化到任务栏」后才出现
            {
                let st = app.state::<AppState>();
                if st.settings_snapshot().close_to_tray {
                    if let Err(e) = tray::ensure_tray(app.handle()) {
                        eprintln!("[tray] 创建托盘失败: {e}");
                    }
                }
            }

            // 全局网速采样：500ms 一次
            let handle = app.handle().clone();
            let sampler = Arc::new(tokio::sync::Mutex::new(net::speed::SpeedSampler::new()));
            let stop = {
                let st = app.state::<AppState>();
                st.speed_stop.clone()
            };
            tauri::async_runtime::spawn(async move {
                net::speed::run_sampler(sampler, stop, move |ev| {
                    use tauri::Emitter;
                    let _ = handle.emit("speed:update", ev);
                })
                .await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // 从托盘恢复时通知前端重新拉一次后端状态。
            // 窗口隐藏期间 WebView 可能被节流，进度事件未必都能到达；
            // 恢复后主动同步一次，避免界面停在被隐藏前的旧数字。
            if let WindowEvent::Focused(true) = event {
                if window.label() == "main" {
                    use tauri::Emitter;
                    let _ = window.app_handle().emit("window:focused", ());
                }
            }

            // 关闭按钮：先拦截，交给前端弹确认框。
            // 直接退出会丢掉正在下载的文件（.tmp 尚未 rename），也可能让用户误操作丢任务。
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                if tray::QUITTING.load(std::sync::atomic::Ordering::Relaxed) {
                    return; // 已确认退出，放行
                }
                api.prevent_close();

                let app = window.app_handle();
                let (busy, downloading) = match app.try_state::<AppState>() {
                    Some(st) => (
                        st.any_busy(),
                        st.all_modes().iter().any(|m| m.downloading),
                    ),
                    None => (false, false),
                };
                tray::emit_close_request(app, busy, downloading);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::init_app,
            commands::get_settings,
            commands::save_settings,
            commands::open_login,
            commands::wait_login,
            commands::cancel_login,
            commands::check_cookie,
            commands::set_cookie,
            commands::logout,
            commands::lookup_profile,
            commands::start_fetch,
            commands::batch_fetch,
            commands::stop_fetch,
            commands::start_download,
            commands::stop_download,
            commands::query_busy,
            commands::stop_all,
            commands::hide_to_tray,
            commands::quit_app,
            commands::show_main,
            commands::get_runtime,
            commands::get_tasks,
            commands::clear_tasks,
            commands::export_urls,
            commands::export_excel,
            commands::app_version,
            commands::reveal_path,
            commands::open_path,
            commands::download_base,
            commands::open_download_root,
            commands::debug_cookie_probe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
