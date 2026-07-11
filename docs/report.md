# Project Report

## Current Phase

Phase 5 (complete) — Moving to Phase 6

## Status

All MVP features through Phase 5 are implemented, tested and build-validated. Multi-guest support added beyond original MVP scope. E2E test suite passes with 1 host + 3 guests.

## Completed

### Phase 0 — Research, Spikes & Architecture
- Created documentation folder and initial docs.
- Documented MVP architecture, technical decisions, networking model, NAT limitations, and streaming quality strategy.
- Selected frontend browser WebRTC APIs as the first MVP strategy.
- Selected embedded Rust WebSocket signaling server as the local-first signaling strategy.

### Phase 1 — Project Bootstrap
- Bootstrapped Tauri v2 app.
- Configured SvelteKit + TypeScript template.
- Added TailwindCSS via `@tailwindcss/vite`.
- Added base MVP landing UI.
- Simplified project structure by moving the Tauri/Svelte app to the repository root.
- Added Rust formatting config, frontend Prettier config, and `.gitignore`.
- Set Linux bundle target to AppImage only.
- Approved `esbuild` build script for pnpm.

### Phase 2 — Embedded Signaling Server
- Added `src-tauri/src/signaling.rs` with in-memory room state and `/ws/:room_id` WebSocket relay.
- Added peer-scoped relay behavior (no self-echo).
- Added `src-tauri/src/network.rs` for local IP discovery.
- Started the signaling server from the Tauri backend during app setup.
- Added Tauri commands: `signaling_status`, `create_room`, `stop_room`.
- Selected default local signaling port `17777`.
- Updated landing UI to show signaling status, local endpoint and room WebSocket URL.
- Runtime-validated the embedded WebSocket relay with two Node WebSocket clients.
- Added standalone `signal-server` binary for E2E testing.

### Phase 3 — WebRTC Peer Connection
- Added typed frontend signaling protocol with `peerId` routing for multi-peer support.
- Implemented host/guest peer connection flow over embedded WebSocket signaling.
- Multi-guest support: host creates one `RTCPeerConnection` per guest, keyed by `peerId`.
- Auto-accept guests (removed accept/reject UI).
- Broadcast signaling server routes all messages to all clients; `peerId` filtering in frontend.
- Connection state tracked per-guest, shown as peer count in UI.
- Runtime-tested with 1 host + 3 guests via Playwright E2E test.

### Phase 4 — Camera Video
- Added camera capture with `getUserMedia({ video: true, audio: true })`.
- Added local camera preview in video grid.
- Wired local camera tracks into all host peer connections.
- Dynamic video grid: 1 col (mobile), 2 cols (sm), 3 cols (lg).
- Guest receives remote host stream without sending own camera (streamer→viewer model).

### Phase 5 — Microphone Audio
- Microphone audio captured alongside camera.
- Mic mute/unmute toggle via `track.enabled`.
- Hidden `<audio>` elements per guest peer for remote audio playback.
- Mic state display in devices panel.

### UX Improvements (partial Phase 9)
- Single "Join" button with auto-detect: checks `role=host` in URL.
- Auto-join on page load when `?room=` param present (with loading spinner).
- Copy button on Connection input field.
- Guest stream URL box in Tauri view (without `role=host`).
- "Stop Room" button in Tauri view to disconnect all guests.
- Loading state (`isConnecting`) with spinner during connection.
- Host header shows viewer count.

## In Progress

- None currently.

## Remaining

- Phase 6: Screen sharing.
- Phase 7: Manual quality selection.
- Phase 8: Stream statistics.
- Phase 9: Remaining UX (device selectors, settings page, keyboard shortcuts).
- Phase 10: Stability and recovery (disconnect handling, ICE restart, memory cleanup).
- Phase 11: Polish (visual design, icons, README, troubleshooting guide).

## Last Completed Task

Multi-guest WebRTC support with auto-accept, `peerId` routing, dynamic video grid, and E2E test coverage (1 host + 3 guests verified).

## Current Task

None — ready to start Phase 6.

## Next Task

Phase 6: Screen sharing via `getDisplayMedia()`.

## Known Issues

- `pnpm tauri build` AppImage bundling uses Tauri-downloaded `linuxdeploy` whose embedded `strip` fails on Arch with modern `.relr.dyn` ELF sections. Workaround: manual AppImage generation with patched `linuxdeploy`.
- `getDisplayMedia()` may not work consistently across all Tauri WebView platforms.
- Direct connections can fail behind restrictive NAT without VPN, port forwarding or TURN.
- High-quality screen sharing may exceed upload bandwidth or encoding capacity.

## Technical Decisions

- Local-first architecture.
- Embedded signaling server inside Tauri process on port `17777`.
- No external backend, TURN server, or authentication for MVP.
- WebRTC in Svelte frontend using browser APIs.
- Rust handles signaling, room state, IP discovery via Tauri commands.
- Signaling rooms are in-memory, ephemeral. No persistence.
- WebSocket broadcasting with frontend-side `peerId` routing for multi-peer.
- Multi-guest: host creates one `RTCPeerConnection` per guest. Guests receive-only (no camera upload).
- Auto-accept: no accept/reject flow; guests join transparently.
- Camera and microphone captured together via `getUserMedia`.

## Technical Debt

- Frontend signaling protocol (`src/lib/signaling-protocol.ts`) tightly coupled to Svelte page.
- No renegotiation queue for media changes.
- `startCamera` re-sends offers to all connected guests (could be optimized).
- `sendDataPing` is a no-op in multi-guest mode (data channel removed).

## Blockers

- None.

## Risks

- `getDisplayMedia()` may not work consistently across Tauri WebView platforms.
- Screen capture behavior varies by OS: Linux (WebKitGTK + portals), Windows (WebView2), macOS (WKWebView).
- Multi-guest bandwidth scaling is untested beyond localhost.

## Verification

- `pnpm check` passed (0 errors, 0 warnings).
- `pnpm build` passed.
- `pnpm format:check` passed.
- `cargo check` passed in `src-tauri`.
- `cargo fmt --check` passed in `src-tauri`.
- E2E test: 1 host + 3 guests establish WebRTC connections (Playwright, ~6s).

## Resume Instructions

Read this file before making changes.
Continue from Phase 6 (screen sharing).
The app supports multi-guest streaming: host with camera/audio, guests view-only.
All prior phases are complete and tested. Do not restart completed work.

Update this file before ending the session.