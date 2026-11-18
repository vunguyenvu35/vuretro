#!/usr/bin/env bash
# Build the runner in release mode and put it where Tauri's `externalBin` expects it:
# src-tauri/binaries/vuretro-runner-<target triple>[.exe]. Usage: scripts/prepare-sidecar.sh [triple]
set -euo pipefail
cd "$(dirname "$0")/.."

TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
EXT=""; [[ "$TRIPLE" == *windows* ]] && EXT=".exe"

if [[ "$TRIPLE" == "universal-apple-darwin" ]]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$t" >/dev/null
    cargo build --release -p vuretro-runner --target "$t"
  done
  mkdir -p src-tauri/binaries
  lipo -create -output "src-tauri/binaries/vuretro-runner-$TRIPLE" \
    target/aarch64-apple-darwin/release/vuretro-runner \
    target/x86_64-apple-darwin/release/vuretro-runner
  # tauri builds each architecture separately before joining them, and wants a sidecar for each
  cp target/aarch64-apple-darwin/release/vuretro-runner "src-tauri/binaries/vuretro-runner-aarch64-apple-darwin"
  cp target/x86_64-apple-darwin/release/vuretro-runner "src-tauri/binaries/vuretro-runner-x86_64-apple-darwin"
else
  cargo build --release -p vuretro-runner --target "$TRIPLE"
  mkdir -p src-tauri/binaries
  cp "target/$TRIPLE/release/vuretro-runner$EXT" "src-tauri/binaries/vuretro-runner-$TRIPLE$EXT"
fi
echo "sidecar ready: $(ls src-tauri/binaries)"
