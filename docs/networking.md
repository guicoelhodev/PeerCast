# Networking

## Local-First Scope

The MVP is local-first. It does not provide a hosted signaling server, cloud relay, paid TURN server or automatic NAT traversal guarantee.

The host starts the Tauri desktop app, which starts an embedded WebSocket signaling server. Guests connect directly to the host machine.

## Supported Connection Mode

Release builds are private to the host's Tailscale network. PeerCast starts its
server on `127.0.0.1:17777`; the host configures `tailscale serve` for HTTPS
before creating a room. The application displays commands to copy but never
executes Tailscale commands or enables Funnel.

## Connection URLs

Tailscale rooms use the host-provided MagicDNS HTTPS origin:

```text
https://HOST.tailnet.ts.net/?room=ROOM_ID
wss://HOST.tailnet.ts.net/ws/ROOM_ID
```

## VPN Mesh Usage

All participants need to be connected to the host's tailnet (and permitted by
its ACLs). They use the HTTPS room link in a browser; only the host installs
PeerCast.

### Tailscale HTTPS URL

Tailscale remains an external application: PeerCast does not install it,
authenticate it or store credentials. Sign in to Tailscale on the host first:

```bash
sudo systemctl start tailscaled
tailscale up
```

Run the Serve command shown in the app, get the MagicDNS hostname and enter its
HTTPS origin in the app before creating a room.

Start the desktop app with `pnpm start`. The command builds the static browser client before launching Tauri, so the embedded server can serve it to participants. Create a room; generated participant links use the HTTPS hostname and matching secure WebSocket (`wss://`) endpoint.

## NAT Limitations

Tailscale provides the private path to signaling. WebRTC still attempts a
direct peer-to-peer media path; restrictive NATs can require Tailscale relay
transport or a future TURN service.

## Security Considerations

- Room URLs should be treated as sensitive because the MVP has no authentication.
- Only expose the signaling port on trusted networks unless intentionally using port forwarding.
- Use VPN mesh networking for remote sessions when possible.
- Do not assume a random room ID is a full security boundary.
- WebRTC media is encrypted in transit, but access control is still required for production-grade usage.
