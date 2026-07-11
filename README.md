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

## Sharing Modes

When creating a room in the desktop application, choose one of these modes:

- **Local network**: generates links using the host's local IP and port `17777`. All participants must be on the same network, and the host firewall must allow inbound connections on that port. Browser media permissions can be restricted on plain HTTP IP addresses.
- **Tailscale**: generates secure HTTPS and WSS links using the MagicDNS URL provided by the host. Tailscale remains an external application; PeerCast does not manage its credentials or installation.

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

Open `http://localhost:1420` in a browser. This mode is useful for working on the interface. For a complete local room, use the Tauri app.

### Tauri desktop app

```bash
pnpm install
pnpm tauri:dev
```

The desktop app starts the embedded signaling server on port `17777`. Create a room in the app, then open the generated browser URL or share the participant URL.

### Create a local room

1. Run the desktop app:

```bash
pnpm start
```

2. Choose **Local network** in the sidebar and create a room.
3. Share the participant URL with devices on the same LAN.
4. Allow inbound connections to port `17777` in the host firewall if prompted.

### Tailscale sharing

Install and authenticate Tailscale directly on the host machine. Docker and a Tailscale auth key are not required by PeerCast.

1. Start Tailscale and authenticate the host if it is not already connected:

```bash
tailscale up
```

2. Expose PeerCast's local server through Tailscale HTTPS:

```bash
tailscale serve --bg --https=443 http://127.0.0.1:17777
```

3. Find the MagicDNS hostname:

```bash
tailscale status --json | jq -r '.Self.DNSName'
```

4. Run PeerCast with `pnpm start`, choose **Tailscale**, and enter the HTTPS MagicDNS URL without a path:

```text
https://host.tailnet.ts.net
```

5. Create the room and share the generated participant link.

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
