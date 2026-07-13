// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    configure_linux_appimage_renderer();

    desktop_lib::run()
}

#[cfg(target_os = "linux")]
fn configure_linux_appimage_renderer() {
    if std::env::var_os("APPIMAGE").is_some() && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        // WebKitGTK can abort while creating its EGL display on some Wayland
        // graphics stacks. Software compositing avoids that EGL path.
        if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
}
