# Adex

**High-performance Android screen mirroring and desktop control — written in pure Rust.**

Adex aims to be a modern, low-latency open-source foundation for an Android desktop experience (inspired by Samsung DeX / Android-Dex style interfaces).

> Status: **v0.1** — solid core + polished desktop GUI shell. Video decode/render and full control next.

## Features

### Core
- Pure Rust (`adex-core`)
- Device discovery (USB + wireless via ADB)
- scrcpy-server lifecycle + wire protocol
- Async session with event stream
- Clean CLI (`adex devices`, `adex mirror`)

### Desktop GUI (default)
- Dark modern UI inspired by Android DeX
- Sidebar navigation (Home / Devices / Mirror / Apps / Media / Settings)
- Live device list with status
- Mirror surface placeholder ready for H.264 + wgpu
- App grid & media stubs for future multi-window mode

### Infrastructure
- Multi-platform CI (Linux, Windows, macOS x64/arm64)
- Release automation on tags
- LTO + optimized release profile

## Quick start

```bash
git clone https://github.com/kekih/adex.git
cd adex
cargo run --release          # launches the desktop GUI

# or pure CLI
cargo run --release -- devices
cargo run --release -- mirror --server path/to/scrcpy-server
```

Requirements:
- Rust 1.80+
- `adb` in PATH
- scrcpy-server jar for actual mirroring sessions

## Architecture

```
adex/
├── crates/
│   ├── adex-core/     # ADB, scrcpy protocol, session
│   └── adex-gui/      # egui desktop shell + CLI
├── .github/workflows/
└── docs/
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Roadmap

- [x] Core protocol + session
- [x] Desktop shell (DeX-style)
- [ ] H.264 decode + wgpu texture
- [ ] Mouse / keyboard injection
- [ ] Audio
- [ ] Multi-window / per-app
- [ ] Game keymapper

## License

MIT OR Apache-2.0

## Acknowledgements

Built on the excellent [scrcpy](https://github.com/Genymobile/scrcpy) protocol and server by Romain Vimont / Genymobile.
