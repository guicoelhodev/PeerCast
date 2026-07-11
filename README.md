# Streaming Open

Local-first Tauri and Svelte application for screen sharing, video calls and ephemeral room chat.

The embedded host server relays room signaling and chat messages over WebSocket. Chat messages are limited to 500 characters, available only while connected to a room, and are not persisted.

## Sharing a room with Tailscale

The desktop host serves both the built web client and room WebSockets on port `17777`. For local use with friends, run Tailscale in Docker while keeping Tauri on the host, where it can access the camera, microphone, screen share and desktop window normally.

```bash
cp .env.example .env
# Set TAILSCALE_AUTHKEY and PUBLIC_APP_URL in .env
pnpm start
```

The command connects the Docker Tailscale node, configures its HTTPS proxy to `127.0.0.1:17777`, then opens Tauri. The Tauri server reads `PUBLIC_APP_URL` to generate secure participant links. Docker must run on Linux because the Compose service uses the host network.

The public URL must be an `http://` or `https://` origin without a path. HTTPS is recommended so browser guests can use media permissions. If the WebSocket drops temporarily, connected clients retry automatically with exponential backoff and renegotiate their peer connections when the room returns.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
