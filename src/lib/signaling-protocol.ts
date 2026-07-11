export type SignalMessage =
  | { type: "ready"; peerId: string }
  | { type: "participant-left"; peerId: string }
  | {
      type: "offer";
      peerId: string;
      targetPeerId: string;
      isHost?: boolean;
      description: RTCSessionDescriptionInit;
    }
  | {
      type: "answer";
      peerId: string;
      targetPeerId: string;
      description: RTCSessionDescriptionInit;
    }
  | {
      type: "ice";
      peerId: string;
      targetPeerId: string;
      candidate: RTCIceCandidateInit;
    }
  | {
      type: "audio-kind";
      peerId: string;
      targetPeerId: string;
      audioKind: "microphone" | "system";
    };

export function serializeSignalMessage(message: SignalMessage): string {
  return JSON.stringify(message);
}

export function parseSignalMessage(data: string): SignalMessage | null {
  try {
    const value: unknown = JSON.parse(data);

    if (!isRecord(value) || typeof value.type !== "string" || typeof value.peerId !== "string") {
      return null;
    }

    const { peerId } = value;

    switch (value.type) {
      case "ready":
        return { type: "ready", peerId };
      case "participant-left":
        return { type: "participant-left", peerId };
      case "offer":
      case "answer":
        if (typeof value.targetPeerId === "string" && isSessionDescription(value.description)) {
          return {
            type: value.type,
            peerId,
            targetPeerId: value.targetPeerId,
            ...(typeof value.isHost === "boolean" ? { isHost: value.isHost } : {}),
            description: value.description,
          };
        }
        return null;
      case "ice":
        if (typeof value.targetPeerId === "string" && isIceCandidate(value.candidate)) {
          return {
            type: "ice",
            peerId,
            targetPeerId: value.targetPeerId,
            candidate: value.candidate,
          };
        }
        return null;
      case "audio-kind":
        if (
          typeof value.targetPeerId === "string" &&
          (value.audioKind === "microphone" || value.audioKind === "system")
        ) {
          return {
            type: "audio-kind",
            peerId,
            targetPeerId: value.targetPeerId,
            audioKind: value.audioKind,
          };
        }
        return null;
      default:
        return null;
    }
  } catch {
    return null;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isSessionDescription(
  value: unknown,
): value is RTCSessionDescriptionInit {
  return (
    isRecord(value) &&
    (value.type === "offer" || value.type === "answer") &&
    typeof value.sdp === "string"
  );
}

function isIceCandidate(value: unknown): value is RTCIceCandidateInit {
  return (
    isRecord(value) &&
    typeof value.candidate === "string" &&
    (typeof value.sdpMid === "string" || value.sdpMid === null) &&
    (typeof value.sdpMLineIndex === "number" || value.sdpMLineIndex === null)
  );
}
