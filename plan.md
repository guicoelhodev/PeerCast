# Local-First Tauri Screen Share & Video Call MVP

## Implementation Status (2026-07-10)

- Phases 0-7 are complete: local signaling, room management, multi-participant WebRTC, camera, microphone, screen sharing and manual quality selection.
- Rooms use a peer-to-peer mesh. Hosts and guests can all publish camera, microphone and screen media.
- The participant grid is fixed by room size: `1x1`, `1x2`, `2x2` or `3x3`.
- The Tauri server dashboard shows connection URLs and connected participants.
- Next implementation phase: Phase 8, stream statistics and network feedback.

## Project Goal

Build a **local-first desktop application** using **Tauri v2 + Rust + Svelte** that allows users to:

* Create a local room
* Share their screen in high quality
* Start a video call
* Use microphone audio
* Choose the streaming quality manually
* Connect other users directly to the host machine
* Run without a hosted backend or paid infrastructure

The application should behave like a lightweight, self-hosted alternative to Discord screen sharing, focused on **control, quality and low latency**.

The MVP must prioritize a working flow over perfect architecture.

---

# Core Requirement

The host should be able to open the desktop application and start a local session.

Other users should be able to connect to the host through one of these methods:

* LAN IP
* Port forwarding
* Tailscale
* Hamachi
* ZeroTier
* Any similar private networking tool

The MVP must not require a cloud backend, hosted signaling server, paid TURN server or SaaS infrastructure.

---

# Important Networking Constraint

This project is **local-first**, not magically NAT-proof.

The application should work reliably in the following scenarios:

## Required for MVP

* Same local network
* VPN mesh network such as Tailscale, Hamachi or ZeroTier
* Direct access to host IP and port

## Optional / Future

* Public internet access through port forwarding
* Public STUN support
* TURN relay support
* Hosted signaling server

## Explicit Limitation

If two users are behind restrictive NATs and no VPN, port forwarding or TURN server is available, direct peer-to-peer connection may fail.

The agent must document this clearly instead of pretending the app can always connect without infrastructure.

---

# MVP Scope

The MVP should include:

* Tauri desktop app
* Embedded local signaling server
* Room creation
* Room joining
* Peer-to-peer WebRTC connection
* Webcam video
* Microphone audio
* Screen sharing
* Quality selector
* Basic connection status
* Basic stream statistics
* Documentation
* Persistent progress report

---

# Non-Goals for MVP

Do not implement these during the MVP unless all previous phases are complete:

* User accounts
* Authentication
* Cloud deployment
* TURN server
* Mobile app
* Large group calls
* Advanced permissions system
* Chat system
* File transfer
* Recording
* End-to-end encryption beyond default WebRTC encryption
* Auto updater
* Production installer polishing

---

# Tech Stack

## Desktop App

* Tauri v2
* Rust
* Svelte
* TypeScript
* TailwindCSS

## Real-Time Communication

* WebRTC
* WebSocket signaling
* Embedded signaling server started by the Tauri application

## Possible Rust Libraries to Research

The agent must research and decide before implementation:

* WebRTC:

  * `webrtc-rs`
  * browser WebRTC APIs through the Tauri WebView
* WebSocket server:

  * `tokio`
  * `axum`
  * `tokio-tungstenite`
* Screen capture:

  * native Rust capture library
  * browser `getDisplayMedia`
  * platform-specific APIs if needed
* Audio:

  * browser `getUserMedia`
  * Rust `cpal`
* Camera:

  * browser `getUserMedia`
  * Rust-native alternatives if needed

The agent must not assume that browser APIs behave the same inside every Tauri WebView. This must be validated during the research phase.

---

# Recommended Architecture

The preferred MVP architecture is:

```text
                 Host Computer

      ┌─────────────────────────────────────┐
      │              Tauri App              │
      │                                     │
      │  ┌───────────────────────────────┐  │
      │  │          Svelte UI             │  │
      │  │                               │  │
      │  │  Room UI                      │  │
      │  │  Device Controls              │  │
      │  │  Quality Selector             │  │
      │  │  Local Preview                │  │
      │  │  Remote Preview               │  │
      │  └───────────────────────────────┘  │
      │                                     │
      │  ┌───────────────────────────────┐  │
      │  │        Rust Backend            │  │
      │  │                               │  │
      │  │  Embedded WebSocket Server    │  │
      │  │  Local IP Discovery           │  │
      │  │  Room State                   │  │
      │  │  Tauri Commands               │  │
      │  └───────────────────────────────┘  │
      │                                     │
      └──────────────────┬──────────────────┘
                         │
                  WebSocket Signaling
                         │
       ┌─────────────────┴─────────────────┐
       │                                   │
  Remote Peer A                      Remote Peer B

              WebRTC Peer-to-Peer Media
```

---

# Architecture Rule

The signaling server must be embedded into the host desktop app.

The host app should expose a connection URL such as:

```text
http://HOST_IP:PORT/room/ROOM_ID
```

or:

```text
ws://HOST_IP:PORT/signaling?room=ROOM_ID
```

The exact format can be decided during implementation.

The important requirement is that users can connect directly to the host machine without deploying a separate backend.

---

# Quality Control Requirement

The user must be able to manually choose the streaming quality.

The quality selector should support at least:

* Resolution
* FPS
* Bitrate

Suggested presets:

| Preset       | Resolution |   FPS | Suggested Bitrate |
| ------------ | ---------: | ----: | ----------------: |
| Low          |       720p |    30 |          2.5 Mbps |
| Balanced     |      1080p |    30 |            5 Mbps |
| High         |      1080p |    60 |            8 Mbps |
| Ultra        |      1440p |    60 |           14 Mbps |
| Experimental |         4K | 30/60 |          20+ Mbps |


The agent should implement quality control in a realistic way depending on the selected WebRTC approach.

Examples:

* Apply media constraints during capture
* Use `RTCRtpSender.setParameters()` when using browser WebRTC APIs
* Configure encoder parameters if using native Rust WebRTC
* Replace tracks when changing resolution
* Monitor actual sent bitrate, packet loss and latency

The MVP does not need perfect adaptive streaming, but it must expose manual quality selection clearly.

---

# Development Philosophy

The project must be built incrementally.

The agent must not attempt to build the entire app in one step.

Each phase must:

1. Be implemented in small changes.
2. Keep the project buildable.
3. Update documentation.
4. Update `docs/report.md`.
5. Record decisions and blockers.
6. Leave clear resume instructions.

If the context window ends, the next agent must be able to continue by reading `docs/report.md`.

---

# Engineering Loop

For every phase, the agent must follow this loop:

```text
1. Read docs/report.md
2. Confirm current phase
3. Implement only the next unfinished task
4. Build the project
5. Fix errors
6. Update docs/report.md
7. Update docs/decisions.md if a decision was made
8. Update docs/architecture.md if architecture changed
9. Summarize what was completed
10. Stop at a safe checkpoint
```

The agent must never skip the report update.

---

# Repository Structure

Recommended structure:

```text
.
├── apps/
│   └── desktop/
│       ├── src/
│       ├── src-tauri/
│       └── package.json
│
├── crates/
│   └── signaling/
│
├── shared/
│   ├── types/
│   └── protocol/
│
├── docs/
│   ├── report.md
│   ├── architecture.md
│   ├── decisions.md
│   ├── networking.md
│   └── quality.md
│
├── README.md
└── package.json
```

The agent may adjust this structure if technically necessary, but it must document the reason in `docs/decisions.md`.

---

# PHASE 0 — Research, Spikes & Architecture

## Goal

Avoid building the wrong architecture.

Before implementing the app, validate the most important technical assumptions.

## Tasks

* Verify if Tauri WebView supports:

  * `getUserMedia`
  * `getDisplayMedia`
  * WebRTC APIs
  * audio/video permissions
* Decide whether WebRTC should live:

  * mostly in the frontend using browser APIs
  * mostly in Rust using `webrtc-rs`
  * hybrid approach
* Research screen capture options.
* Research audio capture options.
* Research camera capture options.
* Research how to expose the local server from Tauri.
* Research local IP discovery.
* Define the signaling protocol.
* Define quality preset strategy.
* Document known platform limitations.

## Deliverables

* `docs/architecture.md`
* `docs/decisions.md`
* `docs/networking.md`
* `docs/quality.md`
* Initial `docs/report.md`

## Acceptance Criteria

* Architecture is documented.
* Key risks are documented.
* The agent knows which APIs/libraries will be used.
* No production code is written before the main decisions are documented.

---

# PHASE 1 — Project Bootstrap

## Goal

Create a working Tauri v2 app foundation.

## Tasks

* Initialize the repository.
* Configure Tauri v2.
* Configure Svelte.
* Configure TypeScript.
* Configure TailwindCSS.
* Configure Rust formatting.
* Configure frontend formatting.
* Configure linting if practical.
* Create base app layout.
* Create documentation folder.
* Create initial report file.
* Add basic dev scripts.

## Deliverable

The desktop app opens successfully.

## Acceptance Criteria

* App starts in development mode.
* App builds without errors.
* Basic UI is visible.
* `docs/report.md` exists.
* `docs/architecture.md` exists.
* `docs/decisions.md` exists.

---

# PHASE 2 — Embedded Signaling Server

## Goal

Run a WebSocket signaling server inside the Tauri host app.

## Tasks

* Start local WebSocket server from Rust.
* Pick a default port.
* Handle port conflicts gracefully.
* Display the host IP and port in the UI.
* Create room IDs.
* Allow remote clients to join a room.
* Handle peer join.
* Handle peer leave.
* Forward signaling messages.
* Log connection events.
* Add basic error handling.

## Signaling Messages

The protocol should support at least:

```ts
type SignalingMessage =
  | { type: "join-room"; roomId: string; peerId: string }
  | { type: "peer-joined"; roomId: string; peerId: string }
  | { type: "peer-left"; roomId: string; peerId: string }
  | { type: "offer"; roomId: string; from: string; to: string; sdp: string }
  | { type: "answer"; roomId: string; from: string; to: string; sdp: string }
  | { type: "ice-candidate"; roomId: string; from: string; to: string; candidate: unknown }
  | { type: "error"; message: string };
```

The exact type can be adjusted, but the protocol must be documented in `shared/protocol` or `docs/architecture.md`.

## Deliverable

Two clients can join the same room and exchange signaling messages.

## Acceptance Criteria

* Host app starts the signaling server automatically.
* UI displays connection URL.
* Remote client can connect to the host signaling server.
* Join/leave events work.
* Signaling messages are relayed correctly.
* No media transmission is required yet.

---

# PHASE 3 — WebRTC Peer Connection

## Goal

Establish a peer-to-peer WebRTC connection.

## Tasks

* Create peer IDs.
* Create `RTCPeerConnection`.
* Exchange SDP offer.
* Exchange SDP answer.
* Exchange ICE candidates.
* Track connection state.
* Show connection state in the UI.
* Handle disconnect.
* Clean up peer connection.

## Deliverable

Two clients can establish a WebRTC peer connection.

## Acceptance Criteria

* Offer/answer exchange works.
* ICE candidate exchange works.
* Connection state reaches connected.
* Disconnection is visible in the UI.
* Errors are shown clearly.

---

# PHASE 4 — Camera Video

## Goal

Add webcam video call support.

## Tasks

* Request camera permission.
* Select camera device if possible.
* Show local camera preview.
* Add camera track to peer connection.
* Receive remote camera track.
* Show remote video.
* Stop camera cleanly.

## Deliverable

Basic video call works.

## Acceptance Criteria

* Local camera preview works.
* Remote video appears.
* Camera can be stopped.
* Camera resources are released after stopping.

---

# PHASE 5 — Microphone Audio

## Goal

Add microphone audio support.

## Tasks

* Request microphone permission.
* Select microphone device if possible.
* Add audio track to peer connection.
* Receive remote audio.
* Implement mute.
* Implement unmute.
* Show microphone state in UI.
* Clean up audio resources.

## Deliverable

Video call with audio works.

## Acceptance Criteria

* Microphone audio is transmitted.
* Remote audio is heard.
* Mute/unmute works.
* Device cleanup works.

---

# PHASE 6 — Screen Sharing

## Status

Complete (2026-07-10).

Screen sharing is available to every participant through `getDisplayMedia()`. It replaces the outgoing camera video track, sends system audio in a separate track when the chosen source provides it, keeps microphone audio independent, restores the camera after sharing ends when applicable, and is covered by the multi-participant signaling flow.

## Goal

Implement screen sharing.

## Tasks

* Add screen share button.
* Request screen capture permission.
* Allow choosing display/window if supported.
* Capture screen.
* Replace camera video track with screen track.
* Restore camera when screen sharing stops.
* Handle user canceling screen picker.
* Handle screen sharing ending externally.
* Show local screen preview.
* Show remote screen stream.

## Deliverable

Screen sharing works.

## Acceptance Criteria

* User can start screen sharing.
* Remote peer receives screen stream.
* User can stop screen sharing.
* Camera can be restored after screen sharing.
* App does not crash if screen capture is denied.

---

# PHASE 7 — Manual Streaming Quality

## Status

Complete (2026-07-10).

The UI provides documented presets while a screen share is active. Resolution and FPS are requested as display-capture constraints, and bitrate/FPS are applied to video senders when the current browser/WebView supports those WebRTC parameters.

## Goal

Allow the user to control screen sharing quality.

## Tasks

* Add quality selector to the UI.
* Add resolution presets.
* Add FPS presets.
* Add bitrate presets.
* Apply selected quality before starting screen sharing.
* Allow changing quality while streaming if technically feasible.
* If live quality changes are not feasible, document the limitation and restart the stream with the new settings.
* Show current quality in the UI.

## Presets

```ts
const qualityPresets = [
  {
    id: "low",
    label: "Low",
    width: 1280,
    height: 720,
    fps: 30,
    bitrate: 2_500_000
  },
  {
    id: "balanced",
    label: "Balanced",
    width: 1920,
    height: 1080,
    fps: 30,
    bitrate: 5_000_000
  },
  {
    id: "high",
    label: "High",
    width: 1920,
    height: 1080,
    fps: 60,
    bitrate: 8_000_000
  },
  {
    id: "ultra",
    label: "Ultra",
    width: 2560,
    height: 1440,
    fps: 60,
    bitrate: 14_000_000
  },
  {
    id: "experimental-4k",
    label: "4K Experimental",
    width: 3840,
    height: 2160,
    fps: 30,
    bitrate: 20_000_000
  }
];
```

The agent may adjust these values based on implementation constraints, but must document changes.

## Deliverable

The user can choose screen sharing quality.

## Acceptance Criteria

* Quality selector is visible.
* Selected quality is applied to screen sharing.
* User can choose resolution.
* User can choose FPS.
* User can choose bitrate.
* Current active quality is visible.
* Limitations are documented.

---

# PHASE 8 — Stream Statistics & Network Feedback

## Goal

Show whether the selected quality is actually working well.

## Tasks

* Display connection state.
* Display approximate bitrate.
* Display packet loss if available.
* Display latency/round-trip time if available.
* Display selected resolution.
* Display selected FPS.
* Display warning when network quality is poor.
* Log WebRTC stats for debugging.

## Deliverable

User can understand stream health.

## Acceptance Criteria

* UI shows useful stream stats.
* Poor connection warning appears when needed.
* Debug information is available for troubleshooting.

---

# PHASE 9 — UX Improvements

## Goal

Make the MVP usable by non-technical users.

## Tasks

* Create home screen.
* Create host room flow.
* Create join room flow.
* Show local IP.
* Show room link.
* Add copy link button.
* Add device selectors.
* Add quality selector.
* Add call controls.
* Add clear error messages.
* Add loading states.
* Add empty states.

## Deliverable

Usable MVP interface.

## Acceptance Criteria

* Host can create a room easily.
* Guest can join a room easily.
* User understands connection status.
* User understands how to share the connection URL.
* User can control camera, mic and screen share.

---

# PHASE 10 — Stability & Recovery

## Goal

Make the app reliable enough for real usage.

## Tasks

* Handle peer disconnects.
* Handle signaling disconnects.
* Implement reconnect attempt.
* Implement ICE restart if feasible.
* Clean up media tracks.
* Clean up WebSocket connections.
* Avoid duplicate peer connections.
* Avoid memory leaks.
* Improve error handling.
* Improve logs.

## Deliverable

Stable MVP.

## Acceptance Criteria

* App handles disconnects gracefully.
* App does not keep camera/mic active after leaving.
* App does not crash on failed connection.
* User receives useful error messages.

---

# PHASE 11 — Polish

## Goal

Prepare the app for continued development.

## Tasks

* Improve visual design.
* Add settings page.
* Add keyboard shortcuts.
* Add theme support.
* Add icons.
* Improve README.
* Add troubleshooting guide.
* Add build instructions.
* Add known limitations section.

## Deliverable

Clean MVP ready for future iteration.

---

# Documentation Requirements

The following files must exist:

```text
docs/
├── report.md
├── architecture.md
├── decisions.md
├── networking.md
└── quality.md
```

---

# docs/report.md Specification

`docs/report.md` is mandatory.

It must always reflect the current state of the project.

Template:

```md
# Project Report

## Current Phase

Phase 0

## Status

Not Started

## Completed

- None

## In Progress

- Research and architecture

## Remaining

- Bootstrap
- Embedded signaling server
- WebRTC connection
- Camera
- Microphone
- Screen sharing
- Manual quality selection
- Stream statistics
- UX
- Stability
- Polish

## Last Completed Task

None

## Current Task

Validate Tauri WebView media/WebRTC support.

## Next Task

Choose WebRTC implementation strategy.

## Known Issues

- None yet.

## Technical Decisions

- Local-first architecture.
- Embedded signaling server.
- No external backend for MVP.
- No TURN server for MVP.

## Technical Debt

- None yet.

## Blockers

- Need to validate screen capture support in Tauri WebView.

## Risks

- Screen capture support may vary by OS.
- WebRTC support may vary depending on WebView.
- Direct connection may fail behind restrictive NAT without VPN, port forwarding or TURN.

## Resume Instructions

Read this file before making changes.
Continue from the current task.
Do not restart completed work.
Update this file before ending the session.
```

---

# docs/decisions.md Specification

Use this format for every important decision:

```md
# Decision: Use Embedded Signaling Server

## Status

Accepted

## Context

The project should work without a hosted backend.

## Decision

The signaling server will run inside the host Tauri application.

## Consequences

Users can connect directly to the host, but the host must be reachable through LAN, VPN or port forwarding.
```

---

# docs/networking.md Specification

Must explain:

* LAN usage
* Host IP
* Port forwarding
* VPN mesh usage
* NAT limitations
* Why TURN is out of scope
* Why remote users may fail to connect without proper routing
* Security considerations when exposing ports

---

# docs/quality.md Specification

Must explain:

* Available quality presets
* Resolution
* FPS
* Bitrate
* How quality is applied
* Whether quality can change live
* Known limitations
* Recommended upload speed for each preset

Suggested upload speed recommendations:

| Preset  | Minimum Upload |
| ------- | -------------: |
| 720p30  |         5 Mbps |
| 1080p30 |        10 Mbps |
| 1080p60 |        15 Mbps |
| 1440p60 |        25 Mbps |
| 4K      |       40+ Mbps |

---

# Agent Rules

The agent must:

* Work phase by phase.
* Never skip Phase 0.
* Never implement unrelated features.
* Never rewrite the whole project without reason.
* Prefer simple working code.
* Avoid premature abstractions.
* Keep the app buildable after every phase.
* Update `docs/report.md` after every meaningful change.
* Document every important technical decision.
* Clearly document blockers.
* Clearly document limitations.
* Stop at safe checkpoints.
* Prefer MVP completion over perfect architecture.

---

# Stop Conditions

The agent should stop and update the report if:

* A phase is complete.
* A major blocker is found.
* A technical assumption is false.
* The app cannot build.
* The implementation path needs a decision.
* The context is getting too large.

Before stopping, the agent must update `docs/report.md`.

---

# MVP Definition of Done

The MVP is complete when:

* The host can start the desktop app.
* The embedded signaling server starts automatically.
* The host can create a room.
* The host can see a local connection URL.
* Another user can connect to the host.
* WebRTC peer connection works.
* Camera video works.
* Microphone audio works.
* Screen sharing works.
* The user can choose resolution.
* The user can choose FPS.
* The user can choose bitrate.
* 1080p30 screen sharing works under good network conditions.
* 1080p60 screen sharing works when bandwidth is sufficient.
* Basic stream statistics are visible.
* Disconnects are handled gracefully.
* Documentation is updated.
* `docs/report.md` accurately reflects the final state.

---

# First Instruction for the Agent

Start by creating the documentation and performing Phase 0.

Do not write the full app yet.

First validate the technical foundation:

1. Tauri media support.
2. WebRTC strategy.
3. Screen capture strategy.
4. Embedded signaling strategy.
5. Quality control strategy.

After Phase 0, update `docs/report.md` and proceed to Phase 1.
