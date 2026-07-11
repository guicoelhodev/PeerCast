<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import "iconify-icon";

  import {
    MAX_CHAT_MESSAGE_LENGTH,
    MAX_DISPLAY_NAME_LENGTH,
    parseSignalMessage,
    serializeSignalMessage,
    type SignalMessage,
  } from "$lib/signaling-protocol";

  type SignalingStatus = {
    isRunning: boolean;
    port: number;
    localIp: string | null;
    url: string | null;
    activeRoom: RoomInfo | null;
  };

  type RoomInfo = {
    roomId: string;
    appUrl: string;
    signalingUrl: string;
  };

  type ScreenQualityPreset = {
    id: "low" | "balanced" | "high" | "ultra" | "4k";
    label: string;
    width: number;
    height: number;
    fps: number;
    bitrate: number;
  };

  type ChatMessage = {
    id: string;
    peerId: string;
    text: string;
    sentAt: string;
    isOwn: boolean;
  };

  const screenQualityPresets: ScreenQualityPreset[] = [
    {
      id: "low",
      label: "Low - 720p30",
      width: 1280,
      height: 720,
      fps: 30,
      bitrate: 2_500_000,
    },
    {
      id: "balanced",
      label: "Balanced - 1080p30",
      width: 1920,
      height: 1080,
      fps: 30,
      bitrate: 5_000_000,
    },
    {
      id: "high",
      label: "High - 1080p60",
      width: 1920,
      height: 1080,
      fps: 60,
      bitrate: 8_000_000,
    },
    {
      id: "ultra",
      label: "Ultra - 1440p60",
      width: 2560,
      height: 1440,
      fps: 60,
      bitrate: 14_000_000,
    },
    {
      id: "4k",
      label: "4K Experimental - 4K30",
      width: 3840,
      height: 2160,
      fps: 30,
      bitrate: 20_000_000,
    },
  ];

  interface GuestPeer {
    id: string;
    pc: RTCPeerConnection;
    stream: MediaStream;
    audioStream: MediaStream;
    connected: boolean;
    isHost: boolean;
    videoEl: HTMLVideoElement | null;
    audioEl: HTMLAudioElement | null;
    audioVolume: number;
    systemAudioSender: RTCRtpSender | null;
    displayName: string;
    isMuted: boolean;
    isSpeaking: boolean;
    videoState: "camera" | "screen" | "off";
  }

  let guestPeers: GuestPeer[] = [];

  let signalingStatus: SignalingStatus | null = null;
  let room: RoomInfo | null = null;
  let roomParticipants: string[] = [];
  let errorMessage = "";
  let infoMessage = "";
  let isCreatingRoom = false;
  let joinUrl = "";
  let browserHostUrl = "";
  let shareMode: "local" | "tailscale" = "local";
  let tailscaleUrl = "";
  let peerRole: "host" | "guest" | null = null;
  let signalingConnectionState = "Disconnected";
  let peerConnectionState = "New";
  let dataChannelState = "Closed";
  let lastDataMessage = "None";
  let chatMessages: ChatMessage[] = [];
  let chatDraft = "";
  let isChatOpen = false;
  let isSidebarOpen = false;
  let unreadChatMessages = 0;
  let chatHistory: HTMLDivElement | null = null;
  let chatInput: HTMLTextAreaElement | null = null;
  let cameraState = "Stopped";
  let micState = "Stopped";
  let screenShareState = "Stopped";
  let selectedScreenQualityId: ScreenQualityPreset["id"] = "balanced";
  let wasCameraOnBeforeShare = false;
  let screenAudioAvailable = false;
  let screenStream: MediaStream | null = null;
  let webrtcAvailable = true;
  let isTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  let isConnecting = false;
  let myPeerId = "";
  let displayName = "";

  let ws: WebSocket | null = null;
  let dataChannel: RTCDataChannel | null = null;
  let localStream: MediaStream | null = null;
  let microphoneStream: MediaStream | null = null;
  let microphoneTrack: MediaStreamTrack | null = null;
  let localVideo: HTMLVideoElement | null = null;
  let isLocalSpeaking = false;
  let videoGridClass = "video-grid--one";

  let copiedTarget:
    "host" | "participant" | "tailscale-serve" | "tailscale-magicdns" | null =
    null;
  let infoMessageTimer: ReturnType<typeof setTimeout> | null = null;
  let participantPoll: ReturnType<typeof setInterval> | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let reconnectAttempts = 0;
  let reconnectUrl = "";
  let shouldReconnect = false;
  let connectionGeneration = 0;
  let audioContext: AudioContext | null = null;
  let speakingPoll: ReturnType<typeof setInterval> | null = null;
  const audioAnalysers = new Map<
    string,
    { analyser: AnalyserNode; samples: Uint8Array }
  >();

  $: videoGridClass = getVideoGridClass(guestPeers.length + 1);

  onMount(async () => {
    webrtcAvailable = typeof RTCPeerConnection !== "undefined";
    myPeerId =
      sessionStorage.getItem("peercast-peer-id") ?? crypto.randomUUID();
    sessionStorage.setItem("peercast-peer-id", myPeerId);
    displayName = sessionStorage.getItem("peercast-display-name") ?? "";

    const params = new URLSearchParams(window.location.search);
    const roomUrl = params.get("room");
    const role = params.get("role");

    if (roomUrl) {
      joinUrl = roomUrl;
      browserHostUrl = buildBrowserRoomUrl(roomUrl);

      if (displayName && role === "host" && webrtcAvailable) {
        isConnecting = true;
        await startHost(roomUrl);
        isConnecting = false;
      } else if (displayName && webrtcAvailable) {
        isConnecting = true;
        await joinRoom();
        isConnecting = false;
      }
    }

    if (isTauri) {
      try {
        signalingStatus = await invoke<SignalingStatus>("signaling_status");
        restoreActiveRoom(signalingStatus.activeRoom);
      } catch (error) {
        infoMessage = `Signaling server error: ${String(error)}`;
      }
    }
  });

  async function createRoom() {
    isCreatingRoom = true;
    errorMessage = "";
    infoMessage = "";

    try {
      const publicAppUrl =
        shareMode === "tailscale" ? tailscaleUrl.trim() : null;
      if (shareMode === "tailscale" && !publicAppUrl) {
        errorMessage = "Enter the HTTPS MagicDNS URL before creating a room.";
        return false;
      }

      room = await invoke<RoomInfo>("create_room", { publicAppUrl });
      joinUrl = room.signalingUrl;
      browserHostUrl = buildBrowserRoomUrl(room.signalingUrl, room.appUrl);
      signalingStatus = await invoke<SignalingStatus>("signaling_status");
      startParticipantPolling();
      infoMessage =
        "Room created. Open the host URL in Chrome or Firefox to start streaming.";
      return true;
    } catch (error) {
      errorMessage = `Failed to create room: ${String(error)}`;
      signalingStatus = await invoke<SignalingStatus>("signaling_status");
      restoreActiveRoom(signalingStatus.activeRoom);
      return false;
    } finally {
      isCreatingRoom = false;
    }
  }

  function restoreActiveRoom(activeRoom: RoomInfo | null) {
    if (!activeRoom) return;

    room = activeRoom;
    joinUrl = activeRoom.signalingUrl;
    browserHostUrl = buildBrowserRoomUrl(
      activeRoom.signalingUrl,
      activeRoom.appUrl,
    );
    shareMode = activeRoom.appUrl.startsWith("https://")
      ? "tailscale"
      : "local";
    startParticipantPolling();
  }

  async function stopCurrentRoom() {
    if (!room) return;

    try {
      await invoke<boolean>("stop_room", { roomId: room.roomId });
      room = null;
      roomParticipants = [];
      stopParticipantPolling();
      joinUrl = "";
      browserHostUrl = "";
      infoMessage = "Room stopped. All guests disconnected.";
    } catch (error) {
      errorMessage = `Failed to stop room: ${String(error)}`;
    }
  }

  function buildBrowserRoomUrl(
    signalingUrl: string,
    appUrl = window.location.origin,
  ) {
    const url = new URL(appUrl);
    url.searchParams.set("role", "host");
    url.searchParams.set("room", signalingUrl);
    return url.toString();
  }

  function buildGuestUrl(
    signalingUrl: string,
    appUrl = window.location.origin,
  ) {
    const url = new URL(appUrl);
    url.searchParams.set("room", signalingUrl);
    return url.toString();
  }

  async function refreshRoomParticipants() {
    if (!room || !isTauri) return;
    roomParticipants = await invoke<string[]>("room_participants", {
      roomId: room.roomId,
    });
  }

  function startParticipantPolling() {
    stopParticipantPolling();
    refreshRoomParticipants();
    participantPoll = setInterval(refreshRoomParticipants, 2000);
  }

  function stopParticipantPolling() {
    if (participantPoll) {
      clearInterval(participantPoll);
      participantPoll = null;
    }
  }

  async function copyText(
    text: string,
    target: "host" | "participant" | "tailscale-serve" | "tailscale-magicdns",
  ) {
    try {
      await navigator.clipboard.writeText(text);
      copiedTarget = target;
      setTimeout(() => {
        if (copiedTarget === target) copiedTarget = null;
      }, 2000);
      return true;
    } catch {
      return false;
    }
  }

  function showTemporaryInfo(message: string) {
    if (infoMessageTimer) clearTimeout(infoMessageTimer);
    infoMessage = message;
    infoMessageTimer = setTimeout(() => {
      if (infoMessage === message) infoMessage = "";
      infoMessageTimer = null;
    }, 2000);
  }

  const tailscaleServeCommand =
    "tailscale serve --bg --https=443 http://127.0.0.1:17777";
  const tailscaleMagicDnsCommand =
    "tailscale status --json | jq -r '.Self.DNSName'";

  async function copyTailscaleCommand(
    command: string,
    target: "tailscale-serve" | "tailscale-magicdns",
  ) {
    if (await copyText(command, target)) {
      showTemporaryInfo("Command copied successfully.");
    } else {
      errorMessage = "Unable to copy the command. Copy it manually.";
    }
  }

  async function startHost(signalingUrl: string) {
    closeConnection();
    shouldReconnect = true;
    reconnectUrl = signalingUrl;
    peerRole = "host";
    guestPeers = [];

    try {
      await connectSignaling(signalingUrl);
      await startMicrophone();
      // A refreshed host gets a new peer ID; existing guests use this to negotiate again.
      sendReadySignal();
      infoMessage = webrtcAvailable
        ? "Host ready. Waiting for guests to join."
        : "Room created (signaling only). Open the room URL in Chrome/Firefox to stream.";
    } catch {
      infoMessage = "Unable to connect. Retrying automatically...";
    }
  }

  async function joinRoom() {
    if (!joinUrl.trim()) {
      errorMessage = "Open a room link before joining.";
      return;
    }

    const params = new URLSearchParams(window.location.search);
    const role = params.get("role");
    const normalizedName = displayName.trim();

    if (!normalizedName) {
      errorMessage = "Enter your name before joining the room.";
      return;
    }

    displayName = normalizedName;
    sessionStorage.setItem("peercast-display-name", displayName);

    if (role === "host") {
      errorMessage = "";
      infoMessage = "";
      isConnecting = true;
      browserHostUrl = buildBrowserRoomUrl(joinUrl.trim());
      await startHost(joinUrl.trim());
      isConnecting = false;
      return;
    }

    isConnecting = true;
    closeConnection();
    shouldReconnect = true;
    reconnectUrl = joinUrl.trim();
    errorMessage = "";
    infoMessage = "";
    peerRole = "guest";

    try {
      await connectSignaling(joinUrl.trim());

      if (webrtcAvailable) {
        await startMicrophone();
        sendReadySignal();
        infoMessage = "Joined the room. Connecting to participants.";
      } else {
        infoMessage =
          "WebSocket connected but WebRTC not available. Open this URL in a browser.";
      }
    } catch {
      infoMessage = "Unable to connect. Retrying automatically...";
    }
    isConnecting = false;
  }

  function createPeerConnectionForGuest(peerId: string): RTCPeerConnection {
    const pc = new RTCPeerConnection({ iceServers: [] });

    pc.onicecandidate = (event) => {
      if (event.candidate) {
        sendSignal({
          type: "ice",
          peerId: myPeerId,
          targetPeerId: peerId,
          candidate: event.candidate.toJSON(),
        });
      }
    };

    pc.onconnectionstatechange = () => {
      const peer = guestPeers.find((guest) => guest.id === peerId);
      if (!peer) return;

      peer.connected = pc.connectionState === "connected";
      if (pc.connectionState === "failed" || pc.connectionState === "closed") {
        guestPeers = guestPeers.filter((guest) => guest.id !== peerId);
      }
      guestPeers = guestPeers;
      updatePeerCount();
    };

    return pc;
  }

  function bindGuestOnTrack(guest: GuestPeer) {
    guest.pc.ontrack = (event) => {
      if (event.track.kind === "audio") {
        // A remote peer can send microphone and display audio in either order.
        // Keep them in one MediaStream so playback never depends on a separate
        // signaling message being received before the WebRTC track event.
        guest.audioStream.addTrack(event.track);
        event.track.onended = () => {
          guest.audioStream.removeTrack(event.track);
          guestPeers = guestPeers;
        };
      } else {
        guest.stream.addTrack(event.track);
        if (guest.videoState === "off") guest.videoState = "camera";
        event.track.onended = () => {
          guest.stream.removeTrack(event.track);
          guestPeers = guestPeers;
          attachVideoStreams();
        };
      }
      guestPeers = guestPeers;
      attachVideoStreams();
      startSpeakingDetection();
    };
  }

  function getStreamAudioLevel(stream: MediaStream, key: string) {
    if (stream.getAudioTracks().length === 0) return 0;

    if (!audioContext) audioContext = new AudioContext();
    if (audioContext.state === "suspended")
      audioContext.resume().catch(() => {});

    let audio = audioAnalysers.get(key);
    if (!audio) {
      const analyser = audioContext.createAnalyser();
      analyser.fftSize = 512;
      audioContext.createMediaStreamSource(stream).connect(analyser);
      audio = { analyser, samples: new Uint8Array(analyser.fftSize) };
      audioAnalysers.set(key, audio);
    }

    audio.analyser.getByteTimeDomainData(audio.samples);
    const sum = audio.samples.reduce(
      (total, sample) => total + Math.abs(sample - 128),
      0,
    );
    return sum / audio.samples.length;
  }

  function updateSpeakingIndicators() {
    isLocalSpeaking = Boolean(
      microphoneStream &&
      microphoneTrack?.enabled &&
      getStreamAudioLevel(microphoneStream, "local") > 3,
    );

    for (const guest of guestPeers) {
      guest.isSpeaking =
        !guest.isMuted && getStreamAudioLevel(guest.audioStream, guest.id) > 3;
    }
    guestPeers = guestPeers;
  }

  function startSpeakingDetection() {
    if (speakingPoll) return;
    speakingPoll = setInterval(updateSpeakingIndicators, 150);
  }

  function stopSpeakingDetection() {
    if (speakingPoll) clearInterval(speakingPoll);
    speakingPoll = null;
    audioAnalysers.clear();
    audioContext?.close().catch(() => {});
    audioContext = null;
    isLocalSpeaking = false;
  }

  function addChatMessage(message: ChatMessage) {
    if (chatMessages.some((existing) => existing.id === message.id)) return;
    chatMessages = [...chatMessages, message].slice(-200);

    if (!message.isOwn && !isChatOpen) {
      unreadChatMessages += 1;
    }

    requestAnimationFrame(() => {
      if (chatHistory) chatHistory.scrollTop = chatHistory.scrollHeight;
    });
  }

  function participantName(peerId: string, isOwn: boolean) {
    if (isOwn) return "You";
    const peer = guestPeers.find((guest) => guest.id === peerId);
    return peer?.displayName ?? (peer?.isHost ? "Host" : "Participant");
  }

  function formatChatTime(sentAt: string) {
    return new Intl.DateTimeFormat(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(sentAt));
  }

  function toggleChat() {
    isChatOpen = !isChatOpen;
    if (isChatOpen) {
      unreadChatMessages = 0;
      requestAnimationFrame(() => chatInput?.focus());
    }
  }

  function toggleSidebar() {
    isSidebarOpen = !isSidebarOpen;
  }

  function closeSidebar() {
    isSidebarOpen = false;
  }

  function sendChatMessage() {
    const text = chatDraft.trim();
    if (!text || text.length > MAX_CHAT_MESSAGE_LENGTH) return;
    if (ws?.readyState !== WebSocket.OPEN || !peerRole) {
      errorMessage = "Chat is unavailable while disconnected from the room.";
      return;
    }

    const message: SignalMessage = {
      type: "chat",
      peerId: myPeerId,
      messageId: crypto.randomUUID(),
      text,
      sentAt: new Date().toISOString(),
    };
    sendSignal(message);
    addChatMessage({ ...message, id: message.messageId, isOwn: true });
    chatDraft = "";
    requestAnimationFrame(() => chatInput?.focus());
  }

  function handleChatKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      sendChatMessage();
    }
  }

  async function connectSignaling(signalingUrl: string) {
    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(signalingUrl);
      ws = socket;
      signalingConnectionState = "Connecting";

      socket.onopen = () => {
        if (ws !== socket || !shouldReconnect) {
          socket.close();
          reject(new Error("WebSocket connection cancelled"));
          return;
        }
        signalingConnectionState = "Connected";
        resolve();
      };
      socket.onerror = () => {
        signalingConnectionState = "Error";
        reject(new Error("WebSocket connection failed"));
      };
      socket.onclose = () => {
        if (ws !== socket) return;
        signalingConnectionState = "Disconnected";
        handleUnexpectedDisconnect();
      };
      socket.onmessage = async (event) => {
        await handleSignalMessage(String(event.data));
      };
    });
  }

  function handleUnexpectedDisconnect() {
    if (!shouldReconnect || !peerRole || !reconnectUrl) return;
    for (const guest of guestPeers) guest.pc.close();
    guestPeers = [];
    peerConnectionState = "Reconnecting";
    scheduleReconnect();
  }

  function scheduleReconnect() {
    if (reconnectTimer || !shouldReconnect) return;
    const generation = connectionGeneration;
    const delay = Math.min(1_000 * 2 ** reconnectAttempts, 10_000);
    reconnectAttempts += 1;
    signalingConnectionState = "Reconnecting";
    infoMessage = `Connection lost. Retrying in ${Math.ceil(delay / 1000)}s...`;
    reconnectTimer = setTimeout(async () => {
      reconnectTimer = null;
      try {
        await connectSignaling(reconnectUrl);
        if (!shouldReconnect || generation !== connectionGeneration) return;
        reconnectAttempts = 0;
        sendReadySignal();
        infoMessage = "Reconnected to the room.";
      } catch {
        if (shouldReconnect && generation === connectionGeneration) {
          scheduleReconnect();
        }
      }
    }, delay);
  }

  async function handleSignalMessage(data: string) {
    const message = parseSignalMessage(data);
    if (!message) return;

    if (message.type === "ready") {
      if (!webrtcAvailable) {
        return;
      }

      const guestId = message.peerId;
      if (guestId === myPeerId) {
        return;
      }

      const existingPeer = guestPeers.find((guest) => guest.id === guestId);
      if (existingPeer) {
        existingPeer.displayName =
          message.displayName ?? existingPeer.displayName;
        existingPeer.isMuted = message.microphoneMuted ?? existingPeer.isMuted;
        guestPeers = guestPeers;
        if (!existingPeer.isHost) return;
        // A host refresh keeps its ID and replaces the previous peer connection.
        existingPeer.pc.close();
        guestPeers = guestPeers.filter((guest) => guest.id !== guestId);
      }

      const pc = createPeerConnectionForGuest(guestId);
      const guest: GuestPeer = {
        id: guestId,
        pc,
        stream: new MediaStream(),
        audioStream: new MediaStream(),
        connected: false,
        isHost: false,
        audioVolume: 1,
        systemAudioSender: null,
        displayName: message.displayName ?? "Participant",
        isMuted: message.microphoneMuted ?? false,
        isSpeaking: false,
        videoState: "off",
        videoEl: null,
        audioEl: null,
      };
      bindGuestOnTrack(guest);
      guestPeers = [...guestPeers, guest];
      addLocalTracksToPeerConnection(pc);
      addCurrentSystemAudioToPeer(guest, false);
      updatePeerCount();

      infoMessage = `${guest.displayName} joined. Connecting...`;

      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      sendSignal({
        type: "offer",
        peerId: myPeerId,
        targetPeerId: guestId,
        isHost: peerRole === "host",
        displayName,
        microphoneMuted: micState !== "Active",
        description: offer,
      });
      infoMessage = `Offer sent to ${guest.displayName}.`;

      return;
    }

    if (message.type === "participant-left") {
      const guest = guestPeers.find((peer) => peer.id === message.peerId);
      if (guest) {
        guest.pc.close();
        guestPeers = guestPeers.filter((peer) => peer.id !== message.peerId);
        updatePeerCount();
      }
      return;
    }

    if (message.type === "chat") {
      if (message.peerId === myPeerId) return;
      addChatMessage({ ...message, id: message.messageId, isOwn: false });
      return;
    }

    if (message.type === "offer") {
      if (message.targetPeerId !== myPeerId) return;

      if (!webrtcAvailable) {
        return;
      }

      let guest = guestPeers.find((peer) => peer.id === message.peerId);
      if (!guest) {
        const pc = createPeerConnectionForGuest(message.peerId);
        guest = {
          id: message.peerId,
          pc,
          stream: new MediaStream(),
          audioStream: new MediaStream(),
          connected: false,
          isHost: message.isHost === true,
          audioVolume: 1,
          systemAudioSender: null,
          displayName: message.displayName ?? "Participant",
          isMuted: message.microphoneMuted ?? false,
          isSpeaking: false,
          videoState: "off",
          videoEl: null,
          audioEl: null,
        };
        bindGuestOnTrack(guest);
        guestPeers = [...guestPeers, guest];
      } else if (message.isHost) {
        guest.isHost = true;
        guest.displayName = message.displayName ?? guest.displayName;
        guest.isMuted = message.microphoneMuted ?? guest.isMuted;
        guestPeers = guestPeers;
      }

      addLocalTracksToPeerConnection(guest.pc);
      addCurrentSystemAudioToPeer(guest, false);
      await guest.pc.setRemoteDescription(message.description);
      const answer = await guest.pc.createAnswer();
      await guest.pc.setLocalDescription(answer);
      sendSignal({
        type: "answer",
        peerId: myPeerId,
        targetPeerId: message.peerId,
        displayName,
        microphoneMuted: micState !== "Active",
        description: answer,
      });
      infoMessage = "Answer sent. Waiting for peer connection.";
      return;
    }

    if (message.type === "answer") {
      if (message.targetPeerId !== myPeerId) return;
      const guest = guestPeers.find((g) => g.id === message.peerId);
      if (!guest) return;

      guest.isMuted = message.microphoneMuted ?? guest.isMuted;
      guestPeers = guestPeers;
      await guest.pc.setRemoteDescription(message.description);
      infoMessage = `${guest.displayName} connected.`;
      return;
    }

    if (message.type === "ice") {
      if (message.targetPeerId !== myPeerId) return;
      const guest = guestPeers.find((g) => g.id === message.peerId);
      if (guest) {
        await guest.pc.addIceCandidate(message.candidate);
      }
    }

    if (message.type === "microphone-state") {
      if (message.peerId === myPeerId) return;
      const guest = guestPeers.find((peer) => peer.id === message.peerId);
      if (guest) {
        guest.isMuted = message.microphoneMuted;
        guest.isSpeaking = false;
        guestPeers = guestPeers;
      }
    }

    if (message.type === "video-state") {
      if (message.peerId === myPeerId) return;
      const guest = guestPeers.find((peer) => peer.id === message.peerId);
      if (!guest) return;

      guest.videoState = message.videoState;
      if (message.videoState === "off") {
        for (const track of guest.stream.getVideoTracks()) {
          guest.stream.removeTrack(track);
        }
        if (guest.videoEl) guest.videoEl.srcObject = null;
      }
      guestPeers = guestPeers;
      attachVideoStreams();
    }
  }

  function sendSignal(message: SignalMessage) {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(serializeSignalMessage(message));
    }
  }

  function sendReadySignal() {
    sendSignal({
      type: "ready",
      peerId: myPeerId,
      displayName,
      microphoneMuted: micState !== "Active",
    });
  }

  function updatePeerCount() {
    const connected = guestPeers.filter((g) => g.connected).length;
    peerConnectionState = connected > 0 ? `${connected} connected` : "New";
  }

  function sendDataPing() {
    errorMessage = "Data channel not available in multi-guest mode.";
  }

  async function startCamera() {
    errorMessage = "";

    if (screenShareState === "Running") {
      await stopScreenShare();
    }

    try {
      localStream = await navigator.mediaDevices.getUserMedia({ video: true });
      cameraState = "Running";
      attachVideoStreams();
      addLocalTracksToAllPeers();

      if (!microphoneTrack) await startMicrophone();

      for (const guest of guestPeers) {
        // Camera tracks need an offer even while the connection state is still settling.
        await renegotiate(guest);
      }
      sendVideoState("camera");
    } catch (error) {
      cameraState = "Error";
      const hint =
        window.location.hostname !== "localhost"
          ? " Access via http://localhost:5173 or use pnpm tauri:dev."
          : " Grant camera permission in your browser and try again.";
      errorMessage = `Failed to start camera: ${String(error)}.${hint}`;
    }
  }

  async function startMicrophone() {
    try {
      microphoneStream = await navigator.mediaDevices.getUserMedia({
        audio: true,
      });
      microphoneTrack = microphoneStream.getAudioTracks()[0] ?? null;
      if (!microphoneTrack) throw new Error("No microphone track available");

      micState = "Active";
      startSpeakingDetection();
      addLocalTracksToAllPeers();
      await Promise.all(guestPeers.map(renegotiate));
      sendSignal({
        type: "microphone-state",
        peerId: myPeerId,
        microphoneMuted: false,
      });
    } catch (error) {
      micState = "Error";
      errorMessage = `Failed to start microphone: ${String(error)}`;
    }
  }

  async function toggleMic() {
    if (!microphoneTrack) {
      await startMicrophone();
      return;
    }

    const currentlyMuted = !microphoneTrack.enabled;
    microphoneTrack.enabled = currentlyMuted;
    micState = currentlyMuted ? "Active" : "Muted";
    if (!currentlyMuted) isLocalSpeaking = false;
    sendSignal({
      type: "microphone-state",
      peerId: myPeerId,
      microphoneMuted: !currentlyMuted,
    });
  }

  async function stopCamera() {
    localStream?.getVideoTracks().forEach((track) => track.stop());
    localStream = null;
    cameraState = "Stopped";
    sendVideoState("off");
    await removeVideoTracksFromAllPeers();
    attachVideoStreams();
  }

  async function toggleCamera() {
    if (cameraState === "Running") {
      await stopCamera();
    } else {
      await startCamera();
    }
  }

  async function startScreenShare() {
    errorMessage = "";
    const quality = getSelectedScreenQuality();

    try {
      const displayOptions: Omit<DisplayMediaStreamOptions, "audio"> & {
        audio?: boolean | (MediaTrackConstraints & {
          suppressLocalAudioPlayback?: boolean;
        });
        surfaceSwitching?: "include" | "exclude";
        systemAudio?: "include" | "exclude";
        windowAudio?: "exclude" | "window" | "system";
      } = {
        video: {
          width: { ideal: quality.width },
          height: { ideal: quality.height },
          frameRate: { ideal: quality.fps },
        },
        audio: { suppressLocalAudioPlayback: false },
        // Ask Chromium to offer audio-capable sources in the full picker.
        // Unsupported WebViews ignore these hints.
        surfaceSwitching: "include",
        systemAudio: "include",
        windowAudio: "system",
      };

      screenStream =
        await navigator.mediaDevices.getDisplayMedia(displayOptions);

      screenShareState = "Running";

      const screenTrack = screenStream.getVideoTracks()[0];
      screenTrack.onended = () => stopScreenShare();
      const screenAudioTrack = screenStream.getAudioTracks()[0];
      screenAudioAvailable = Boolean(screenAudioTrack);
      if (screenAudioTrack) {
        screenAudioTrack.onended = () => {
          screenAudioAvailable = false;
          removeSystemAudioTracksFromAllPeers();
        };
      }

      wasCameraOnBeforeShare = cameraState === "Running";

      if (localStream) {
        localStream.getVideoTracks().forEach((track) => track.stop());
      }

      localStream = screenStream;
      attachVideoStreams();
      sendVideoTrackToAllPeers(screenTrack);
      sendVideoState("screen");
      if (screenAudioTrack) {
        sendSystemAudioTrackToAllPeers(screenAudioTrack);
      }
      await applyScreenQuality(screenTrack, quality);
      infoMessage = screenAudioTrack
        ? `Screen and system audio sharing at ${quality.width}x${quality.height}, ${quality.fps} FPS, ${formatBitrate(quality.bitrate)}.`
        : "Screen sharing started without audio. On Linux, choose a browser tab in the picker and enable Share tab audio; entire-screen and window capture usually provide video only.";
    } catch (error) {
      screenShareState = "Stopped";
      if (String(error) !== "AbortError") {
        errorMessage = `Screen share failed: ${String(error)}`;
      }
    }
  }

  async function stopScreenShare() {
    if (!screenStream) return;

    screenStream.getTracks().forEach((track) => track.stop());
    screenStream = null;
    localStream = null;
    screenShareState = "Stopped";
    screenAudioAvailable = false;
    sendVideoState("off");
    attachVideoStreams();
    await removeVideoTracksFromAllPeers();

    if (wasCameraOnBeforeShare) {
      await startCamera();
    } else {
      removeSystemAudioTracksFromAllPeers();
    }
  }

  async function removeVideoTracksFromAllPeers() {
    await Promise.all(
      guestPeers.map(async (guest) => {
        for (const sender of guest.pc.getSenders()) {
          if (sender.track?.kind === "video") {
            guest.pc.removeTrack(sender);
          }
        }
        await renegotiate(guest);
      }),
    );
  }

  function sendVideoState(videoState: "camera" | "screen" | "off") {
    sendSignal({ type: "video-state", peerId: myPeerId, videoState });
  }

  function sendVideoTrackToAllPeers(track: MediaStreamTrack) {
    for (const guest of guestPeers) {
      const sender = guest.pc
        .getSenders()
        .find((s) => s.track?.kind === "video");
      if (sender) {
        sender.replaceTrack(track).catch(() => {});
      } else if (localStream) {
        guest.pc.addTrack(track, localStream);
        renegotiate(guest);
      }
    }
  }

  function sendSystemAudioTrackToAllPeers(track: MediaStreamTrack) {
    for (const guest of guestPeers) {
      addSystemAudioTrackToPeer(guest, track);
    }
  }

  function addCurrentSystemAudioToPeer(
    guest: GuestPeer,
    renegotiateAfterAdd: boolean,
  ) {
    const track = screenStream?.getAudioTracks()[0];
    if (track) addSystemAudioTrackToPeer(guest, track, renegotiateAfterAdd);
  }

  function addSystemAudioTrackToPeer(
    guest: GuestPeer,
    track: MediaStreamTrack,
    renegotiateAfterAdd = true,
  ) {
    if (guest.systemAudioSender) return;
    guest.systemAudioSender = guest.pc.addTrack(
      track,
      new MediaStream([track]),
    );
    if (renegotiateAfterAdd) renegotiate(guest);
  }

  function removeSystemAudioTracksFromAllPeers() {
    for (const guest of guestPeers) {
      if (guest.systemAudioSender) {
        guest.pc.removeTrack(guest.systemAudioSender);
        guest.systemAudioSender = null;
        renegotiate(guest);
      }
    }
  }

  function getSelectedScreenQuality(): ScreenQualityPreset {
    return (
      screenQualityPresets.find(
        (item) => item.id === selectedScreenQualityId,
      ) ?? screenQualityPresets[1]
    );
  }

  async function updateScreenQuality() {
    if (screenShareState !== "Running" || !screenStream) return;

    const quality = getSelectedScreenQuality();
    const track = screenStream.getVideoTracks()[0];
    if (!track) return;

    await applyScreenQuality(track, quality);
    infoMessage = `Quality updated: ${quality.width}x${quality.height}, ${quality.fps} FPS, ${formatBitrate(quality.bitrate)}.`;
  }

  async function applyScreenQuality(
    track: MediaStreamTrack,
    quality: ScreenQualityPreset,
  ) {
    try {
      await track.applyConstraints({
        width: { ideal: quality.width },
        height: { ideal: quality.height },
        frameRate: { ideal: quality.fps },
      });
    } catch {
      // Display capture constraints are browser hints and may be rejected after selection.
    }

    await Promise.all(
      guestPeers.map(async (guest) => {
        const sender = guest.pc
          .getSenders()
          .find((item) => item.track?.kind === "video");
        if (!sender) return;

        try {
          const parameters = sender.getParameters();
          parameters.encodings =
            parameters.encodings.length > 0 ? parameters.encodings : [{}];
          parameters.encodings[0].maxBitrate = quality.bitrate;
          parameters.encodings[0].maxFramerate = quality.fps;
          await sender.setParameters(parameters);
        } catch {
          // Older WebViews may not implement sender quality parameters.
        }
      }),
    );
  }

  function formatBitrate(bitrate: number) {
    return `${(bitrate / 1_000_000).toLocaleString(undefined, { maximumFractionDigits: 1 })} Mbps`;
  }

  async function renegotiate(guest: GuestPeer) {
    try {
      const offer = await guest.pc.createOffer();
      await guest.pc.setLocalDescription(offer);
      sendSignal({
        type: "offer",
        peerId: myPeerId,
        targetPeerId: guest.id,
        isHost: peerRole === "host",
        displayName,
        description: offer,
      });
    } catch {
      // ignore renegotiation errors
    }
  }

  function addLocalTracksToAllPeers() {
    for (const guest of guestPeers) {
      addLocalTracksToPeerConnection(guest.pc);
    }
  }

  function addLocalTracksToPeerConnection(pc: RTCPeerConnection) {
    const tracks = [...(localStream?.getVideoTracks() ?? [])];
    if (microphoneTrack) tracks.push(microphoneTrack);
    for (const track of tracks) {
      const sender = pc
        .getSenders()
        .find((item) => item.track?.kind === track.kind);
      if (sender) {
        sender.replaceTrack(track).catch(() => {});
      } else {
        pc.addTrack(
          track,
          localStream ?? microphoneStream ?? new MediaStream([track]),
        );
      }
    }
  }

  function attachVideoStreams() {
    if (localVideo) localVideo.srcObject = localStream;
    for (const guest of guestPeers) {
      if (guest.videoEl) guest.videoEl.srcObject = guest.stream;
      if (guest.audioEl) {
        guest.audioEl.srcObject = guest.audioStream;
        guest.audioEl.play().catch(() => {});
      }
    }
  }

  function hasActiveVideo(stream: MediaStream | null) {
    return Boolean(
      stream
        ?.getVideoTracks()
        .some((track) => track.readyState === "live" && track.enabled),
    );
  }

  function initials(name: string) {
    return (
      Array.from(name.trim()).slice(0, 2).join("").toLocaleUpperCase() || "?"
    );
  }

  function toggleFullscreen(el: HTMLVideoElement | null) {
    if (!el) return;
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      el.requestFullscreen();
    }
  }

  function setGuestVolume(guest: GuestPeer, volume: number) {
    guest.audioVolume = volume;
    if (guest.audioEl) guest.audioEl.volume = volume;
  }

  function getVideoGridClass(participantCount: number) {
    if (participantCount === 1) {
      return "video-grid--one";
    }

    if (participantCount === 2) {
      return "video-grid--two";
    }

    if (participantCount <= 4) {
      return "video-grid--four";
    }

    return "video-grid--nine";
  }

  function closeConnection() {
    connectionGeneration += 1;
    shouldReconnect = false;
    reconnectAttempts = 0;
    reconnectUrl = "";
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    ws?.close();
    dataChannel?.close();
    screenStream?.getTracks().forEach((track) => track.stop());
    localStream?.getTracks().forEach((track) => track.stop());
    microphoneStream?.getTracks().forEach((track) => track.stop());
    for (const guest of guestPeers) {
      guest.pc.close();
    }
    stopSpeakingDetection();
    ws = null;
    dataChannel = null;
    screenStream = null;
    localStream = null;
    microphoneStream = null;
    microphoneTrack = null;
    guestPeers = [];
    peerRole = null;
    isConnecting = false;
    screenShareState = "Stopped";
    screenAudioAvailable = false;
    cameraState = "Stopped";
    micState = "Stopped";
    signalingConnectionState = "Disconnected";
    peerConnectionState = "Closed";
    dataChannelState = "Closed";
    chatMessages = [];
    chatDraft = "";
    unreadChatMessages = 0;
    isChatOpen = false;
    isSidebarOpen = false;
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") {
      isChatOpen = false;
      isSidebarOpen = false;
    }
  }}
/>

<main
  class="app-shell flex min-h-dvh overflow-hidden bg-slate-950 text-slate-100"
>
  {#if isSidebarOpen}
    <button
      class="sidebar-backdrop"
      type="button"
      aria-label="Close navigation"
      onclick={closeSidebar}
    ></button>
  {/if}

  <!-- LEFT SIDEBAR -->
  <aside
    id="app-sidebar"
    class:sidebar-open={isSidebarOpen}
    class="app-sidebar flex {isTauri
      ? 'w-96'
      : 'w-72'} shrink-0 flex-col border-r border-slate-800 bg-slate-900/60"
    aria-label="Application navigation"
  >
    <!-- App header -->
    <div
      class="flex h-12 items-center justify-between border-b border-slate-800 px-4"
    >
      <span class="text-sm font-semibold tracking-wide">PeerCast</span>
      <span
        class="rounded {isTauri
          ? 'bg-amber-400/20 text-amber-300'
          : 'bg-emerald-400/20 text-emerald-300'} px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider"
      >
        {isTauri ? "Server" : "Client"}
      </span>
    </div>

    <div class="flex-1 space-y-4 overflow-y-auto p-3">
      {#snippet notifications()}
        {#if errorMessage}
          <div
            class="flex items-start justify-between gap-2 rounded-lg border border-red-400/30 bg-red-400/10 px-3 py-2"
          >
            <p class="text-xs text-red-200">{errorMessage}</p>
            <button
              class="-mr-1 -mt-1 rounded p-1 text-red-200/70 transition hover:bg-red-400/10 hover:text-red-100"
              type="button"
              onclick={() => (errorMessage = "")}
              aria-label="Close error message"
              title="Close"
            >
              <iconify-icon icon="mdi:close" class="text-sm"></iconify-icon>
            </button>
          </div>
        {/if}
        {#if infoMessage}
          <div
            class="flex items-start justify-between gap-2 rounded-lg border border-cyan-400/30 bg-cyan-400/10 px-3 py-2"
          >
            <p class="text-xs text-cyan-200">{infoMessage}</p>
            <button
              class="-mr-1 -mt-1 rounded p-1 text-cyan-200/70 transition hover:bg-cyan-400/10 hover:text-cyan-100"
              type="button"
              onclick={() => (infoMessage = "")}
              aria-label="Close information message"
              title="Close"
            >
              <iconify-icon icon="mdi:close" class="text-sm"></iconify-icon>
            </button>
          </div>
        {/if}
      {/snippet}

      {#if isTauri}
        <!-- ========== TAURI SERVER VIEW ========== -->
        {#if !room}
          <div>
            <p
              class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
            >
              Share mode
            </p>
            <div class="mb-3 grid grid-cols-2 gap-2">
              <button
                class="rounded-lg border px-3 py-2 text-left text-xs transition {shareMode ===
                'local'
                  ? 'border-amber-400 bg-amber-500/10 text-amber-200'
                  : 'border-slate-700 bg-slate-800 text-slate-400 hover:border-slate-600'}"
                type="button"
                aria-pressed={shareMode === "local"}
                onclick={() => (shareMode = "local")}
              >
                <span class="block font-semibold">Local network</span>
                <span class="mt-0.5 block text-[10px] opacity-75"
                  >Same Wi-Fi or LAN</span
                >
              </button>
              <button
                class="rounded-lg border px-3 py-2 text-left text-xs transition {shareMode ===
                'tailscale'
                  ? 'border-amber-400 bg-amber-500/10 text-amber-200'
                  : 'border-slate-700 bg-slate-800 text-slate-400 hover:border-slate-600'}"
                type="button"
                aria-pressed={shareMode === "tailscale"}
                onclick={() => (shareMode = "tailscale")}
              >
                <span class="block font-semibold">Tailscale</span>
                <span class="mt-0.5 block text-[10px] opacity-75"
                  >Private remote sharing</span
                >
              </button>
            </div>

            {#if shareMode === "local"}
              <div
                class="mb-3 rounded-lg border border-slate-800 bg-slate-800/40 p-2.5 text-[10px] leading-relaxed text-slate-400"
              >
                <p class="font-semibold text-slate-300">
                  Before creating a local room
                </p>
                <ol class="mt-1.5 list-decimal space-y-1 pl-4">
                  <li>Connect every participant to the same network.</li>
                  <li>
                    Allow inbound connections on port 17777 in the host
                    firewall.
                  </li>
                  <li>Share the generated participant link.</li>
                </ol>
                <p class="mt-2 text-amber-200/80">
                  Browsers can block camera or screen capture on an HTTP IP
                  address. Use Tailscale for a secure HTTPS origin.
                </p>
              </div>
            {:else}
              <label
                class="mb-2 block text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
                for="tailscale-url">MagicDNS HTTPS URL</label
              >
              <input
                id="tailscale-url"
                class="mb-2 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs font-mono text-slate-200 outline-none placeholder:text-slate-600 focus:border-amber-500"
                bind:value={tailscaleUrl}
                placeholder="https://host.tailnet.ts.net"
                type="url"
              />
              <div
                class="mb-3 rounded-lg border border-slate-800 bg-slate-800/40 p-2.5 text-[10px] leading-relaxed text-slate-400"
              >
                <p class="font-semibold text-slate-300">
                  Before creating a Tailscale room
                </p>
                <ol class="mt-1.5 list-decimal space-y-1 pl-4">
                  <li>Install and sign in to Tailscale on this computer.</li>
                  <li>
                    <span class="block">Expose PeerCast with:</span>
                    <div
                      class="mt-1 flex items-center gap-1 rounded bg-slate-950/60 px-1.5 py-1"
                    >
                      <code class="min-w-0 flex-1 break-all text-cyan-300"
                        >{tailscaleServeCommand}</code
                      >
                      <button
                        class="shrink-0 rounded p-1 text-cyan-300 transition hover:bg-cyan-400/10 hover:text-cyan-200"
                        type="button"
                        onclick={() =>
                          copyTailscaleCommand(
                            tailscaleServeCommand,
                            "tailscale-serve",
                          )}
                        aria-label="Copy Tailscale serve command"
                        title="Copy command"
                      >
                        <iconify-icon
                          icon={copiedTarget === "tailscale-serve"
                            ? "mdi:check"
                            : "mdi:content-copy"}
                        ></iconify-icon>
                      </button>
                    </div>
                  </li>
                  <li>
                    <span class="block"
                      >Get this computer's MagicDNS hostname:</span
                    >
                    <div
                      class="mt-1 flex items-center gap-1 rounded bg-slate-950/60 px-1.5 py-1"
                    >
                      <code class="min-w-0 flex-1 break-all text-cyan-300"
                        >{tailscaleMagicDnsCommand}</code
                      >
                      <button
                        class="shrink-0 rounded p-1 text-cyan-300 transition hover:bg-cyan-400/10 hover:text-cyan-200"
                        type="button"
                        onclick={() =>
                          copyTailscaleCommand(
                            tailscaleMagicDnsCommand,
                            "tailscale-magicdns",
                          )}
                        aria-label="Copy MagicDNS command"
                        title="Copy command"
                      >
                        <iconify-icon
                          icon={copiedTarget === "tailscale-magicdns"
                            ? "mdi:check"
                            : "mdi:content-copy"}
                        ></iconify-icon>
                      </button>
                    </div>
                  </li>
                  <li>
                    Enter the result as <code class="text-cyan-300"
                      >https://HOSTNAME</code
                    >
                    above.
                  </li>
                </ol>
              </div>
            {/if}

            <p class="mb-3 text-[10px] leading-relaxed text-slate-500">
              Tailscale stays external to PeerCast. No Docker, auth key, or app
              credentials are required.
            </p>
            <button
              class="flex w-full items-center justify-center gap-2 rounded-lg bg-amber-500 px-4 py-2.5 text-sm font-semibold text-slate-950 shadow-lg shadow-amber-500/25 transition hover:bg-amber-400 disabled:opacity-60"
              type="button"
              disabled={isCreatingRoom}
              onclick={async () => {
                if (await createRoom()) {
                  closeSidebar();
                }
              }}
            >
              <iconify-icon icon="mdi:plus" class="text-base"></iconify-icon>
              {isCreatingRoom ? "Creating..." : "New Room"}
            </button>
          </div>
        {/if}

        {#if room}
          <div>
            <p
              class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
            >
              Active Room
            </p>
            <div class="mb-3 space-y-1.5 rounded-lg bg-slate-800/40 p-2.5">
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Status</span>
                <span
                  class="flex items-center gap-1.5 text-[11px] font-mono text-emerald-300"
                >
                  <span
                    class="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400"
                  ></span>
                  Active
                </span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Mode</span>
                <span class="text-[11px] font-mono text-slate-300">
                  {shareMode === "tailscale" ? "Tailscale" : "Local network"}
                </span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Participants</span>
                <span class="text-[11px] font-mono text-slate-300"
                  >{roomParticipants.length}</span
                >
              </div>
              <div class="flex items-center justify-between gap-2">
                <span class="text-[11px] text-slate-500">Room ID</span>
                <span class="truncate text-[11px] font-mono text-slate-300"
                  >{room.roomId}</span
                >
              </div>
            </div>
            <button
              class="flex w-full items-center justify-center gap-2 rounded-lg bg-red-500 px-3 py-2 text-xs font-semibold text-white shadow-lg shadow-red-500/25 transition hover:bg-red-400"
              type="button"
              onclick={stopCurrentRoom}
            >
              <iconify-icon icon="mdi:stop" class="text-sm"></iconify-icon>
              Stop Room
            </button>
          </div>
        {/if}

        <div>
          <p
            class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
          >
            Server Status
          </p>
          <div class="space-y-1.5 rounded-lg bg-slate-800/40 p-2.5">
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Status</span>
              <span class="flex items-center gap-1.5 text-[11px] font-mono">
                {#if signalingStatus?.isRunning}
                  <span
                    class="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400"
                  ></span>
                  <span class="text-emerald-300">Running</span>
                {:else}
                  <span class="inline-block h-1.5 w-1.5 rounded-full bg-red-400"
                  ></span>
                  <span class="text-red-300">Stopped</span>
                {/if}
              </span>
            </div>
            {#if signalingStatus?.localIp}
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Port</span>
                <span class="text-[11px] font-mono text-slate-300"
                  >{signalingStatus.port}</span
                >
              </div>
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-slate-500">Address</span>
                <span class="text-[11px] font-mono text-slate-300"
                  >{signalingStatus.localIp}</span
                >
              </div>
            {/if}
          </div>
        </div>
        {@render notifications()}
      {:else}
        <!-- ========== BROWSER CLIENT VIEW ========== -->
        <div>
          <p
            class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
          >
            Connection
          </p>
          <label
            class="mb-2 block text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
            for="display-name">Your name</label
          >
          <input
            id="display-name"
            class="mb-2 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 outline-none placeholder:text-slate-600 focus:border-cyan-500"
            bind:value={displayName}
            maxlength={MAX_DISPLAY_NAME_LENGTH}
            placeholder="How should people see you?"
            type="text"
          />
          <button
            class="flex w-full items-center justify-center gap-1.5 rounded-lg bg-cyan-500 px-3 py-2 text-xs font-semibold text-slate-950 shadow-lg shadow-cyan-500/25 transition hover:bg-cyan-400"
            type="button"
            onclick={() => {
              joinRoom();
              closeSidebar();
            }}
          >
            <iconify-icon icon="mdi:login" class="text-sm"></iconify-icon>
            Join
          </button>
        </div>

        <div>
          <p
            class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
          >
            Status
          </p>
          <div class="space-y-1.5 rounded-lg bg-slate-800/40 p-2.5">
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Role</span>
              <span class="text-[11px] font-mono text-slate-300 capitalize"
                >{peerRole ?? "none"}</span
              >
            </div>
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Socket</span>
              <span
                class="flex items-center gap-1.5 text-[11px] font-mono text-slate-300"
              >
                <span
                  class="inline-block h-1.5 w-1.5 rounded-full {signalingConnectionState ===
                  'Connected'
                    ? 'bg-emerald-400'
                    : signalingConnectionState === 'Error'
                      ? 'bg-red-400'
                      : 'bg-amber-400'}"
                ></span>
                {signalingConnectionState}
              </span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-[11px] text-slate-500">Peers</span>
              <span
                class="flex items-center gap-1.5 text-[11px] font-mono text-slate-300"
              >
                <span
                  class="inline-block h-1.5 w-1.5 rounded-full {guestPeers.length >
                  0
                    ? 'bg-emerald-400'
                    : 'bg-amber-400'}"
                ></span>
                {guestPeers.length}
              </span>
            </div>
          </div>
        </div>
        {@render notifications()}

        {#if peerRole}
          <div>
            <p
              class="mb-2 text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-500"
            >
              Devices
            </p>
            <div class="flex gap-2">
              <button
                class="flex min-h-11 flex-1 items-center justify-center gap-1.5 rounded-lg border {cameraState ===
                'Running'
                  ? 'border-emerald-500/50 bg-emerald-500/10 text-emerald-300'
                  : 'border-slate-700 text-slate-300'} px-2 py-2 text-[11px] font-medium transition hover:border-emerald-500 hover:text-emerald-300"
                type="button"
                onclick={toggleCamera}
                title={cameraState === "Running"
                  ? "Stop camera"
                  : "Start camera"}
              >
                <iconify-icon
                  icon={cameraState === "Running"
                    ? "mdi:video-off"
                    : "mdi:video"}
                  class="text-sm"
                ></iconify-icon>
              </button>
              <button
                class="flex min-h-11 flex-1 items-center justify-center gap-1.5 rounded-lg border {micState ===
                'Muted'
                  ? 'border-red-500/50 bg-red-500/10 text-red-300'
                  : micState === 'Active'
                    ? 'border-emerald-500/50 bg-emerald-500/10 text-emerald-300'
                    : 'border-slate-700 text-slate-300'} px-2 py-2 text-[11px] font-medium transition hover:border-cyan-500 hover:text-cyan-300"
                type="button"
                onclick={toggleMic}
                title={!microphoneTrack
                  ? "Start microphone"
                  : micState === "Muted"
                    ? "Unmute"
                    : "Mute"}
              >
                <iconify-icon
                  icon={micState === "Muted"
                    ? "mdi:microphone-off"
                    : "mdi:microphone"}
                  class="text-sm"
                ></iconify-icon>
              </button>
              <button
                class="flex min-h-11 flex-1 items-center justify-center gap-1.5 rounded-lg border {screenShareState ===
                'Running'
                  ? 'border-purple-500/50 bg-purple-500/10 text-purple-300'
                  : 'border-slate-700 text-slate-300'} px-2 py-2 text-[11px] font-medium transition hover:border-purple-500 hover:text-purple-300"
                type="button"
                onclick={screenShareState === "Running"
                  ? stopScreenShare
                  : startScreenShare}
                title={screenShareState === "Running"
                  ? "Stop sharing"
                  : "Share screen"}
              >
                <iconify-icon
                  icon={screenShareState === "Running"
                    ? "mdi:monitor-off"
                    : "mdi:monitor-share"}
                  class="text-sm"
                ></iconify-icon>
              </button>
            </div>
            {#if screenShareState === "Running"}
              <div class="mt-3 space-y-2 rounded-lg bg-slate-800/40 p-2.5">
                <div class="flex items-center justify-between">
                  <p
                    class="text-[10px] font-semibold uppercase tracking-[0.15em] text-slate-500"
                  >
                    Screen quality
                  </p>
                  <span class="text-[10px] text-purple-300">Live</span>
                </div>
                <select
                  class="w-full rounded border border-slate-700 bg-slate-800 px-2 py-1.5 text-[11px] text-slate-200 outline-none focus:border-purple-500"
                  bind:value={selectedScreenQualityId}
                  onchange={updateScreenQuality}
                >
                  {#each screenQualityPresets as preset}
                    <option value={preset.id}>{preset.label}</option>
                  {/each}
                </select>
                <p class="text-[10px] leading-relaxed text-slate-500">
                  Resolution and FPS are capture hints; bitrate is applied to
                  video senders when supported.
                </p>
                {#if !screenAudioAvailable}
                  <div
                    class="rounded border border-amber-400/30 bg-amber-400/10 px-2 py-2 text-[10px] leading-relaxed text-amber-200"
                    role="alert"
                  >
                    <span class="font-semibold">No transmission audio.</span>
                    Stop sharing, choose a browser tab, and enable Share tab
                    audio. On Linux, sharing an entire screen or window usually
                    captures video only.
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Bottom user area -->
    <div
      class="flex items-center gap-2.5 border-t border-slate-800 bg-slate-900/80 px-3 py-2"
    >
      <div
        class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full {isTauri
          ? 'bg-amber-500/30'
          : peerRole === 'host'
            ? 'bg-emerald-500/30'
            : peerRole === 'guest'
              ? 'bg-cyan-500/30'
              : 'bg-slate-700'}"
      >
        <iconify-icon
          icon={isTauri
            ? "mdi:server"
            : peerRole === "host"
              ? "mdi:broadcast"
              : peerRole === "guest"
                ? "mdi:account"
                : "mdi:help"}
          class="text-sm {isTauri
            ? 'text-amber-300'
            : peerRole
              ? 'text-emerald-300'
              : 'text-slate-500'}"
        >
        </iconify-icon>
      </div>
      <div class="min-w-0 flex-1">
        <p class="truncate text-xs font-medium">
          {isTauri ? "Signaling server" : displayName || "Not connected"}
        </p>
        <p class="truncate text-[10px] text-slate-500">
          {isTauri
            ? signalingStatus?.isRunning
              ? "Running on :" + signalingStatus.port
              : "Stopped"
            : room?.roomId
              ? room.roomId.slice(0, 12) + "..."
              : peerRole === "host"
                ? `${guestPeers.filter((g) => g.connected).length} guest(s)`
                : "No room"}
        </p>
      </div>
    </div>
  </aside>

  <!-- MAIN CONTENT -->
  <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
    <div
      class="app-header flex h-12 shrink-0 items-center gap-3 border-b border-slate-800 bg-slate-900/40 px-4"
    >
      <button
        class="mobile-menu-button rounded-lg border border-slate-700 p-2 text-slate-300 transition hover:border-slate-500 hover:text-white"
        type="button"
        onclick={toggleSidebar}
        aria-label={isSidebarOpen ? "Close navigation" : "Open navigation"}
        aria-expanded={isSidebarOpen}
        aria-controls="app-sidebar"
      >
        <iconify-icon
          icon={isSidebarOpen ? "mdi:close" : "mdi:menu"}
          class="text-base"
        ></iconify-icon>
      </button>
      <h2 class="text-sm font-semibold text-slate-200">
        {#if isTauri}
          Signaling Server
        {:else if peerRole === "host"}
          Room
        {:else if peerRole === "guest"}
          Room
        {:else}
          Join a stream
        {/if}
      </h2>
      <div class="flex-1"></div>
      {#if !isTauri && peerRole}
        <button
          class="relative rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-400 transition hover:border-slate-500 hover:text-white"
          type="button"
          onclick={toggleChat}
          aria-label="Toggle chat"
          title="Chat"
        >
          <iconify-icon icon="mdi:message-text-outline" class="text-sm"
          ></iconify-icon>
          {#if unreadChatMessages > 0}
            <span
              class="absolute -right-1 -top-1 min-w-4 rounded-full bg-cyan-400 px-1 text-[9px] font-bold leading-4 text-slate-950"
              >{unreadChatMessages > 99 ? "99+" : unreadChatMessages}</span
            >
          {/if}
        </button>
        <button
          class="flex items-center gap-1.5 rounded-lg bg-red-500/20 px-3 py-1 text-xs font-medium text-red-400 transition hover:bg-red-500/30"
          type="button"
          onclick={closeConnection}
        >
          <iconify-icon icon="mdi:phone-hangup" class="text-sm"></iconify-icon>
          Leave
        </button>
      {/if}
    </div>

    <div class="main-scroll flex-1 overflow-y-auto p-4">
      {#if isTauri}
        {#if room}
          <div class="mx-auto flex h-full w-full max-w-5xl flex-col gap-5 py-4">
            <div class="flex items-center justify-between">
              <div>
                <p
                  class="text-[11px] font-semibold uppercase tracking-[0.15em] text-amber-300"
                >
                  Room active
                </p>
                <h3 class="mt-1 text-2xl font-semibold text-slate-100">
                  Connection details
                </h3>
              </div>
              <span
                class="rounded-full bg-emerald-400/10 px-3 py-1 text-xs font-medium text-emerald-300"
                >{roomParticipants.length} connected</span
              >
            </div>

            <div class="grid gap-4 lg:grid-cols-2">
              <div
                class="rounded-xl border border-slate-800 bg-slate-900/60 p-4"
              >
                <p class="text-[10px] uppercase tracking-wider text-slate-500">
                  Room ID
                </p>
                <p class="mt-2 break-all font-mono text-sm text-slate-300">
                  {room.roomId}
                </p>
              </div>
              <div
                class="rounded-xl border border-slate-800 bg-slate-900/60 p-4"
              >
                <p class="text-[10px] uppercase tracking-wider text-slate-500">
                  Connected participants
                </p>
                {#if roomParticipants.length}
                  <div class="mt-3 space-y-2">
                    {#each roomParticipants as participant}
                      <div
                        class="flex items-center gap-2 text-sm text-slate-200"
                      >
                        <span class="h-2 w-2 rounded-full bg-emerald-400"
                        ></span>{participant}
                      </div>
                    {/each}
                  </div>
                {:else}
                  <p class="mt-2 text-sm text-slate-500">
                    Waiting for participants to join.
                  </p>
                {/if}
              </div>
            </div>

            <div class="grid gap-4 lg:grid-cols-2">
              <div
                class="rounded-xl border border-slate-800 bg-slate-900/60 p-4"
              >
                <div class="flex items-center justify-between">
                  <p
                    class="text-[10px] uppercase tracking-wider text-slate-500"
                  >
                    Host URL
                  </p>
                  <button
                    type="button"
                    onclick={() => copyText(browserHostUrl, "host")}
                    title="Copy"
                    ><iconify-icon
                      icon={copiedTarget === "host"
                        ? "mdi:check"
                        : "mdi:content-copy"}
                    ></iconify-icon></button
                  >
                </div>
                <p class="mt-3 break-all font-mono text-xs text-cyan-300">
                  {browserHostUrl}
                </p>
              </div>
              <div
                class="rounded-xl border border-slate-800 bg-slate-900/60 p-4"
              >
                <div class="flex items-center justify-between">
                  <p
                    class="text-[10px] uppercase tracking-wider text-slate-500"
                  >
                    Participant URL
                  </p>
                  <button
                    type="button"
                    onclick={() =>
                      copyText(
                        buildGuestUrl(room!.signalingUrl, room!.appUrl),
                        "participant",
                      )}
                    title="Copy"
                    ><iconify-icon
                      icon={copiedTarget === "participant"
                        ? "mdi:check"
                        : "mdi:content-copy"}
                    ></iconify-icon></button
                  >
                </div>
                <p class="mt-3 break-all font-mono text-xs text-purple-300">
                  {buildGuestUrl(room.signalingUrl, room.appUrl)}
                </p>
              </div>
            </div>
          </div>
        {:else}
          <!-- TAURI: Server instructions -->
          <div
            class="flex h-full flex-col items-center justify-center px-4 text-center"
          >
            <div
              class="mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-amber-500/10"
            >
              <iconify-icon icon="mdi:server" class="text-4xl text-amber-400"
              ></iconify-icon>
            </div>
            <h3 class="text-xl font-semibold text-slate-200">
              Signaling server control
            </h3>
            <p class="mt-2 max-w-md text-sm text-slate-400">
              This app runs the room signaling server. Create a room, then open
              the stream in a browser.
            </p>
            <div class="mt-6 space-y-2 text-xs text-slate-500">
              <p>
                <span class="mr-2 font-bold text-slate-400">1.</span>Click
                <span class="font-semibold text-amber-300">New Room</span> in the
                sidebar
              </p>
              <p>
                <span class="mr-2 font-bold text-slate-400">2.</span>Copy the
                <span class="font-semibold text-cyan-300">browser URL</span> and open
                in Chrome/Firefox
              </p>
              <p>
                <span class="mr-2 font-bold text-slate-400">3.</span>Share the
                <span class="font-semibold text-purple-300"
                  >participant URL</span
                > with guests
              </p>
              <p>
                <span class="mr-2 font-bold text-slate-400">4.</span>Close this
                app when done. Rooms are ephemeral.
              </p>
            </div>
          </div>
        {/if}
      {:else if isConnecting}
        <!-- BROWSER: Connecting -->
        <div
          class="flex h-full flex-col items-center justify-center px-4 text-center"
        >
          <div
            class="mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-cyan-500/10"
          >
            <iconify-icon
              icon="mdi:sync"
              class="animate-spin text-4xl text-cyan-400"
            ></iconify-icon>
          </div>
          <h3 class="text-xl font-semibold text-slate-200">Connecting...</h3>
          <p class="mt-2 max-w-md text-sm text-slate-400">
            Joining the room. This may take a moment.
          </p>
        </div>
      {:else if peerRole}
        <!-- BROWSER: Call active -->
        <div class="call-layout flex h-full min-h-0 gap-3">
          <div class={`video-grid ${videoGridClass}`}>
            <div
              class="video-tile relative overflow-hidden rounded-xl border border-slate-800 bg-slate-900/60 group"
            >
              <div
                class="absolute left-3 top-3 z-10 flex items-center gap-2 rounded-lg bg-slate-950/70 px-2.5 py-1 backdrop-blur"
              >
                <span class="text-xs text-slate-300">You</span>
                {#if micState === "Muted"}
                  <span
                    class="flex items-center gap-1 rounded bg-red-500/20 px-1.5 py-0.5 text-[10px] text-red-300"
                  >
                    <iconify-icon icon="mdi:microphone-off" class="text-xs"
                    ></iconify-icon>
                    Muted
                  </span>
                {/if}
                <span
                  class="rounded {screenShareState === 'Running'
                    ? 'bg-purple-500/30 text-purple-300'
                    : cameraState === 'Running'
                      ? 'bg-emerald-500/30 text-emerald-300'
                      : 'bg-slate-700 text-slate-400'} px-1.5 py-0.5 text-[10px]"
                  >{screenShareState === "Running"
                    ? screenAudioAvailable
                      ? "Screen + audio"
                      : "Screen"
                    : cameraState}</span
                >
              </div>
              <div
                class="absolute right-3 top-3 z-10 flex gap-1.5 opacity-0 transition-opacity group-hover:opacity-100"
              >
                <button
                  class="rounded-lg bg-slate-950/70 p-2 text-slate-300 backdrop-blur transition hover:text-white"
                  type="button"
                  onclick={() => toggleFullscreen(localVideo)}
                  title="Fullscreen"
                >
                  <iconify-icon icon="mdi:fullscreen" class="text-sm"
                  ></iconify-icon>
                </button>
              </div>
              {#if !hasActiveVideo(localStream)}
                <div
                  class="absolute inset-0 flex items-center justify-center"
                  aria-label={`${displayName} avatar`}
                >
                  <span
                    class="flex h-20 w-20 items-center justify-center rounded-full bg-cyan-500/20 text-2xl font-semibold tracking-wide text-cyan-200 transition-all {isLocalSpeaking
                      ? 'ring-2 ring-emerald-400 ring-offset-2 ring-offset-slate-900'
                      : ''}">{initials(displayName)}</span
                  >
                </div>
              {/if}
              <video
                class="h-full w-full object-cover"
                autoplay
                bind:this={localVideo}
                muted
                playsinline
              ></video>
            </div>
            {#each guestPeers as guest (guest.id)}
              <div
                class="video-tile relative overflow-hidden rounded-xl border border-slate-800 bg-slate-900/60 group"
              >
                <div
                  class="absolute left-3 top-3 z-10 rounded-lg bg-slate-950/70 px-2.5 py-1 backdrop-blur"
                >
                  <div class="flex items-center gap-2">
                    <span class="text-xs text-slate-300"
                      >{guest.displayName}</span
                    >
                    {#if guest.isMuted}
                      <span
                        class="flex items-center gap-1 rounded bg-red-500/20 px-1.5 py-0.5 text-[10px] text-red-300"
                      >
                        <iconify-icon icon="mdi:microphone-off" class="text-xs"
                        ></iconify-icon>
                        Muted
                      </span>
                    {/if}
                  </div>
                </div>
                <div
                  class="absolute right-3 top-3 z-10 flex gap-1.5 opacity-0 transition-opacity group-hover:opacity-100"
                >
                  <button
                    class="rounded-lg bg-slate-950/70 p-2 text-slate-300 backdrop-blur transition hover:text-white"
                    type="button"
                    onclick={() => toggleFullscreen(guest.videoEl)}
                    title="Fullscreen"
                  >
                    <iconify-icon icon="mdi:fullscreen" class="text-sm"
                    ></iconify-icon>
                  </button>
                </div>
                <div
                  class="absolute bottom-3 right-3 z-10 flex rounded-lg bg-slate-950/70 px-2 py-2 opacity-0 backdrop-blur transition-opacity group-hover:opacity-100"
                >
                  <div class="flex flex-col items-center gap-2">
                    <input
                      class="volume-slider cursor-pointer accent-cyan-400"
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={guest.audioVolume}
                      oninput={(e) =>
                        setGuestVolume(
                          guest,
                          parseFloat(e.currentTarget.value),
                        )}
                    />
                    <iconify-icon
                      icon={guest.audioVolume === 0
                        ? "mdi:microphone-off"
                        : "mdi:volume-high"}
                      class="text-lg text-slate-300"
                    ></iconify-icon>
                  </div>
                </div>
                {#if guest.videoState === "off" || !hasActiveVideo(guest.stream)}
                  <div
                    class="absolute inset-0 flex items-center justify-center"
                    aria-label={`${guest.displayName} avatar`}
                  >
                    <span
                      class="flex h-20 w-20 items-center justify-center rounded-full bg-purple-500/20 text-2xl font-semibold tracking-wide text-purple-200 transition-all {guest.isSpeaking
                        ? 'ring-2 ring-emerald-400 ring-offset-2 ring-offset-slate-900'
                        : ''}">{initials(guest.displayName)}</span
                    >
                  </div>
                {/if}
                <video
                  class="h-full w-full object-cover {guest.videoState ===
                    'off' || !hasActiveVideo(guest.stream)
                    ? 'invisible'
                    : ''}"
                  autoplay
                  bind:this={guest.videoEl}
                  muted
                  playsinline
                ></video>
                <audio bind:this={guest.audioEl} autoplay playsinline></audio>
              </div>
            {/each}
          </div>
          {#if isChatOpen}
            <section
              class="chat-panel flex w-80 shrink-0 flex-col overflow-hidden rounded-xl border border-slate-800 bg-slate-900/80"
              aria-label="Room chat"
            >
              <header
                class="flex items-center justify-between border-b border-slate-800 px-3 py-2.5"
              >
                <div class="flex items-center gap-2">
                  <iconify-icon
                    icon="mdi:message-text-outline"
                    class="text-cyan-300"
                  ></iconify-icon>
                  <h3 class="text-sm font-semibold text-slate-200">
                    Room chat
                  </h3>
                </div>
                <button
                  class="rounded p-1 text-slate-400 transition hover:bg-slate-800 hover:text-white"
                  type="button"
                  onclick={toggleChat}
                  aria-label="Close chat"
                  title="Close chat"
                >
                  <iconify-icon icon="mdi:close"></iconify-icon>
                </button>
              </header>
              <div
                class="flex-1 space-y-3 overflow-y-auto p-3"
                bind:this={chatHistory}
                aria-live="polite"
              >
                {#if chatMessages.length === 0}
                  <p class="pt-5 text-center text-xs text-slate-500">
                    No messages yet. Say hello to the room.
                  </p>
                {:else}
                  {#each chatMessages as message (message.id)}
                    <article
                      class="flex flex-col {message.isOwn
                        ? 'items-end'
                        : 'items-start'}"
                    >
                      <div
                        class="mb-1 flex max-w-full items-center gap-1.5 text-[10px] text-slate-500"
                      >
                        <span
                          >{participantName(
                            message.peerId,
                            message.isOwn,
                          )}</span
                        >
                        <time datetime={message.sentAt}
                          >{formatChatTime(message.sentAt)}</time
                        >
                      </div>
                      <p
                        class="max-w-full whitespace-pre-wrap break-words rounded-xl px-3 py-2 text-xs leading-relaxed {message.isOwn
                          ? 'bg-cyan-500 text-slate-950'
                          : 'bg-slate-800 text-slate-200'}"
                      >
                        {message.text}
                      </p>
                    </article>
                  {/each}
                {/if}
              </div>
              <form
                class="border-t border-slate-800 p-3"
                onsubmit={(event) => {
                  event.preventDefault();
                  sendChatMessage();
                }}
              >
                <textarea
                  class="min-h-18 w-full resize-none rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-100 outline-none placeholder:text-slate-500 focus:border-cyan-500 disabled:opacity-60"
                  bind:this={chatInput}
                  bind:value={chatDraft}
                  maxlength={MAX_CHAT_MESSAGE_LENGTH}
                  onkeydown={handleChatKeydown}
                  placeholder="Message the room"
                  aria-label="Chat message"
                  disabled={signalingConnectionState !== "Connected"}
                ></textarea>
                <div class="mt-2 flex items-center justify-between gap-2">
                  <span class="text-[10px] text-slate-500"
                    >{chatDraft.length}/{MAX_CHAT_MESSAGE_LENGTH}</span
                  >
                  <button
                    class="rounded-lg bg-cyan-500 p-1.5 text-slate-950 transition hover:bg-cyan-400 disabled:cursor-not-allowed disabled:opacity-50"
                    type="submit"
                    disabled={!chatDraft.trim() ||
                      signalingConnectionState !== "Connected"}
                    aria-label="Send message"
                    title="Send message"
                  >
                    <iconify-icon icon="mdi:send" class="text-sm"
                    ></iconify-icon>
                  </button>
                </div>
              </form>
            </section>
          {/if}
        </div>
      {:else}
        <!-- BROWSER: No call -->
        <div
          class="flex h-full flex-col items-center justify-center px-4 text-center"
        >
          <div
            class="mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-emerald-500/10"
          >
            <iconify-icon icon="mdi:video" class="text-4xl text-emerald-400"
            ></iconify-icon>
          </div>
          <h3 class="text-xl font-semibold text-slate-200">
            Join a streaming session
          </h3>
          <p class="mt-2 max-w-md text-sm text-slate-400">
            Open the room link shared by the host, enter your name, and click
            <span class="font-semibold text-cyan-300">Join</span> to connect.
          </p>
          <div class="mt-6 space-y-2 text-xs text-slate-500">
            <p>
              <span class="mr-2 font-bold text-slate-400">1.</span>Open the
              participant link shared by the host
            </p>
            <p>
              <span class="mr-2 font-bold text-slate-400">2.</span>Enter the
              name you want participants to see
            </p>
            <p>
              <span class="mr-2 font-bold text-slate-400">3.</span>Click
              <span class="font-semibold text-cyan-300">Join</span> to enter the room
              and share your screen when needed
            </p>
          </div>
        </div>
      {/if}
    </div>
  </div>
</main>
