#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

pushd "$ROOT_DIR" >/dev/null
cargo build --release -p ncp
cp "$ROOT_DIR/target/release/ncp" "$BIN_DIR/ncp"
popd >/dev/null

if ! grep -q 'export PATH="$HOME/.local/bin:$PATH"' "$HOME/.zshrc" 2>/dev/null; then
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
fi

echo "Installed ncp to $BIN_DIR/ncp"
echo "Run: source ~/.zshrc"
echo "Then: ncp --help"
