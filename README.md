# Adex

**High-performance Android screen mirroring and desktop control — written in pure Rust.**

Adex is a modern, low-latency alternative foundation for Android desktop experiences. It uses the battle-tested scrcpy protocol and server while providing a clean, optimised Rust client designed for speed and future extensibility (multi-window, gaming controls, audio, etc.).

> Status: **v0.1 foundation** — solid core (ADB + protocol + session) is ready. Full video decode/render and polished GUI are next.

## Features (current)

- Pure Rust core (`adex-core`)
- Device discovery (USB + wireless via ADB)
- Automatic scrcpy-server push & lifecycle management
- Scrcpy wire protocol (video packets + control messages)
- Async session with event stream
- Clean CLI (`adex devices`, `adex mirror`)
- Multi-platform CI (Linux, Windows, macOS x64/arm64)
- Release automation on tags

## Planned

- Hardware-accelerated H.264 decode + wgpu rendering
- Full mouse / keyboard / touch injection
- Audio forwarding
- Multi-device & multi-window mode
- Game key-mapping profiles
- Recording

## Requirements

- Rust 1.80+
- `adb` (Android platform-tools) in `PATH`
- scrcpy-server jar (official builds from [Genymobile/scrcpy](https://github.com/Genymobile/scrcpy/releases))

## Quick start

```bash
# Clone
git clone https://github.com/kekih/adex.git
cd adex

# Build
cargo build --release

# List devices
./target/release/adex devices

# Start mirroring (place scrcpy-server next to the binary or pass --server)
./target/release/adex mirror --max-size 1920 --bit-rate 8000000
```

## Architecture

```
adex/
├── crates/
│   ├── adex-core/     # ADB, scrcpy protocol, session, video pipeline
│   └── adex-gui/      # CLI + future egui frontend (binary name: adex)
├── .github/workflows/ # CI + multi-platform releases
└── docs/
```

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for details.

## Performance focus

- LTO + single codegen unit in release
- Minimal allocations on the video path
- Async I/O with tokio
- Designed so the hot path can later become zero-copy (GPU textures)

## License

Licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.

## Acknowledgements

- [scrcpy](https://github.com/Genymobile/scrcpy) by Romain Vimont / Genymobile — the excellent server and protocol this project builds upon.
