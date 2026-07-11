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

## Why Tailscale?

PeerCast is designed to run locally, but browser participants need to reach the host computer. Tailscale provides a private VPN between your devices and gives the host a stable MagicDNS address. Its HTTPS proxy also gives browser clients a secure origin, which is important for camera, microphone and screen-sharing permissions.

The included Docker Compose setup runs Tailscale with host networking and proxies the PeerCast server on port `17777`. The desktop app continues to run on the host, where it can access the camera, microphone and screen capture normally.

## Run Locally

### Requirements

- Node.js and `pnpm`.
- Rust and Cargo.
- Tauri system dependencies for your operating system.
- Docker, only if you want to share a room through Tailscale.

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

### Tailscale sharing

1. Create a Tailscale auth key in the Tailscale admin console.
2. Copy the environment template:

```bash
cp .env.example .env
```

3. Set `TAILSCALE_AUTHKEY` in `.env`.
4. Find your Tailscale DNS hostname before starting Docker. If Tailscale is already installed and connected on the host, run:

```bash
tailscale status --json | jq -r '.Self.DNSName'
```

If Tailscale is not installed on the host, get the MagicDNS domain from the Tailscale admin console or from another connected Tailscale device. The hostname configured by this project is `peercast`.

5. Set `PUBLIC_APP_URL` in `.env` to the HTTPS URL for the `peercast` hostname. For example:

```bash
PUBLIC_APP_URL=https://peercast.example.ts.net
```

The URL must be an `http://` or `https://` origin without a path. HTTPS is recommended because browser clients need a secure context for camera, microphone and screen-sharing permissions.

6. Start PeerCast with the shared Tailscale setup. This starts Docker Compose after `PUBLIC_APP_URL` has been configured:

```bash
pnpm start
```

The command starts the Tailscale container, configures the HTTPS proxy to the local PeerCast server and opens the Tauri app. Docker must run on Linux because the Compose service uses host networking.

## Useful Commands

```bash
pnpm check
pnpm build
pnpm tauri:build
```

`pnpm tauri:build` creates the Linux AppImage configured by the Tauri project.

## Security Notes

PeerCast has no accounts or authentication. Anyone with a room link may be able to join that room. Use it on trusted networks or through a private Tailscale network, and never commit a real Tailscale auth key.

## License

This project is licensed under the MIT License.
