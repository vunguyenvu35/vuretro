#!/usr/bin/env bash
# Prepare a fresh machine so `make dev` works: Xcode CLT (macOS), Node, Rust.
# Safe to re-run; installs only what is missing.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

case "$(uname -s)" in
  Darwin)
    if ! xcode-select -p >/dev/null 2>&1; then
      echo "→ Xcode Command Line Tools missing. A dialog will open; finish it, then re-run: make dev"
      xcode-select --install || true
      exit 1
    fi ;;
  Linux)
    echo "→ Linux: Tauri needs system libs (webkit2gtk, libsoup, SDL/ALSA...). See https://tauri.app/start/prerequisites/" ;;
esac

if ! command -v node >/dev/null || ! command -v npm >/dev/null; then
  echo "✗ Node.js not found. Install Node 20+ (https://nodejs.org or nvm), then re-run." >&2
  exit 1
fi

if ! command -v cargo >/dev/null; then
  echo "→ Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi

# rust-toolchain.toml pins channel + components; this fetches them.
cargo --version >/dev/null
echo "✓ Toolchain ready: $(cargo --version), node $(node --version)"
