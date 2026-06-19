# NetCat++

A modern cross-platform networking toolkit with a Rust backend, CLI, and native SwiftUI frontend.

- Port scanning (TCP/UDP, subnet ping sweep)
- File transfer with optional compression & encryption
- Direct TCP text messaging
- HTTP client (GET, POST, PUT, PATCH, DELETE)
- Live packet sniffer (requires `sudo`)
- System metrics dashboard (TUI + SwiftUI)
- macOS native app (SwiftUI via UniFFI/FFI bridge)

---

## Workspace Layout

| Path | Purpose |
|---|---|
| `crates/ncp-core` | Core networking backend (scanner, transfer, HTTP, sniffer, dashboard, system metrics) |
| `crates/ncp-cli` | `ncp` CLI binary |
| `crates/ncp-ffi` | UniFFI bridge for native frontends (Swift, Android) |
| `swiftui/NCPKit` | Swift package wrapping Rust FFI bindings |
| `swiftui/NCPDashApp` | Native SwiftUI macOS app |

---

## Quick Start — CLI

### Install

```bash
cargo build --release -p ncp
cp target/release/ncp ~/.local/bin/
```

### Usage

```bash
# Port scan
ncp scan example.com --range 1-1000 --timeout 500ms
ncp scan 192.168.1.1 --range 22,80,443           # specific ports
ncp scan 192.168.1.0/24 --subnet 192.168.1.0/24  # ping sweep

# HTTP
ncp http https://api.example.com
ncp http https://httpbin.org/post -X POST --body '{"key":"value"}' -H "Content-Type: application/json"

# File transfer
ncp send 192.168.1.100 9000 ./myfile.txt --compress
ncp receive 9000

# Text messaging
ncp text-listen 9000 --keep-alive
ncp text-send 127.0.0.1 9000 "hello" --repeat 3 --interval 500ms

# Packet capture (requires sudo)
sudo ncp sniff --interface en0

# TUI dashboard (requires sudo for packet view)
sudo ncp dash
```

---

## SwiftUI macOS App

### Build & Run (SPM)

```bash
# 1. Build Rust FFI library
cargo build --release -p ncp-ffi

# 2. Copy dylib
cp target/release/libncpffi.dylib swiftui/NCPKit/Sources/
cp target/release/libncpffi.dylib swiftui/NCPKit/Sources/NCPKit.xcframework/macos-arm64/

# 3. Run the app
cd swiftui/NCPDashApp
swift run -c release
```

### One-command launcher

```bash
./start.sh            # build & run in debug mode
./start.sh sudo       # run with sudo for packet capture
```

### Package as .app bundle

```bash
./scripts/package_app.sh
open dist/NetCat++.app
```

### Open in Xcode

Add `swiftui/NCPKit` and `swiftui/NCPDashApp` as local Swift packages, then build & run.

---

## Permissions

Packet capture requires root:

```bash
sudo ncp sniff --interface en0
sudo ncp dash
```

---

## All Bugs Fixed / v1.0.0

- ✅ CPU & memory values now accurate (persistent system stats — continuous measurement window)
- ✅ Memory pressure parsing fixed (removed broken `-Q` flag)
- ✅ Port ranges support comma-separated lists (e.g. `22,80,443`)
- ✅ Swift Package Manager links FFI dylib correctly via xcframework binary target
- ✅ Packet data JSON decoding fixed (`protocol` → `protocol_` coding key)
- ✅ Release .app bundle with portable dylib linking
- ✅ Zero compiler warnings across all crates

---

## Android Compatibility

- `ncp-core` is portable and ready for Android reuse.
- Packet capture & TUI dashboard are disabled on Android with explicit fallbacks.
- System/process snapshot APIs work on all platforms.
