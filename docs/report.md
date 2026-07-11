# Project Report

## Current Phase

Phase 7 (complete) — Ready for Phase 8

## Status

All MVP features through Phase 7 are implemented, tested and build-validated. Rooms use a peer-to-peer mesh: host and guests can publish camera, microphone and screen media. Screen sharing includes quality presets available while sharing. E2E coverage validates 1 host + 3 guests and guest-camera delivery to the host.

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
- Added Tauri commands: `signaling_status`, `create_room`, `stop_room`, `room_participants`.
- Selected default local signaling port `17777`.
- Updated landing UI to show signaling status, local endpoint and room WebSocket URL.
- Runtime-validated the embedded WebSocket relay with two Node WebSocket clients.
- Added standalone `signal-server` binary for E2E testing.

### Phase 3 — WebRTC Peer Connection
- Added typed frontend signaling protocol with source and target peer IDs for multi-peer routing.
- Implemented a room mesh over embedded WebSocket signaling: every existing participant connects to a new participant.
- Auto-accept participants (removed accept/reject UI).
- Broadcast signaling server routes all messages to all clients; clients filter target peer IDs locally.
- Connection state tracked per-guest, shown as peer count in UI.
- Runtime-tested with 1 host + 3 guests via Playwright E2E test.

### Phase 4 — Camera Video
- Added camera capture with `getUserMedia({ video: true, audio: true })`.
- Added local camera preview in video grid.
- Wired local camera tracks into every peer connection, for hosts and guests.
- Camera start always renegotiates existing connections, including connections whose state is still settling.
- Dynamic video grid: `1x1` for one participant, `1x2` for two, `2x2` for three or four, and `3x3` for five or more.

### Phase 5 — Microphone Audio
- Microphone audio captured alongside camera.
- Mic mute/unmute toggle via `track.enabled`.
- Hidden `<audio>` elements per guest peer for remote audio playback.
- Mic state display in devices panel.
- Active and muted microphone controls have distinct visual states.
- Per-participant volume is a vertical control in the lower-right video overlay.

### Phase 6 — Screen Sharing
- Added `getDisplayMedia()` screen capture for every participant, not only the host.
- Replaces the outgoing camera video track while sharing and restores the camera when sharing stops.
- Handles picker cancellation and screen capture ending externally.
- Shows local and remote screen streams in the participant grid.
- Uses WebRTC renegotiation when a peer has no existing video sender.
- Requests system audio with the selected display source and publishes it as a separate track when the browser provides one; microphone audio remains independent.

### Phase 7 — Manual Streaming Quality
- Added presets for 720p30, 1080p30, 1080p60, 1440p60 and experimental 4K30.
- Applies ideal capture constraints for resolution and frame rate before screen sharing.
- Attempts live updates through track constraints and `RTCRtpSender.setParameters()` for bitrate and frame rate.
- Shows whether a selection applies to the next share or is being updated live.

### UX Improvements (partial Phase 9)
- Single "Join" button with auto-detect: checks `role=host` in URL.
- Auto-join on page load when `?room=` param present (with loading spinner).
- Copy button on Connection input field.
- Guest stream URL box in Tauri view (without `role=host`).
- "Stop Room" button in Tauri view to disconnect all guests.
- Loading state (`isConnecting`) with spinner during connection.
- Room header shows connected participant count.
- Tauri room dashboard now displays room URLs and a periodically refreshed list of connected participants.

## In Progress

- None currently.

## Remaining

- Phase 8: Stream statistics.
- Phase 9: Remaining UX (device selectors, settings page, keyboard shortcuts).
- Phase 10: Stability and recovery (disconnect handling, ICE restart, memory cleanup).
- Phase 11: Polish (visual design, icons, README, troubleshooting guide).

## Last Completed Task

Manual screen-share quality selection with live presets.

## Current Task

None — Phase 7 is complete.

## Next Task

Phase 8: Stream statistics and network feedback.

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
- Multi-participant mesh: every participant has one `RTCPeerConnection` per other participant and can publish media.
- Auto-accept: no accept/reject flow; guests join transparently.
- Camera and microphone captured together via `getUserMedia`.

## Technical Debt

- Frontend signaling protocol (`src/lib/signaling-protocol.ts`) tightly coupled to Svelte page.
- No renegotiation queue or perfect-negotiation collision handling for simultaneous media changes.
- Mesh bandwidth grows with the number of participants and is not suitable for large rooms.
- Participant identity is an ephemeral peer-ID prefix; there are no user accounts or display names.
- WebViews can ignore capture constraints or sender bitrate parameters; the selected quality is a target, not a guarantee.
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
- E2E test: 1 host + 3 guests establish WebRTC connections, and a guest camera track reaches the host (Playwright).

## Resume Instructions

Read this file before making changes.
Continue from Phase 8 (stream statistics and network feedback).
The app supports a multi-participant mesh: every participant can use camera, microphone and screen sharing.
All prior phases are complete and tested. Do not restart completed work.

Update this file before ending the session.
