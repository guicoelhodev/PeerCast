# Decisions

## Decision: Use Embedded Signaling Server

### Status

Accepted

### Context

The project must work without a hosted backend, cloud signaling server or paid infrastructure.

### Decision

The signaling server will run inside the host Tauri application and listen on a local network port.

### Consequences

Users can connect directly to the host over LAN, VPN mesh or port forwarding. The host machine must be reachable by IP and port. Restrictive NAT without VPN, port forwarding or TURN can prevent connections.

## Decision: Use Frontend Browser WebRTC APIs First

### Status

Accepted

### Context

The app needs camera, microphone, screen sharing, peer connection setup and quality control. Implementing all of this natively in Rust would require more capture and media pipeline work.

### Decision

The MVP will implement WebRTC primarily in the Svelte frontend using `RTCPeerConnection`, `getUserMedia()`, `getDisplayMedia()` and `RTCRtpSender.setParameters()`.

### Consequences

The MVP can move faster and use standard WebRTC media streams. Platform-specific WebView behavior remains a risk, especially for screen capture. Practical validation is required after bootstrap.

## Decision: Use Rust For Signaling And Local System Integration

### Status

Accepted

### Context

The application needs a local server, local IP discovery and room state tied to the desktop host.

### Decision

Rust will own the embedded signaling server, room registry, local IP discovery and Tauri commands. `tokio` + `axum` is the preferred server stack.

### Consequences

The frontend remains focused on UI and WebRTC, while Rust handles local-first infrastructure. The app stays backend-free from a deployment perspective.

## Decision: Defer Rust-Native Media Pipeline

### Status

Accepted

### Context

Rust-native screen/audio/camera capture could offer deeper control, but it would increase platform complexity early.

### Decision

Native Rust media capture and `webrtc-rs` are deferred unless Tauri WebView browser APIs fail MVP requirements.

### Consequences

The first MVP is simpler. If WebView media support is insufficient, the project will need a targeted spike for native capture and Rust WebRTC.

## Decision: Use Root-Level Tauri App Structure

### Status

Accepted

### Context

The project currently targets a single local-first Tauri desktop app. A nested `apps/desktop` workspace adds unnecessary complexity.

### Decision

The Tauri/Svelte application lives at the repository root. Rust backend inside `src-tauri/src`.

### Consequences

Commands run directly from the repository root. If the signaling layer grows enough to justify reuse, it can later be extracted to `crates/signaling`.

## Decision: Build Linux As AppImage Only

### Status

Accepted

### Context

The primary Linux test environment is Arch Linux, where `.deb` and `.rpm` packages are not useful as the main local test artifact.

### Decision

The official Linux bundle target is AppImage only.

### Consequences

Linux packaging focuses on a portable AppImage artifact. On Arch, the default Tauri-downloaded `linuxdeploy` may require a workaround.

## Decision: Broadcast Signaling With Frontend peerId Routing

### Status

Accepted

### Context

The axum signaling server broadcasts all WebSocket messages to every client in a room (minus sender). With multiple guests, hosts need to route WebRTC signaling to specific peers.

### Decision

Add a `peerId` field to all signaling messages. Guests generate a random `peerId` on mount. The host uses it to create per-guest `RTCPeerConnection` instances and filter incoming messages. Guests filter host messages by matching their own `peerId`.

### Consequences

The signaling server stays simple (broadcast relay). Routing logic lives entirely in the frontend. The protocol is self-describing and debuggable. Adding more peers requires no server changes.

## Decision: Auto-Accept Guests (No Accept/Reject Flow)

### Status

Accepted

### Context

The original flow required the host to manually accept each guest. This adds friction and doesn't scale to multiple guests.

### Decision

When a guest sends a `ready` message, the host immediately creates a peer connection, sends `guest-connected` and `offer` without any user interaction.

### Consequences

Faster connection, better UX for multi-guest scenarios. Trade-off: any visitor with the room URL can join. Acceptable for local-first MVP since rooms are shared on trusted networks or VPNs.

## Decision: Guest Receive-Only (Streamer→Viewer Model)

### Status

Accepted

### Context

The initial multi-guest implementation had guests adding their local camera/audio tracks to peer connections, creating a group call where everyone sees everyone. This is not the intended UX — the host streams, guests watch.

### Decision

Guests do not call `addTrack` on their peer connections. Only the host sends media tracks. Guests are pure receivers.

### Consequences

Simpler bandwidth model (host upload only). Guests don't need camera/mic permissions unless they want to become host later. The app behaves like a streaming service, not a group video call.