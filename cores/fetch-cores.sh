#!/usr/bin/env bash
# Download libretro cores for the current OS from the libretro buildbot
# into cores/bin/. Usage: ./cores/fetch-cores.sh [core ...]
set -euo pipefail

if [[ $# -gt 0 ]]; then CORES=("$@"); else CORES=(fbneo pcsx_rearmed snes9x mgba fceumm gambatte genesis_plus_gx mednafen_pce_fast); fi
BASE="https://buildbot.libretro.com/nightly"
DEST="$(cd "$(dirname "$0")" && pwd)/bin"
mkdir -p "$DEST"

case "$(uname -s)" in
  Darwin)
    ARCH="$(uname -m)"; [[ "$ARCH" == "arm64" ]] || ARCH="x86_64"
    URL="$BASE/apple/osx/$ARCH/latest"; EXT="dylib" ;;
  Linux)
    URL="$BASE/linux/x86_64/latest"; EXT="so" ;;
  MINGW*|MSYS*|CYGWIN*)
    URL="$BASE/windows/x86_64/latest"; EXT="dll" ;;
  *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

for core in "${CORES[@]}"; do
  file="${core}_libretro.${EXT}"
  echo "→ $file"
  tmp="$(mktemp -d)"
  curl -fsSL "$URL/$file.zip" -o "$tmp/core.zip"
  unzip -o -q "$tmp/core.zip" -d "$DEST"
  rm -rf "$tmp"
done

echo "Cores in $DEST:"
ls -1 "$DEST"
