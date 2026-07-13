# Architecture

## Goal

Build a local-first Tauri v2 desktop application for screen sharing, camera video and microphone audio without a hosted backend.

## MVP Architecture

The MVP will use a hybrid architecture:

- Tauri v2 + Rust for the desktop shell, local commands, local IP discovery and embedded signaling server.
- Svelte + TypeScript for UI and WebRTC media/session logic.
- Browser WebRTC APIs inside the Tauri WebView for peer connections, camera, microphone, screen capture and stream quality controls.
- WebSocket signaling hosted inside the Tauri process.

```text
Host machine

Tauri app
  Rust backend
    Embedded WebSocket signaling server
    Local IP discovery
    Room state
    Tauri commands

  Svelte frontend
    Room UI
    Media controls
    Quality selector
    RTCPeerConnection
    getUserMedia/getDisplayMedia

Remote clients
  Connect to ws://HOST_IP:PORT/signaling?room=ROOM_ID
  Exchange SDP and ICE through host signaling server
  Send media peer-to-peer through WebRTC
```

## WebRTC Strategy

WebRTC will live primarily in the frontend using browser APIs:

- `RTCPeerConnection` for peer connections.
- `navigator.mediaDevices.getUserMedia()` for camera and microphone.
- `navigator.mediaDevices.getDisplayMedia()` for screen sharing.
- `RTCRtpSender.setParameters()` for bitrate and framerate hints where supported.
- `MediaStreamTrack.applyConstraints()` or stream restart as fallback for resolution/FPS changes.

This is the simplest MVP path because the WebView already provides media streams in the format expected by browser WebRTC. A Rust-native `webrtc-rs` approach would require bridging native capture, encoders and media tracks into WebRTC, which is more complex and less suitable for the first working flow.

## Embedded Signaling

The host app starts a local WebSocket server from Rust. Implementation uses `tokio` + `axum` WebSocket support for HTTP/WebSocket routing, state sharing and error handling.

Default connection shape:

```text
ws://HOST_IP:17777/ws/ROOM_ID
```

All WebSocket messages in a room are broadcast to every client (minus sender). No server-side peer addressing — routing is handled by the frontend using `peerId`.

## Room Chat

The room chat uses the existing WebSocket relay rather than WebRTC data channels. This makes text chat available as soon as a participant connects to the room, independently of media negotiation. Chat messages are broadcast to the room, retained only in each browser session (up to 200 messages), and are not persisted by the Rust server.

Chat text is limited to 500 characters. The frontend validates incoming payloads and the relay rejects malformed, oversized, empty, or peer-ID-spoofed chat messages.

## Sharing Modes and Reconnection

The embedded Axum server serves the built Svelte client when the desktop app is running. It listens only on localhost; rooms use a host-provided Tailscale MagicDNS HTTPS origin and derive a matching `wss://` signaling endpoint. Tailscale stays external to the application; PeerCast does not manage Docker, authentication keys, or the Tailscale daemon.

Browser clients treat unexpected WebSocket closure as temporary. They reconnect with exponential backoff, send `ready` again, and recreate their WebRTC mesh connections while retaining active local camera and screen tracks. A deliberate Leave action cancels retries.

## Signaling Protocol

Current protocol (implemented in `src/lib/signaling-protocol.ts`):

```ts
type SignalMessage =
  | { type: "ready"; peerId: string }
  | { type: "offer"; description: RTCSessionDescriptionInit; peerId: string }
  | { type: "answer"; description: RTCSessionDescriptionInit; peerId: string }
  | { type: "ice"; candidate: RTCIceCandidateInit; peerId: string }
  | { type: "reject"; peerId: string }
  | { type: "guest-connected"; peerId: string };
```

### Message Flow

1. **Guest connects**: sends `ready` with its randomly generated `peerId`
2. **Host receives `ready`**: creates a new `RTCPeerConnection` for that `peerId`, sends `guest-connected` with the same `peerId`, then sends `offer`
3. **Guest receives `guest-connected`** and `offer` (filtering by `peerId`): creates answer, sends back
4. **ICE candidates**: exchanged with `peerId` for host-side routing
5. **`reject`**: host can reject specific guests (reserved for future use; currently auto-accept)

### Multi-Guest Architecture

```
Broadcast::Sender<SignalMessage>  (one per room)
              │
    ┌─────────┼─────────┐
    │         │         │
  Guest A   Guest B    Host
  (peerId=  (peerId=   (peerId=
   aaa)      bbb)       host-xxx)
    │         │         │
    │  ready  │         │
    ├─────────┼─────────┤
    │         │         │
    │  guest-connected  │
    │  offer  │         │
    │◄────────┤  (filtered by
    │         │   peerId)  │
    │  answer │         │
    ├─────────┤         │
    │         │         │
    │  ice    │  ice    │
    ├─────────┼─────────┤
    │         │         │
    ▼         ▼         ▼
  WebRTC    WebRTC    WebRTC
  PC(host)  PC(host)  PC(aaa)
                       PC(bbb)
```

Host creates one `RTCPeerConnection` per guest. Guests have a single `RTCPeerConnection` to the host. Guests are receive-only (no local tracks added).

## Screen Capture

The MVP will first attempt browser `getDisplayMedia()` inside the Tauri WebView. This requires runtime validation on Linux, Windows and macOS because Tauri uses the OS web renderer, and screen capture behavior can differ by platform and renderer.

If `getDisplayMedia()` is unavailable or unreliable on a target platform, a later fallback can use native screen capture libraries or platform APIs. That fallback is out of scope until the frontend path is validated.

## Camera And Audio

The MVP will use browser `getUserMedia()` for camera and microphone. It is widely supported in secure contexts and avoids native device capture complexity for the first version.

## Quality Control

Quality control will be applied in layers:

- Capture constraints for requested resolution and FPS where possible.
- `RTCRtpSender.setParameters()` with `maxBitrate` and `maxFramerate` where supported.
- `scaleResolutionDownBy` where supported.
- `MediaStreamTrack.applyConstraints()` as a fallback.
- Stream restart when live changes cannot be applied reliably.

## Platform Notes

- The Tauri app uses native WebViews, not a bundled Chromium engine.
- Windows depends on WebView2 behavior.
- macOS depends on WKWebView behavior and OS screen-recording permissions.
- Linux depends on WebKitGTK/WebRTC support and the desktop session/portal stack.
- `getDisplayMedia()` is less universally available than `getUserMedia()` and needs practical validation.

## Security Model

The MVP has no accounts or authentication. Users must only expose the signaling port on trusted networks or VPNs. WebRTC provides transport encryption, but room URLs should be treated as bearer access to the session.
