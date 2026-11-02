# Architecture

## Goals

- Low latency
- Low resource usage
- Clean separation of concerns
- Easy to extend (multi-window, audio, game controls, recording)

## High-level flow

```
┌─────────────┐     ADB      ┌──────────────────┐
│  Adex CLI / │ ───────────► │  Android Device  │
│  GUI        │              │                  │
│             │   TCP        │  scrcpy-server   │
│  Session    │ ◄─────────── │  (MediaCodec)    │
└─────────────┘   H.264 +    └──────────────────┘
                  control
```

1. Discover device via ADB
2. Push `scrcpy-server` jar
3. Start server process on device (it opens an abstract socket)
4. `adb forward` local TCP port → device socket
5. Client connects, receives device meta + continuous video packets
6. Control messages flow client → device on the same connection

## Crates

### `adex-core`

- `adb` — thin async wrapper around system `adb` (future: pure-Rust `rsadb`)
- `scrcpy::protocol` — handshake, video packet framing, control message encoding
- `scrcpy::server` — push + start + cleanup of the server jar
- `session` — high-level session that owns the TCP connection and event stream

### `adex` (gui crate)

Currently provides a solid CLI. The same crate will host the egui/wgpu frontend.

## Video path (current → future)

**Now**
- Receive framed H.264 NAL units
- Emit `SessionEvent::Video`

**Next**
- Decode (openh264 or ffmpeg / hardware)
- Upload to wgpu texture
- Render in egui or raw wgpu window

## Control path

`ControlMessage` enum covers the important scrcpy control types (touch, key, scroll, clipboard, power, rotate…). Messages are encoded and written on a dedicated task.

## Extensibility points

- Multiple sessions (multi-device)
- Multiple video surfaces (multi-window / per-app)
- Input remapping layer (game profiles)
- Audio stream (scrcpy already supports it)
