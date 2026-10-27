#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! 应用入口。
//!
//! 开发模式（`cargo run` / `tauri dev`，不带 `custom-protocol`）下，窗口指向
//! `devUrl`（localhost:1420）。若此时 Vite 没起来，用户会看到浏览器错误页
//! ERR_CONNECTION_REFUSED —— 看起来像软件坏了，实际只是少了前端服务。
//!
//! 所以这里先探一下端口：连不上就弹一个能看懂的中文提示，而不是把错误页丢给用户。
//! 正式构建（`tauri build`）带 `custom-protocol`，前端已嵌入 exe，不走这条路径。

fn main() {
    #[cfg(all(dev, not(feature = "custom-protocol")))]
    warn_if_dev_server_missing();

    douyin_downloader_lib::run()
}

#[cfg(all(dev, not(feature = "custom-protocol")))]
fn warn_if_dev_server_missing() {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = "127.0.0.1:1420".parse().expect("valid addr");
    if TcpStream::connect_timeout(&addr, Duration::from_millis(600)).is_ok() {
        return;
    }

    let msg = "前端开发服务器未启动（localhost:1420 连接被拒绝）。\n\n\
               请用以下方式启动，不要直接双击 target/debug 里的 exe：\n\
               · 开发调试：pnpm tauri dev\n\
               · 正式使用：pnpm tauri build 后再运行生成的 exe";
    eprintln!("[启动失败] {msg}");
    show_message_box("抖音作品下载 - 启动失败", msg);
}

/// 用系统原生弹窗提示。控制台被隐藏时这是唯一用户可见的信息。
#[cfg(all(dev, not(feature = "custom-protocol"), windows))]
fn show_message_box(title: &str, text: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, utype: u32) -> i32;
    }

    let to_wide = |s: &str| -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    };
    let t = to_wide(text);
    let c = to_wide(title);
    unsafe {
        MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), 0x0000_0010);
    }
}

#[cfg(all(dev, not(feature = "custom-protocol"), not(windows)))]
fn show_message_box(_title: &str, _text: &str) {}
