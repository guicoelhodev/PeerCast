# Networking

## Local-First Scope

The MVP is local-first. It does not provide a hosted signaling server, cloud relay, paid TURN server or automatic NAT traversal guarantee.

The host starts the Tauri desktop app, which starts an embedded WebSocket signaling server. Guests connect directly to the host machine.

## Supported Connection Modes

- Same LAN, using the host's local IP address.
- VPN mesh networks such as Tailscale, Hamachi or ZeroTier.
- Direct access to the host IP and port.
- Port forwarding, if the host chooses to expose the app to the public internet.

## Connection URL

The intended signaling URL shape is:

```text
ws://HOST_IP:PORT/signaling?room=ROOM_ID&peer=PEER_ID
```

The UI should also show a human-friendly room URL or copyable connection details.

## LAN Usage

On the same local network, guests should connect to the host's private IP address, for example `192.168.x.x`, `10.x.x.x` or `172.16.x.x`.

The host firewall must allow inbound connections to the selected signaling port.

## VPN Mesh Usage

With Tailscale, Hamachi, ZeroTier or similar tools, users should connect to the host's VPN-assigned IP address. This is the recommended remote usage mode for the MVP because it avoids most manual NAT and router setup.

### Tailscale HTTPS URL

For shareable browser links, start Tailscale on the host and obtain its MagicDNS hostname:

```bash
sudo systemctl start tailscaled
tailscale up
tailscale status --json | jq -r '.Self.DNSName'
```

The final command prints a hostname such as `arch.tail6f452c.ts.net.`. Remove the trailing dot and enter it in the app's **Share URL (Tailscale)** field with HTTPS:

```text
https://arch.tail6f452c.ts.net
```

Expose the app's local port `17777` through the Tailscale HTTPS proxy before creating the room:

```bash
tailscale serve --bg --https=443 http://127.0.0.1:17777
tailscale serve status
```

Start the desktop app with `pnpm tauri:dev` first. Run `pnpm build` once before the first shared development session so the embedded server has the static browser client to serve. The generated participant links then use the HTTPS hostname and a matching secure WebSocket (`wss://`) endpoint.

## Port Forwarding

Public internet access can work if the host forwards the selected TCP port from the router to the host machine. This is optional for MVP and should be documented as advanced usage.

Port forwarding increases exposure and should only be used when the user understands the security implications.

## NAT Limitations

If both peers are behind restrictive NATs and there is no VPN, no port forwarding and no TURN relay, the connection may fail.

This project must not imply that peer-to-peer connectivity always works across the public internet without infrastructure.

## TURN Out Of Scope

TURN is out of scope for MVP because it requires a relay server with bandwidth costs and operational maintenance. TURN can be added later as an optional feature.

## Security Considerations

- Room URLs should be treated as sensitive because the MVP has no authentication.
- Only expose the signaling port on trusted networks unless intentionally using port forwarding.
- Use VPN mesh networking for remote sessions when possible.
- Do not assume a random room ID is a full security boundary.
- WebRTC media is encrypted in transit, but access control is still required for production-grade usage.
