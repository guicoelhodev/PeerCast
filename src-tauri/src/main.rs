// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    configure_linux_webkit_renderer();

    desktop_lib::run()
}

/// AppImages run against the user's WebKitGTK and graphics stack. On some
/// Wayland/EGL combinations, WebKit's DMA-BUF renderer aborts before the
/// window is created with `EGL_BAD_PARAMETER`. Disable that optional renderer
/// before Tauri starts so the setting is inherited by WebKit's subprocesses.
#[cfg(target_os = "linux")]
fn configure_linux_webkit_renderer() {
    if std::env::var_os("APPIMAGE").is_some()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}
