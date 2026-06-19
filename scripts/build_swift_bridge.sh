#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT_DIR/swiftui/NCPKit/Sources/NCPKit/Generated"
FFI_DIR="$ROOT_DIR/swiftui/NCPKit/Sources/ncpffiFFI"
mkdir -p "$OUT_DIR" "$FFI_DIR/include"

pushd "$ROOT_DIR" >/dev/null

cargo build --release -p ncp-ffi

LIB_PATH="$ROOT_DIR/target/release/libncpffi.dylib"
if [[ ! -f "$LIB_PATH" ]]; then
  echo "Missing compiled library: $LIB_PATH" >&2
  exit 1
fi

install_name_tool -id "@rpath/libncpffi.dylib" "$LIB_PATH"
install_name_tool -add_rpath "@executable_path/../Frameworks" "$LIB_PATH"
install_name_tool -add_rpath "@loader_path" "$LIB_PATH"

DEST_DIR="$ROOT_DIR/swiftui/NCPKit/Sources"
cp "$LIB_PATH" "$DEST_DIR/libncpffi.dylib"

cargo run -p uniffi-gen -- "$LIB_PATH" "$OUT_DIR"

cp "$OUT_DIR/ncpffiFFI.h" "$FFI_DIR/include/ncpffiFFI.h"
cp "$OUT_DIR/ncpffiFFI.modulemap" "$FFI_DIR/include/module.modulemap"

cat <<MSG
Swift bindings generated in:
  $OUT_DIR

C FFI module synced to:
  $FFI_DIR

Rust library:
  $DEST_DIR/libncpffi.dylib (macOS)
MSG

popd >/dev/null
