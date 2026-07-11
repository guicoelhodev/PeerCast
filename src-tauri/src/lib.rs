pub mod network;
pub mod signaling;

use signaling::{RoomInfo, SignalingState, SignalingStatus};
use std::path::PathBuf;
use tauri::Manager;

#[tauri::command]
fn signaling_status(state: tauri::State<'_, SignalingState>) -> SignalingStatus {
    state.status()
}

#[tauri::command]
fn create_room(state: tauri::State<'_, SignalingState>) -> RoomInfo {
    state.create_room()
}

#[tauri::command]
fn stop_room(state: tauri::State<'_, SignalingState>, room_id: String) -> bool {
    state.remove_room(&room_id)
}

#[tauri::command]
fn room_participants(state: tauri::State<'_, SignalingState>, room_id: String) -> Vec<String> {
    state.room_participants(&room_id)
}

#[tauri::command]
fn set_public_app_url(
    state: tauri::State<'_, SignalingState>,
    public_app_url: String,
) -> Result<(), String> {
    state.set_public_app_url(public_app_url)
}

fn frontend_static_dir(app: &tauri::App) -> Option<PathBuf> {
    let working_dir = std::env::current_dir().ok();
    let candidates = vec![
        working_dir.clone().map(|path| path.join("build")),
        working_dir
            .as_ref()
            .and_then(|path| path.parent())
            .map(|path| path.join("build")),
        app.path()
            .resource_dir()
            .ok()
            .map(|path| path.join("build")),
        app.path().resource_dir().ok(),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.join("index.html").is_file())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let signaling_state = SignalingState::new(network::local_ip());
    let server_state = signaling_state.clone();

    tauri::Builder::default()
        .manage(signaling_state)
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            server_state.set_static_dir(frontend_static_dir(app));
            #[cfg(target_os = "linux")]
            if let Some(webview) = app.get_webview_window("main") {
                webview.with_webview(|webview| {
                    use webkit2gtk::{SettingsExt, WebViewExt};

                    if let Some(settings) = webview.inner().settings() {
                        settings.set_enable_webrtc(true);
                    }
                })?;
            }

            tauri::async_runtime::spawn(async move {
                if let Err(err) = signaling::run_server(server_state).await {
                    eprintln!("{err}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            signaling_status,
            create_room,
            stop_room,
            room_participants,
            set_public_app_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
