# PeerCast

I like Discord, but I can't afford Nitro to stream my screen in a call.

PeerCast is a local-first, open-source screen sharing and video call app. It lets you create a temporary room on your own machine, share a room link, stream your screen, use your camera and microphone, and chat with participants without relying on a hosted streaming service.

<div align="center">
    <img src=".github/screen_client.png" alt="PeerCast client view" />
</div>
<br/>
PeerCast also lets you change the transmission quality while sharing your screen. Choose between quality presets with different resolutions, frame rates and bitrates to balance image quality and network usage.

## What It Uses

- **Tauri v2** for the desktop application and embedded local server.
- **Svelte 5 and TypeScript** for the user interface and session logic.
- **WebRTC** for peer-to-peer video, screen sharing and audio.
- **WebSocket** signaling through the embedded Rust server to coordinate connections.
- **Rust, Tokio and Axum** for the local signaling server and room state.
- **Tailwind CSS** for the interface styling.

Rooms are ephemeral. PeerCast does not require accounts, does not persist chat messages and does not use a central media server. Treat room links as private access tokens and only share them with people you trust.

## Private Tailscale Sharing

Release builds use Tailscale only. The embedded server listens on localhost;
the host configures private Tailscale HTTPS/WSS access before creating a room.
Only the host installs PeerCast; participants connected to the same tailnet
open the generated link in a browser.

Tailscale must already be installed and authenticated on the host. PeerCast
only displays commands to copy; it never executes them, enables Funnel or
exposes the room to the public internet.

## Run Locally

### Requirements

- Node.js and `pnpm`.
- Rust and Cargo.
- Tauri system dependencies for your operating system.

### Browser development mode

This runs the Svelte client locally without starting the Tauri server:

```bash
pnpm install
pnpm dev
```

Open `http://localhost:1420` in a browser. This mode is useful for working on the interface. For a complete room, use the Tauri app with Tailscale.

### Tauri desktop app

```bash
pnpm install
pnpm tauri:dev
```

The desktop app starts the embedded signaling server on port `17777`. Create a room in the app, then open the generated browser URL or share the participant URL.

### Tailscale sharing

Install and authenticate Tailscale directly on the host machine. Docker and a Tailscale auth key are not required by PeerCast.

1. Start Tailscale and authenticate the host if it is not already connected:

```bash
tailscale up
```

2. Run PeerCast with `pnpm start`, copy and run the Tailscale Serve command
shown in the app, then enter the displayed MagicDNS HTTPS URL.

3. Create the room and share the generated participant link with members allowed by your tailnet
ACLs.

## Useful Commands

```bash
pnpm check
pnpm build
pnpm tauri:build
```

`pnpm tauri:build` creates the Linux AppImage configured by the Tauri project.

## Security Notes

PeerCast has no accounts or authentication. Anyone with a room link may be able to join that room. Use it on trusted networks or through a private Tailscale network.

## License

This project is licensed under the MIT License.
