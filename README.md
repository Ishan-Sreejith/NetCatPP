# NetCat++

A cross-platform networking toolkit with a Rust backend, CLI, and native SwiftUI frontend.

- Port scanning (TCP/UDP, subnet ping sweep)
- File transfer with optional compression & encryption
- Direct TCP text messaging
- HTTP client (GET, POST, PUT, PATCH, DELETE)
- Live packet sniffer (requires root)
- System metrics dashboard (TUI + SwiftUI)

## Workspace Layout

| Path | Purpose |
|---|---|
| `crates/ncp-core` | Core networking backend (scanner, transfer, HTTP, sniffer, dashboard, system metrics) |
| `crates/ncp-cli` | `ncp` CLI binary |
| `crates/ncp-ffi` | UniFFI bridge for native frontends (Swift, Android) |
| `crates/ncp-capture-helper` | Standalone privileged helper for packet capture |
| `tools/uniffi-gen` | UniFFI binding generator for the FFI bridge |
| `swiftui/NCPKit` | Swift package wrapping Rust FFI bindings |
| `swiftui/NCPDashApp` | Native SwiftUI macOS app |
| `scripts/` | Build and packaging scripts |

## CLI

### Install

```bash
cargo build --release -p ncp
cp target/release/ncp ~/.local/bin/
```

### Usage

```
# Port scan
ncp scan example.com --range 1-1000 --timeout 500ms
ncp scan 192.168.1.1 --range 22,80,443
ncp scan 192.168.1.0/24 --subnet 192.168.1.0/24

# HTTP
ncp http https://api.example.com
ncp http https://httpbin.org/post -X POST --body '{"key":"value"}'

# File transfer
ncp send 192.168.1.100 9000 ./myfile.txt --compress
ncp receive 9000

# Text messaging
ncp text-listen 9000 --keep-alive
ncp text-send 127.0.0.1 9000 "hello" --repeat 3 --interval 500ms

# Packet capture (requires root)
sudo ncp sniff --interface en0

# TUI dashboard
sudo ncp dash
```

## SwiftUI macOS App

### Build & Run

```bash
# One-time: build Rust FFI and regenerate Swift bindings
bash scripts/build_swift_bridge.sh

# Build and run the SwiftUI app
cd swiftui/NCPDashApp
swift run
```

### Quick start

```bash
./start.sh            # build & run in debug mode
./start.sh sudo       # run with sudo for packet capture
```

### Package as .app

```bash
bash scripts/package_app.sh
open dist/NetCat++.app
```

### Packet capture

The app uses FFI-based capture (works if run with sudo). For normal use, run the helper binary with root:

```bash
# Build the helper (one-time)
cargo build --release -p ncp-capture-helper

# Run it alongside the app
sudo target/release/ncp-capture-helper --interface en0 > /tmp/ncp-capture-output
```

The app reads packets from `/tmp/ncp-capture-output` automatically while capture is active.

## Permissions

Packet capture requires root. The app requests escalation or you can run the helper manually as described above.

## Build Requirements

- Rust (latest stable)
- Xcode 15+ (for SwiftUI app)
- macOS 13+ (for SwiftUI app)
