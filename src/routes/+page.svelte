<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import "iconify-icon";

  import {
    parseSignalMessage,
    serializeSignalMessage,
    type SignalMessage,
  } from "$lib/signaling-protocol";

  type SignalingStatus = {
    isRunning: boolean;
    port: number;
    localIp: string | null;
    url: string | null;
  };

  type RoomInfo = {
    roomId: string;
    signalingUrl: string;
  };

  let signalingStatus: SignalingStatus | null = null;
  let room: RoomInfo | null = null;
  let errorMessage = "";
  let infoMessage = "";
  let isCreatingRoom = false;
  let joinUrl = "";
  let browserHostUrl = "";
  let peerRole: "host" | "guest" | null = null;
  let signalingConnectionState = "Disconnected";
  let peerConnectionState = "New";
  let dataChannelState = "Closed";
  let lastDataMessage = "None";
  let cameraState = "Stopped";
  let micState = "Stopped";
  let webrtcAvailable = true;
  let isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  let ws: WebSocket | null = null;
  let peerConnection: RTCPeerConnection | null = null;
  let dataChannel: RTCDataChannel | null = null;
  let localStream: MediaStream | null = null;
  let remoteStream: MediaStream | null = null;
  let localVideo: HTMLVideoElement | null = null;
  let remoteVideo: HTMLVideoElement | null = null;
  let remoteAudio: HTMLAudioElement | null = null;
  let hasSentOffer = false;
  let pendingGuest = false;

  let copied = false;

  onMount(async () => {
    webrtcAvailable = typeof RTCPeerConnection !== "undefined";

    const params = new URLSearchParams(window.location.search);
    const roomUrl = params.get("room");
    const role = params.get("role");

    if (roomUrl) {
      joinUrl = roomUrl;
      browserHostUrl = buildBrowserRoomUrl(roomUrl);

      if (role === "host" && webrtcAvailable) {
        await startHost(roomUrl);
      }
    }

    if (isTauri) {
      try {
        signalingStatus = await invoke<SignalingStatus>("signaling_status");
      } catch (error) {
        infoMessage = `Signaling server error: ${String(error)}`;
      }
    }
  });

  async function createLocalRoom() {
    isCreatingRoom = true;
    errorMessage = "";
    infoMessage = "";

    try {
      room = await invoke<RoomInfo>("create_room");
      joinUrl = room.signalingUrl;
      browserHostUrl = buildBrowserRoomUrl(room.signalingUrl);
      signalingStatus = await invoke<SignalingStatus>("signaling_status");
      infoMessage = "Room created. Open the browser URL in Chrome/Firefox to start streaming.";
    } catch (error) {
      errorMessage = `Failed to create room: ${String(error)}`;
    } finally {
      isCreatingRoom = false;
    }
  }

  function buildBrowserRoomUrl(signalingUrl: string) {
    const url = new URL("http://localhost:1420/");
    url.searchParams.set("role", "host");
    url.searchParams.set("room", signalingUrl);
    return url.toString();
  }

  function buildGuestUrl(signalingUrl: string) {
    const url = new URL("http://localhost:1420/");
    url.searchParams.set("room", signalingUrl);
    return url.toString();
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      // ignore
    }
  }

  async function startHost(signalingUrl: string) {
    closeConnection();
    peerRole = "host";
    hasSentOffer = false;
    pendingGuest = false;

    if (webrtcAvailable) {
      setupPeerConnection();
      dataChannel = peerConnection?.createDataChannel("control") ?? null;
      bindDataChannel(dataChannel);
    }

    await connectSignaling(signalingUrl);
    infoMessage = webrtcAvailable
      ? "Host ready. Waiting for a guest to join."
      : "Room created (signaling only). Open the room URL in Chrome/Firefox to stream.";
  }

  async function hostRoomInThisBrowser() {
    if (!joinUrl.trim()) {
      errorMessage = "Paste a room WebSocket URL before hosting.";
      return;
    }

    errorMessage = "";
    infoMessage = "";
    browserHostUrl = buildBrowserRoomUrl(joinUrl.trim());
    await startHost(joinUrl.trim());
  }

  async function joinRoom() {
    if (!joinUrl.trim()) {
      errorMessage = "Paste a room WebSocket URL before joining.";
      return;
    }

    closeConnection();
    errorMessage = "";
    infoMessage = "";
    peerRole = "guest";

    if (webrtcAvailable) {
      setupPeerConnection();
    }

    await connectSignaling(joinUrl.trim());

    if (webrtcAvailable) {
      sendSignal({ type: "ready" });
      infoMessage = "Guest ready. Waiting for host offer.";
    } else {
      infoMessage =
        "WebSocket connected but WebRTC not available. Open this URL in a browser.";
    }
  }

  function setupPeerConnection() {
    if (!webrtcAvailable) {
      errorMessage =
        "WebRTC not available in this context. Open http://localhost:1420 in a browser.";
      return;
    }
    peerConnection = new RTCPeerConnection({ iceServers: [] });
    peerConnectionState = peerConnection.connectionState;

    peerConnection.onconnectionstatechange = () => {
      peerConnectionState = peerConnection?.connectionState ?? "closed";
    };

    peerConnection.onicecandidate = (event) => {
      if (event.candidate) {
        sendSignal({ type: "ice", candidate: event.candidate.toJSON() });
      }
    };

    peerConnection.ondatachannel = (event) => {
      dataChannel = event.channel;
      bindDataChannel(dataChannel);
    };

    peerConnection.ontrack = (event) => {
      remoteStream ??= new MediaStream();
      remoteStream.addTrack(event.track);
      attachVideoStreams();
    };

    addLocalTracksToPeerConnection();
  }

  function bindDataChannel(channel: RTCDataChannel | null) {
    if (!channel) {
      dataChannelState = "Closed";
      return;
    }

    dataChannelState = channel.readyState;
    channel.onopen = () => {
      dataChannelState = channel.readyState;
      channel.send(`hello from ${peerRole}`);
    };
    channel.onclose = () => {
      dataChannelState = channel.readyState;
    };
    channel.onmessage = (event) => {
      lastDataMessage = String(event.data);
    };
  }

  async function connectSignaling(signalingUrl: string) {
    await new Promise<void>((resolve, reject) => {
      ws = new WebSocket(signalingUrl);
      signalingConnectionState = "Connecting";

      ws.onopen = () => {
        signalingConnectionState = "Connected";
        resolve();
      };
      ws.onerror = () => {
        signalingConnectionState = "Error";
        reject(new Error("WebSocket connection failed"));
      };
      ws.onclose = () => {
        signalingConnectionState = "Disconnected";
      };
      ws.onmessage = async (event) => {
        await handleSignalMessage(String(event.data));
      };
    });
  }

  async function handleSignalMessage(data: string) {
    const message = parseSignalMessage(data);
    if (!message) return;

    if (message.type === "ready" && peerRole === "host" && !hasSentOffer) {
      if (!peerConnection) {
        errorMessage =
          "Guest tried to connect but WebRTC is not available in this WebView.";
        return;
      }

      if (pendingGuest) {
        sendSignal({ type: "reject" });
        return;
      }

      pendingGuest = true;
      infoMessage = "Guest wants to connect. Accept or reject below.";
      return;
    }

    if (message.type === "guest-connected" && peerRole === "guest") {
      infoMessage = "Host accepted. Waiting for offer.";
      return;
    }

    if (message.type === "reject" && peerRole === "guest") {
      errorMessage = "Host rejected the connection.";
      closeConnection();
      return;
    }

    if (!peerConnection) return;

    if (message.type === "offer" && peerRole === "guest") {
      await peerConnection.setRemoteDescription(message.description);
      const answer = await peerConnection.createAnswer();
      await peerConnection.setLocalDescription(answer);
      sendSignal({ type: "answer", description: answer });
      infoMessage = "Answer sent. Waiting for peer connection.";
      return;
    }

    if (message.type === "answer" && peerRole === "host") {
      await peerConnection.setRemoteDescription(message.description);
      infoMessage = "Answer received. Waiting for peer connection.";
      return;
    }

    if (message.type === "ice") {
      await peerConnection.addIceCandidate(message.candidate);
    }
  }

  async function acceptGuest() {
    if (!pendingGuest || !peerConnection) return;

    pendingGuest = false;
    hasSentOffer = true;
    sendSignal({ type: "guest-connected" });
    await sendHostOffer();
    errorMessage = "";
    infoMessage = "";
  }

  function rejectGuest() {
    if (!pendingGuest) return;

    pendingGuest = false;
    sendSignal({ type: "reject" });
    errorMessage = "";
    infoMessage = "";
  }

  function sendSignal(message: SignalMessage) {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(serializeSignalMessage(message));
    }
  }

  async function sendHostOffer() {
    if (!peerConnection || peerRole !== "host") {
      return;
    }

    const offer = await peerConnection.createOffer();
    await peerConnection.setLocalDescription(offer);
    sendSignal({ type: "offer", description: offer });
  }

  function sendDataPing() {
    if (dataChannel?.readyState !== "open") {
      errorMessage = "Data channel is not open yet.";
      return;
    }

    dataChannel.send(
      `ping from ${peerRole} at ${new Date().toLocaleTimeString()}`,
    );
  }

  async function startCamera() {
    errorMessage = "";

    try {
      localStream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: true,
      });
      cameraState = "Running";
      micState = "Active";
      attachVideoStreams();
      addLocalTracksToPeerConnection();

      if (peerRole === "host" && hasSentOffer) {
        await sendHostOffer();
      }
    } catch (error) {
      cameraState = "Error";
      micState = "Error";
      const hint =
        window.location.hostname !== "localhost"
          ? " Access via http://localhost:5173 or use pnpm tauri:dev."
          : " Grant camera and microphone permission in your browser and try again.";
      errorMessage = `Failed to start camera: ${String(error)}.${hint}`;
    }
  }

  function toggleMic() {
    if (!localStream) return;

    const audioTracks = localStream.getAudioTracks();

    if (audioTracks.length === 0) {
      micState = "No track";
      return;
    }

    const currentlyMuted = !audioTracks[0].enabled;
    audioTracks.forEach((track) => (track.enabled = currentlyMuted));
    micState = currentlyMuted ? "Active" : "Muted";
  }

  function stopCamera() {
    localStream?.getTracks().forEach((track) => track.stop());
    localStream = null;
    cameraState = "Stopped";
    micState = "Stopped";
    attachVideoStreams();
  }

  function addLocalTracksToPeerConnection() {
    if (!peerConnection || !localStream) return;
    const existingTrackIds = new Set(
      peerConnection.getSenders().map((sender) => sender.track?.id),
    );
    for (const track of localStream.getTracks()) {
      if (!existingTrackIds.has(track.id)) {
        peerConnection.addTrack(track, localStream);
      }
    }
  }

  function attachVideoStreams() {
    if (localVideo) localVideo.srcObject = localStream;
    if (remoteVideo) remoteVideo.srcObject = remoteStream;
    if (remoteAudio) remoteAudio.srcObject = remoteStream;
  }

  function closeConnection() {
    ws?.close();
    dataChannel?.close();
    peerConnection?.close();
    ws = null;
    dataChannel = null;
    peerConnection = null;
    peerRole = null;
    hasSentOffer = false;
    pendingGuest = false;
    signalingConnectionState = "Disconnected";
    peerConnectionState = "Closed";
    dataChannelState = "Closed";
  }
</script>

<main class="flex h-screen overflow-hidden bg-slate-950 text-slate-100">
  <!-- LEFT SIDEBAR -->
  <aside class="flex w-72 shrink-0 flex-col border-r border-slate-800 bg-slate-900/60">
    <!-- App header -->
    <div class="flex h-12 items-center justify-between border-b border-slate-800 px-4">
      <span class="text-sm font-semibold tracking-wide">Streaming Open</span>
      <span class="rounded {isTauri ? 'bg-amber-400/20 text-amber-300' : 'bg-emerald-400/20 text-emerald-300'} px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider">
        {isTauri ? 'Server' : 'Client'}
      </span>
    </div>

    <div class="flex-1 space-y-4 overflow-y-auto p-3">
      <!-- ALERTS -->
      {#if errorMessage}
        <div class="rounded-lg border border-red-400/30 bg-red-400/10 px-3 py-2">
          <p class="text-xs text-red-200">{errorMessage}</p>
        </div>
      {/if}
      {#if infoMessage}
        <div class="rounded-lg border border-cyan-400/30 bg-cyan-400/10 px-3 py-2">
          <p class="text-xs text-cyan-200">{infoMessage}</p>
        </div>
      {/if}

      {#if pendingGuest}
        <div class="rounded-lg border border-amber-400/40 bg-amber-400/10 px-3 py-3">
          <p class="mb-2 text-xs font-semibold uppercase tracking-wider text-amber-200">Incoming request</p>
          <div class="flex gap-2">
            <button
              class="flex flex-1 items-center justify-center gap-1.5 rounded bg-emerald-500 px-3 py-2 text-xs font-semibold text-white transition hover:bg-emerald-400"
              type="button" onclick={acceptGuest}>
              <iconify-icon icon="mdi:check" class="text-sm"></iconify-icon>
              Accept
            </button>
            <button
              class="flex flex-1 items-center justify-center gap-1.5 rounded border border-red-400/30 px-3 py-2 text-xs font-semibold text-red-300 transition hover:bg-red-400/10"
              type="button" onclick={rejectGuest}>
              <iconify-icon icon="mdi:close" class="text-sm"></iconify-icon>
              Reject
            </button>
          </div>
        </div>
      {/if}

      {#if isTauri}
        <!-- ========== TAURI SERVER VIEW ========== -->
        <div>
          <p class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500">Signaling Server</p>
          <button
            class="flex w-full items-center justify-center gap-2 rounded-lg bg-amber-500 px-4 py-2.5 text-sm font-semibold text-slate-950 shadow-lg shadow-amber-500/25 transition hover:bg-amber-400 disabled:opacity-60"
            type="button" disabled={isCreatingRoom}
            onclick={createLocalRoom}>
            <iconify-icon icon="mdi:plus" class="text-base"></iconify-icon>
            {isCreatingRoom ? "Creating..." : "New Room"}
          </button>
        </div>

        {#if room}
          <div class="space-y-2">
            <p class="text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500">Room Details</p>
            <div class="rounded-lg bg-slate-800/70 p-2.5">
              <p class="text-[10px] uppercase tracking-wider text-slate-500">Room ID</p>
              <p class="mt-0.5 break-all font-mono text-[11px] text-slate-300">{room.roomId}</p>
            </div>
            <div class="rounded-lg bg-slate-800/70 p-2.5">
              <div class="flex items-center justify-between">
                <p class="text-[10px] uppercase tracking-wider text-slate-500">Signaling URL</p>
                <button
                  class="rounded p-0.5 text-slate-500 transition hover:text-cyan-400"
                  type="button" onclick={() => copyText(room!.signalingUrl)}
                  title="Copy">
                  <iconify-icon icon={copied ? "mdi:check" : "mdi:content-copy"} class="text-xs"></iconify-icon>
                </button>
              </div>
              <p class="mt-0.5 break-all font-mono text-[11px] text-emerald-300">{room.signalingUrl}</p>
            </div>
            <div class="rounded-lg bg-slate-800/70 p-2.5">
              <div class="flex items-center justify-between">
                <p class="text-[10px] uppercase tracking-wider text-slate-500">Open stream in browser</p>
                <button
                  class="rounded p-0.5 text-slate-500 transition hover:text-cyan-400"
                  type="button" onclick={() => copyText(browserHostUrl)}
                  title="Copy">
                  <iconify-icon icon={copied ? "mdi:check" : "mdi:content-copy"} class="text-xs"></iconify-icon>
                </button>
              </div>
              <p class="mt-0.5 break-all font-mono text-[11px] text-cyan-300">{browserHostUrl}</p>
              <p class="mt-2 text-[10px] leading-relaxed text-slate-500">
                Open this URL in Chrome/Firefox. This Tauri app runs the signaling server only.
              </p>
            </div>
          </div>
        {/if}

        <div>
          <p class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500">Server Status</p>
          <div class="space-y-1.5 rounded-lg bg-slate-800/40 p-2.5">
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Status</span>
              <span class="flex items-center gap-1.5 text-[11px] font-mono">
                {#if signalingStatus?.isRunning}
                  <span class="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
                  <span class="text-emerald-300">Running</span>
                {:else}
                  <span class="inline-block h-1.5 w-1.5 rounded-full bg-red-400"></span>
                  <span class="text-red-300">Stopped</span>
                {/if}
              </span>
            </div>
            {#if signalingStatus?.localIp}
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Port</span>
                <span class="text-[11px] font-mono text-slate-300">{signalingStatus.port}</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Address</span>
                <span class="text-[11px] font-mono text-slate-300">{signalingStatus.localIp}</span>
              </div>
            {/if}
          </div>
        </div>

      {:else}
        <!-- ========== BROWSER CLIENT VIEW ========== -->
        <div>
          <p class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500">Connection</p>
          <input
            id="join-url"
            class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs font-mono text-slate-200 outline-none placeholder:text-slate-600 focus:border-cyan-500"
            bind:value={joinUrl}
            placeholder="ws://192.168.15.8:17777/ws/..."
            type="text" />
          <div class="mt-2 flex gap-2">
            <button
              class="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-cyan-500 px-3 py-2 text-xs font-semibold text-slate-950 shadow-lg shadow-cyan-500/25 transition hover:bg-cyan-400"
              type="button" onclick={joinRoom}>
              <iconify-icon icon="mdi:login" class="text-sm"></iconify-icon>
              Join
            </button>
            <button
              class="flex flex-1 items-center justify-center gap-1.5 rounded-lg border border-emerald-400/30 px-3 py-2 text-xs font-semibold text-emerald-300 transition hover:bg-emerald-400/10"
              type="button" onclick={hostRoomInThisBrowser}>
              <iconify-icon icon="mdi:broadcast" class="text-sm"></iconify-icon>
              Host
            </button>
          </div>
        </div>

        <div>
          <p class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500">Status</p>
          <div class="space-y-1.5 rounded-lg bg-slate-800/40 p-2.5">
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Role</span>
              <span class="text-[11px] font-mono text-slate-300 capitalize">{peerRole ?? "none"}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Socket</span>
              <span class="flex items-center gap-1.5 text-[11px] font-mono text-slate-300">
                <span class="inline-block h-1.5 w-1.5 rounded-full {signalingConnectionState === 'Connected' ? 'bg-emerald-400' : signalingConnectionState === 'Error' ? 'bg-red-400' : 'bg-amber-400'}"></span>
                {signalingConnectionState}
              </span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">WebRTC</span>
              <span class="flex items-center gap-1.5 text-[11px] font-mono text-slate-300">
                <span class="inline-block h-1.5 w-1.5 rounded-full {peerConnectionState === 'connected' ? 'bg-emerald-400' : peerConnectionState === 'failed' ? 'bg-red-400' : 'bg-amber-400'}"></span>
                {peerConnectionState}
              </span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Data ch.</span>
              <span class="flex items-center gap-1.5 text-[11px] font-mono text-slate-300">
                <span class="inline-block h-1.5 w-1.5 rounded-full {dataChannelState === 'open' ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
                {dataChannelState}
              </span>
            </div>
          </div>
        </div>

        {#if peerRole}
          <div>
            <p class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500">Devices</p>
            <div class="flex gap-2">
              <button
                class="flex flex-1 items-center justify-center gap-1.5 rounded-lg border border-slate-700 px-2 py-2 text-[11px] font-medium text-slate-300 transition hover:border-emerald-500 hover:text-emerald-300"
                type="button" onclick={startCamera} title="Camera">
                <iconify-icon icon="mdi:video" class="text-sm"></iconify-icon>
              </button>
              <button
                class="flex flex-1 items-center justify-center gap-1.5 rounded-lg border border-slate-700 px-2 py-2 text-[11px] font-medium text-slate-300 transition hover:border-red-500 hover:text-red-300"
                type="button" onclick={stopCamera} title="Stop camera">
                <iconify-icon icon="mdi:video-off" class="text-sm"></iconify-icon>
              </button>
              <button
                class="flex flex-1 items-center justify-center gap-1.5 rounded-lg border {micState === 'Muted' ? 'border-red-500/50 bg-red-500/10 text-red-300' : 'border-slate-700 text-slate-300'} px-2 py-2 text-[11px] font-medium transition hover:border-cyan-500 hover:text-cyan-300"
                type="button" onclick={toggleMic} title={micState === "Muted" ? "Unmute" : "Mute"}>
                <iconify-icon icon={micState === "Muted" ? "mdi:microphone-off" : "mdi:microphone"} class="text-sm"></iconify-icon>
              </button>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Bottom user area -->
    <div class="flex items-center gap-2.5 border-t border-slate-800 bg-slate-900/80 px-3 py-2">
      <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full {isTauri ? 'bg-amber-500/30' : peerRole === 'host' ? 'bg-emerald-500/30' : peerRole === 'guest' ? 'bg-cyan-500/30' : 'bg-slate-700'}">
        <iconify-icon
          icon={isTauri ? "mdi:server" : peerRole === "host" ? "mdi:broadcast" : peerRole === "guest" ? "mdi:account" : "mdi:help"}
          class="text-sm {isTauri ? 'text-amber-300' : peerRole ? 'text-emerald-300' : 'text-slate-500'}">
        </iconify-icon>
      </div>
      <div class="min-w-0 flex-1">
        <p class="truncate text-xs font-medium">
          {isTauri ? "Signaling server" : peerRole === "host" ? "You are the host" : peerRole === "guest" ? "You are a guest" : "Not connected"}
        </p>
        <p class="truncate text-[10px] text-slate-500">
          {isTauri
            ? signalingStatus?.isRunning
              ? "Running on :" + signalingStatus.port
              : "Stopped"
            : room?.roomId
              ? room.roomId.slice(0, 12) + "..."
              : "No room"}
        </p>
      </div>
    </div>
  </aside>

  <!-- MAIN CONTENT -->
  <div class="flex flex-1 flex-col overflow-hidden">
    <div class="flex h-12 shrink-0 items-center gap-3 border-b border-slate-800 bg-slate-900/40 px-4">
      <h2 class="text-sm font-semibold text-slate-200">
        {#if isTauri}
          Signaling Server
        {:else if peerRole === "host"}
          Streaming
        {:else if peerRole === "guest"}
          Watching
        {:else}
          Join a stream
        {/if}
      </h2>
      <div class="flex-1"></div>
      {#if !isTauri && peerRole}
        <button
          class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-400 transition hover:border-slate-500 hover:text-white"
          type="button" onclick={sendDataPing} title="Ping">
          <iconify-icon icon="mdi:send" class="text-sm"></iconify-icon>
        </button>
        <button
          class="flex items-center gap-1.5 rounded-lg bg-red-500/20 px-3 py-1 text-xs font-medium text-red-400 transition hover:bg-red-500/30"
          type="button" onclick={closeConnection}>
          <iconify-icon icon="mdi:phone-hangup" class="text-sm"></iconify-icon>
          Leave
        </button>
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      {#if isTauri}
        <!-- TAURI: Server instructions -->
        <div class="flex h-full flex-col items-center justify-center px-4 text-center">
          <div class="mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-amber-500/10">
            <iconify-icon icon="mdi:server" class="text-4xl text-amber-400"></iconify-icon>
          </div>
          <h3 class="text-xl font-semibold text-slate-200">Signaling server control</h3>
          <p class="mt-2 max-w-md text-sm text-slate-400">
            This app runs the room signaling server. Create a room, then open the stream in a browser.
          </p>
          <div class="mt-6 space-y-2 text-xs text-slate-500">
            <p><span class="mr-2 font-bold text-slate-400">1.</span>Click <span class="font-semibold text-amber-300">New Room</span> in the sidebar</p>
            <p><span class="mr-2 font-bold text-slate-400">2.</span>Copy the <span class="font-semibold text-cyan-300">browser URL</span> and open in Chrome/Firefox</p>
            <p><span class="mr-2 font-bold text-slate-400">3.</span>Share the <span class="font-semibold text-emerald-300">signaling URL</span> with guests</p>
            <p><span class="mr-2 font-bold text-slate-400">4.</span>Close this app when done. Rooms are ephemeral.</p>
          </div>
        </div>
      {:else if peerRole}
        <!-- BROWSER: Call active -->
        <div class="flex h-full flex-col gap-2">
          <div class="grid flex-1 gap-3 grid-rows-2 lg:grid-cols-2 lg:grid-rows-1">
            <div class="relative overflow-hidden rounded-xl border border-slate-800 bg-slate-900/60">
              <div class="absolute left-3 top-3 z-10 flex items-center gap-2 rounded-lg bg-slate-950/70 px-2.5 py-1 backdrop-blur">
                <span class="text-xs text-slate-300">{peerRole === "host" ? "You" : "Remote stream"}</span>
                <span class="rounded {cameraState === 'Running' ? 'bg-emerald-500/30 text-emerald-300' : 'bg-slate-700 text-slate-400'} px-1.5 py-0.5 text-[10px]">{cameraState}</span>
              </div>
              <video class="h-full w-full object-cover" autoplay bind:this={localVideo} muted playsinline></video>
            </div>
            <div class="relative overflow-hidden rounded-xl border border-slate-800 bg-slate-900/60">
              <div class="absolute left-3 top-3 z-10 rounded-lg bg-slate-950/70 px-2.5 py-1 backdrop-blur">
                <span class="text-xs text-slate-300">{peerRole === "host" ? "Guest" : "Host"}</span>
              </div>
              <video class="h-full w-full object-cover" autoplay bind:this={remoteVideo} playsinline></video>
            </div>
          </div>
          <audio bind:this={remoteAudio} autoplay></audio>
        </div>
      {:else}
        <!-- BROWSER: No call -->
        <div class="flex h-full flex-col items-center justify-center px-4 text-center">
          <div class="mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-emerald-500/10">
            <iconify-icon icon="mdi:video" class="text-4xl text-emerald-400"></iconify-icon>
          </div>
          <h3 class="text-xl font-semibold text-slate-200">Join a streaming session</h3>
          <p class="mt-2 max-w-md text-sm text-slate-400">
            Paste a signaling URL in the sidebar, then click <span class="font-semibold text-cyan-300">Join</span> to watch or <span class="font-semibold text-emerald-300">Host</span> to stream.
          </p>
          <div class="mt-6 space-y-2 text-xs text-slate-500">
            <p><span class="mr-2 font-bold text-slate-400">1.</span>Get a signaling URL from the host (starts with <code class="text-cyan-300">ws://</code>)</p>
            <p><span class="mr-2 font-bold text-slate-400">2.</span>Paste it in the <span class="font-semibold text-slate-300">Connection</span> field</p>
            <p><span class="mr-2 font-bold text-slate-400">3.</span>Click <span class="font-semibold text-emerald-300">Host</span> to stream or <span class="font-semibold text-cyan-300">Join</span> to watch</p>
          </div>
        </div>
      {/if}
    </div>
  </div>
</main>