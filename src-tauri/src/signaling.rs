use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{header::CONTENT_TYPE, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::{net::TcpListener, sync::broadcast};
use uuid::Uuid;

const DEFAULT_SIGNALING_PORT: u16 = 17777;
const ROOM_CHANNEL_CAPACITY: usize = 64;
const MAX_CHAT_MESSAGE_LENGTH: usize = 500;
const MAX_DISPLAY_NAME_LENGTH: usize = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalingStatus {
    pub is_running: bool,
    pub port: u16,
    pub active_room: Option<RoomInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomInfo {
    pub room_id: String,
    pub app_url: String,
    pub signaling_url: String,
}

#[derive(Debug)]
struct Room {
    info: RoomInfo,
    tx: broadcast::Sender<SignalMessage>,
    participants: HashMap<Uuid, String>,
    peer_connections: HashMap<String, Uuid>,
}

#[derive(Debug, Clone)]
struct SignalMessage {
    sender_id: Uuid,
    payload: String,
}

#[derive(Debug)]
struct InnerState {
    rooms: HashMap<String, Room>,
    port: u16,
    is_running: bool,
    static_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SignalingState {
    inner: Arc<Mutex<InnerState>>,
}

impl SignalingState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerState {
                rooms: HashMap::new(),
                port: DEFAULT_SIGNALING_PORT,
                is_running: false,
                static_dir: None,
            })),
        }
    }

    pub fn create_room(&self, public_app_url: Option<String>) -> Result<RoomInfo, String> {
        let room_id = Uuid::new_v4().simple().to_string();
        let (tx, _) = broadcast::channel(ROOM_CHANNEL_CAPACITY);
        let (app_url, signaling_url) = self.room_urls(&room_id, public_app_url.as_deref())?;

        let mut inner = self.inner.lock().expect("signaling state mutex poisoned");
        if !inner.rooms.is_empty() {
            return Err("a room is already active; stop it before creating another".to_string());
        }
        let room = RoomInfo {
            room_id: room_id.clone(),
            app_url,
            signaling_url,
        };
        inner.rooms.insert(
            room_id,
            Room {
                info: room.clone(),
                tx,
                participants: HashMap::new(),
                peer_connections: HashMap::new(),
            },
        );

        Ok(room)
    }

    pub fn remove_room(&self, room_id: &str) -> bool {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .rooms
            .remove(room_id)
            .is_some()
    }

    pub fn status(&self) -> SignalingStatus {
        let inner = self.inner.lock().expect("signaling state mutex poisoned");
        SignalingStatus {
            is_running: inner.is_running,
            port: inner.port,
            active_room: inner.rooms.values().next().map(|room| room.info.clone()),
        }
    }

    fn mark_running(&self) {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .is_running = true;
    }

    pub fn set_static_dir(&self, static_dir: Option<PathBuf>) {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .static_dir = static_dir;
    }

    fn room_urls(
        &self,
        room_id: &str,
        public_app_url: Option<&str>,
    ) -> Result<(String, String), String> {
        let inner = self.inner.lock().expect("signaling state mutex poisoned");
        if let Some(public_app_url) = public_app_url {
            let app_url = normalize_public_app_url(public_app_url)?
                .ok_or_else(|| "a public app URL is required for a shared room".to_string())?;
            return Ok((
                app_url.clone(),
                format!("{}/ws/{room_id}", websocket_origin(&app_url)),
            ));
        }
        let host = "127.0.0.1";

        let app_url = format!("http://{host}:{}", inner.port);
        Ok((
            app_url.clone(),
            format!("ws://{host}:{}/ws/{room_id}", inner.port),
        ))
    }

    fn room_sender(&self, room_id: &str) -> Option<broadcast::Sender<SignalMessage>> {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .rooms
            .get(room_id)
            .map(|room| room.tx.clone())
    }

    pub fn room_participants(&self, room_id: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .rooms
            .get(room_id)
            .map(|room| room.participants.values().cloned().collect())
            .unwrap_or_default()
    }

    fn identify_participant(&self, room_id: &str, connection_id: Uuid, payload: &str) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
            return;
        };
        let Some(peer_id) = value.get("peerId").and_then(serde_json::Value::as_str) else {
            return;
        };
        let display_name = value
            .get("displayName")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty() && name.chars().count() <= MAX_DISPLAY_NAME_LENGTH);

        if let Some(room) = self
            .inner
            .lock()
            .expect("signaling state mutex poisoned")
            .rooms
            .get_mut(room_id)
        {
            room.peer_connections
                .insert(peer_id.to_string(), connection_id);
            if let Some(display_name) = display_name {
                room.participants
                    .insert(connection_id, display_name.to_string());
            }
        }
    }

    fn remove_participant(&self, room_id: &str, connection_id: Uuid) {
        if let Some(room) = self
            .inner
            .lock()
            .expect("signaling state mutex poisoned")
            .rooms
            .get_mut(room_id)
        {
            room.participants.remove(&connection_id);
        }
    }

    fn disconnect_peer(&self, room_id: &str, connection_id: Uuid, peer_id: &str) -> bool {
        let mut inner = self.inner.lock().expect("signaling state mutex poisoned");
        let Some(room) = inner.rooms.get_mut(room_id) else {
            return false;
        };

        if room.peer_connections.get(peer_id) != Some(&connection_id) {
            return false;
        }

        room.peer_connections.remove(peer_id);
        true
    }

    fn connection_matches_peer(&self, room_id: &str, connection_id: Uuid, peer_id: &str) -> bool {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .rooms
            .get(room_id)
            .and_then(|room| room.peer_connections.get(peer_id))
            == Some(&connection_id)
    }

    fn static_dir(&self) -> Option<PathBuf> {
        self.inner
            .lock()
            .expect("signaling state mutex poisoned")
            .static_dir
            .clone()
    }
}

fn normalize_public_app_url(value: &str) -> Result<Option<String>, String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Ok(None);
    }
    if !(value.starts_with("https://") || value.starts_with("http://"))
        || value.contains(char::is_whitespace)
    {
        return Err("public app URL must start with http:// or https://".to_string());
    }
    let host = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or_default();
    if host.is_empty() || host.contains(['/', '?', '#']) {
        return Err("public app URL must be an origin without a path".to_string());
    }
    Ok(Some(value.to_string()))
}

fn websocket_origin(public_app_url: &str) -> String {
    public_app_url
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1)
}

fn is_valid_chat_message(payload: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return false;
    };
    if value.get("type").and_then(serde_json::Value::as_str) != Some("chat") {
        return true;
    }

    let Some(text) = value.get("text").and_then(serde_json::Value::as_str) else {
        return false;
    };
    !text.trim().is_empty()
        && text.chars().count() <= MAX_CHAT_MESSAGE_LENGTH
        && value
            .get("messageId")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty())
        && value
            .get("sentAt")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty())
        && value
            .get("peerId")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty())
}

pub async fn run_server(state: SignalingState) -> Result<(), String> {
    let port = state.status().port;
    let app = Router::new()
        .route("/ws/:room_id", get(websocket_handler))
        .fallback(get(static_handler))
        .with_state(state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| format!("failed to bind signaling server on {addr}: {err}"))?;
    state.mark_running();

    axum::serve(listener, app)
        .await
        .map_err(|err| format!("signaling server stopped: {err}"))
}

async fn static_handler(State(state): State<SignalingState>, uri: Uri) -> Response {
    let Some(root) = state.static_dir() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let requested = uri.path().trim_start_matches('/');
    if requested
        .split('/')
        .any(|part| part == ".." || part.contains('\\'))
    {
        return StatusCode::FORBIDDEN.into_response();
    }

    let candidate = if requested.is_empty() {
        root.join("index.html")
    } else {
        root.join(requested)
    };
    let path = if candidate.is_file() {
        candidate
    } else {
        root.join("index.html")
    };
    match fs::read(&path) {
        Ok(contents) => (
            StatusCode::OK,
            [(CONTENT_TYPE, content_type_for(&path))],
            contents,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type_for(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    State(state): State<SignalingState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, room_id, state))
}

async fn handle_socket(socket: WebSocket, room_id: String, state: SignalingState) {
    let Some(tx) = state.room_sender(&room_id) else {
        return;
    };

    let mut rx = tx.subscribe();
    let (mut sender, mut receiver) = socket.split();
    let peer_id = Uuid::new_v4();
    let receive_state = state.clone();
    let receive_room_id = room_id.clone();
    let client_peer_id = Arc::new(Mutex::new(None::<String>));
    let receive_client_peer_id = client_peer_id.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Text(text) => {
                    if !is_valid_chat_message(&text) {
                        continue;
                    }
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(id) = value.get("peerId").and_then(serde_json::Value::as_str) {
                            if value.get("type").and_then(serde_json::Value::as_str) == Some("chat")
                                && !receive_state.connection_matches_peer(
                                    &receive_room_id,
                                    peer_id,
                                    id,
                                )
                            {
                                continue;
                            }
                            *receive_client_peer_id
                                .lock()
                                .expect("client peer ID mutex poisoned") = Some(id.to_string());
                        }
                    }
                    receive_state.identify_participant(&receive_room_id, peer_id, &text);
                    let _ = tx.send(SignalMessage {
                        sender_id: peer_id,
                        payload: text,
                    });
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            if message.sender_id == peer_id {
                continue;
            }

            if sender.send(Message::Text(message.payload)).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = receive_task => {}
        _ = send_task => {}
    }

    let client_peer_id = client_peer_id
        .lock()
        .expect("client peer ID mutex poisoned")
        .clone();
    if let Some(client_peer_id) = client_peer_id {
        if state.disconnect_peer(&room_id, peer_id, &client_peer_id) {
            let payload = serde_json::json!({
                "type": "participant-left",
                "peerId": client_peer_id,
            })
            .to_string();
            let _ = state.room_sender(&room_id).map(|tx| {
                tx.send(SignalMessage {
                    sender_id: Uuid::new_v4(),
                    payload,
                })
            });
        }
    }
    state.remove_participant(&room_id, peer_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_https_origin_creates_secure_room_urls() {
        let state = SignalingState::new();
        let room = state
            .create_room(Some("https://host.tailnet.ts.net".to_string()))
            .expect("valid public URL");
        assert_eq!(room.app_url, "https://host.tailnet.ts.net");
        assert!(room
            .signaling_url
            .starts_with("wss://host.tailnet.ts.net/ws/"));
    }

    #[test]
    fn public_app_url_rejects_paths() {
        let state = SignalingState::new();
        assert!(state
            .create_room(Some("https://host.tailnet.ts.net/room".to_string()))
            .is_err());
    }

    #[test]
    fn only_one_room_can_be_active() {
        let state = SignalingState::new();
        state
            .create_room(None)
            .expect("first room creation succeeds");

        assert!(state.create_room(None).is_err());
    }
}
