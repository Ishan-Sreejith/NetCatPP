#!/bin/bash
set -e

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Building NetCat++ Rust backend..."
cd "$ROOT_DIR"
./scripts/build_swift_bridge.sh

echo "Building SwiftUI app (release)..."
cd "$ROOT_DIR/swiftui/NCPDashApp"
swift build -c release

echo ""
echo "Starting NetCat++ Dashboard..."
echo "Note: Packet capture requires sudo for network access"
echo ""

if [ "$1" = "sudo" ]; then
    echo "Running with sudo for packet capture support..."
    sudo .build/arm64-apple-macosx/release/NCPDashApp
else
    .build/arm64-apple-macosx/release/NCPDashApp
fi
