use desktop_lib::signaling::{self, SignalingState};

#[tokio::main]
async fn main() {
    let state = SignalingState::new();
    let room = state
        .create_room(None)
        .expect("failed to create local room");

    println!("ROOM_URL={}", room.signaling_url);

    if let Err(err) = signaling::run_server(state).await {
        eprintln!("server error: {err}");
        std::process::exit(1);
    }
}
