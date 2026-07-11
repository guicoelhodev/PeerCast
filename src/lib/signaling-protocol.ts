export const MAX_CHAT_MESSAGE_LENGTH = 500;
export const MAX_DISPLAY_NAME_LENGTH = 50;

export type SignalMessage =
  | {
      type: "ready";
      peerId: string;
      displayName?: string;
      microphoneMuted?: boolean;
    }
  | { type: "participant-left"; peerId: string }
  | {
      type: "offer";
      peerId: string;
      targetPeerId: string;
      isHost?: boolean;
      displayName?: string;
      microphoneMuted?: boolean;
      description: RTCSessionDescriptionInit;
    }
  | {
      type: "answer";
      peerId: string;
      targetPeerId: string;
      displayName?: string;
      microphoneMuted?: boolean;
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
    }
  | { type: "microphone-state"; peerId: string; microphoneMuted: boolean }
  | {
      type: "chat";
      peerId: string;
      messageId: string;
      text: string;
      sentAt: string;
    };

export function serializeSignalMessage(message: SignalMessage): string {
  return JSON.stringify(message);
}

export function parseSignalMessage(data: string): SignalMessage | null {
  try {
    const value: unknown = JSON.parse(data);

    if (
      !isRecord(value) ||
      typeof value.type !== "string" ||
      typeof value.peerId !== "string"
    ) {
      return null;
    }

    const { peerId } = value;

    switch (value.type) {
      case "ready":
        if (
          typeof value.displayName === "string" &&
          isValidDisplayName(value.displayName)
        ) {
          return {
            type: "ready",
            peerId,
            displayName: value.displayName.trim(),
            ...(typeof value.microphoneMuted === "boolean"
              ? { microphoneMuted: value.microphoneMuted }
              : {}),
          };
        }
        return {
          type: "ready",
          peerId,
          ...(typeof value.microphoneMuted === "boolean"
            ? { microphoneMuted: value.microphoneMuted }
            : {}),
        };
      case "participant-left":
        return { type: "participant-left", peerId };
      case "offer":
      case "answer":
        if (
          typeof value.targetPeerId === "string" &&
          isSessionDescription(value.description)
        ) {
          return {
            type: value.type,
            peerId,
            targetPeerId: value.targetPeerId,
            ...(typeof value.isHost === "boolean"
              ? { isHost: value.isHost }
              : {}),
            ...(typeof value.displayName === "string" &&
            isValidDisplayName(value.displayName)
              ? { displayName: value.displayName.trim() }
              : {}),
            ...(typeof value.microphoneMuted === "boolean"
              ? { microphoneMuted: value.microphoneMuted }
              : {}),
            description: value.description,
          };
        }
        return null;
      case "ice":
        if (
          typeof value.targetPeerId === "string" &&
          isIceCandidate(value.candidate)
        ) {
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
      case "microphone-state":
        return typeof value.microphoneMuted === "boolean"
          ? {
              type: "microphone-state",
              peerId,
              microphoneMuted: value.microphoneMuted,
            }
          : null;
      case "chat":
        if (
          typeof value.messageId === "string" &&
          value.messageId.length > 0 &&
          typeof value.text === "string" &&
          value.text.trim().length > 0 &&
          value.text.length <= MAX_CHAT_MESSAGE_LENGTH &&
          typeof value.sentAt === "string" &&
          value.sentAt.length > 0 &&
          !Number.isNaN(Date.parse(value.sentAt))
        ) {
          return {
            type: "chat",
            peerId,
            messageId: value.messageId,
            text: value.text,
            sentAt: value.sentAt,
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

function isValidDisplayName(value: string) {
  return value.trim().length > 0 && value.length <= MAX_DISPLAY_NAME_LENGTH;
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
