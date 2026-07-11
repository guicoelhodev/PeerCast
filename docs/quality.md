# Streaming Quality

## Goal

Users must be able to manually choose streaming quality for screen sharing. The MVP exposes clear presets while a screen share is active.

## Presets

| Preset          | Resolution | FPS |  Bitrate | Minimum Upload |
| --------------- | ---------: | --: | -------: | -------------: |
| Low             |   1280x720 |  30 | 2.5 Mbps |         5 Mbps |
| Balanced        |  1920x1080 |  30 |   5 Mbps |        10 Mbps |
| High            |  1920x1080 |  60 |   8 Mbps |        15 Mbps |
| Ultra           |  2560x1440 |  60 |  14 Mbps |        25 Mbps |
| 4K Experimental |  3840x2160 |  30 |  20 Mbps |       40+ Mbps |

## Application Strategy

Quality will be applied using browser WebRTC APIs where available:

- Request screen capture with ideal width, height and frame-rate constraints.
- Configure video sender parameters with `RTCRtpSender.getParameters()` and `setParameters()`.
- Set `encodings[0].maxBitrate` for target bitrate.
- Set `encodings[0].maxFramerate` where supported.
- Use `scaleResolutionDownBy` where supported.
- Use `MediaStreamTrack.applyConstraints()` as a fallback when sender parameters are incomplete.
- Restart screen sharing if live changes cannot be applied reliably.

## Live Changes

Live bitrate changes should be attempted through `RTCRtpSender.setParameters()`. Live resolution/FPS changes may require applying constraints or replacing/restarting the screen track depending on WebView/platform behavior.

If live changes are unreliable, the UI should explain that changes apply on the next screen-share start.

## Known Limitations

- `getDisplayMedia()` support varies by browser/WebView and platform.
- Some screen capture constraints are treated as hints and may be applied only after the user selects a capture source.
- `RTCRtpSender.setParameters()` support varies for `maxBitrate`, `maxFramerate` and `scaleResolutionDownBy`.
- Actual bitrate can be lower than the selected bitrate because of network capacity, encoder behavior or WebRTC congestion control.
- High presets require significant upload bandwidth and CPU/GPU encoding capacity.

## Monitoring

Phase 8 should use WebRTC stats to show approximate sent bitrate, packet loss, round-trip time and selected quality. This is required so users can see whether the selected preset is realistic for their network.
