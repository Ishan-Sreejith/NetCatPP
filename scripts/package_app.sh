#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/swiftui/NCPDashApp/.build/arm64-apple-macosx/release"
APP_NAME="NetCat++.app"
APP_DIR="$ROOT_DIR/dist/$APP_NAME"

echo "Building Rust backend..."
cd "$ROOT_DIR"
cargo build --release -p ncp-ffi -p ncp-capture-helper

echo "Building SwiftUI app (release)..."
cd "$ROOT_DIR/swiftui/NCPDashApp"
swift build -c release

echo "Creating .app bundle..."
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Frameworks"
mkdir -p "$APP_DIR/Contents/Resources"

cp "$BUILD_DIR/NCPDashApp" "$APP_DIR/Contents/MacOS/NCPDashApp"
cp "$ROOT_DIR/target/release/libncpffi.dylib" "$APP_DIR/Contents/Frameworks/libncpffi.dylib"
cp "$ROOT_DIR/target/release/ncp-capture-helper" "$APP_DIR/Contents/MacOS/ncp-capture-helper"

# Fix library load path for portability
OLD_PATH=$(otool -L "$APP_DIR/Contents/MacOS/NCPDashApp" | grep libncpffi | awk '{print $1}')
if [ -n "$OLD_PATH" ] && [ "$OLD_PATH" != "@executable_path/../Frameworks/libncpffi.dylib" ]; then
    install_name_tool -change "$OLD_PATH" @executable_path/../Frameworks/libncpffi.dylib "$APP_DIR/Contents/MacOS/NCPDashApp"
fi

cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>NCPDashApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.netcatpp.dashboard</string>
    <key>CFBundleName</key>
    <string>NetCat++</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
EOF

echo ""
echo "App bundle created at: $APP_DIR"
echo "Run with: open $APP_DIR"
echo ""

cd "$ROOT_DIR"
echo "Distribution size:"
du -sh "$APP_DIR"
