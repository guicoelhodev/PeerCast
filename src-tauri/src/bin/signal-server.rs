use desktop_lib::network;
use desktop_lib::signaling::{self, SignalingState};

#[tokio::main]
async fn main() {
    let local_ip = network::local_ip();
    let state = SignalingState::new(local_ip);
    let room = state.create_room();

    println!("ROOM_URL={}", room.signaling_url);

    if let Err(err) = signaling::run_server(state).await {
        eprintln!("server error: {err}");
        std::process::exit(1);
    }
}
