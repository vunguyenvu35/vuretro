#!/usr/bin/env bash
# Save a copy of what Vuretro downloads for PC (Windows) games (Wine, DXVK, cnc-ddraw, winetricks)
# into cores/offline/downloads/<sha256>, so that a game can still be set up if the site that
# serves one of them is gone or there is no internet. Each file is checked against the SHA-256
# that the app pins (crates/library/src/adapters/wine_installer.rs); the app looks for these
# copies before it downloads. Existing copies are kept.
#
# Usage: ./cores/fetch-downloads.sh          (about 235 MB, nearly all of it Wine)
set -euo pipefail

DEST="$(cd "$(dirname "$0")" && pwd)/offline/downloads"
mkdir -p "$DEST"

# url sha256
FILES=(
  "https://github.com/Gcenx/macOS_Wine_builds/releases/download/11.21/wine-staging-11.21-osx64.tar.xz d7bcd5abf4df27aae397a7898aef074fea4a2e0b51e0a55aa9659517bbf9d452"
  "https://github.com/Gcenx/DXVK-macOS/releases/download/v1.10.3-20230507-repack/dxvk-macOS-async-v1.10.3-20230507-repack.tar.gz acd1520ad105d8ef124a09c8e11a259a5dc8bdc565ad18e0e52693f9807b2477"
  "https://github.com/FunkyFr3sh/cnc-ddraw/releases/download/v7.1.0.0/cnc-ddraw.zip 0b13ab89a64c9918189b1dadd449ef6ed3cb3b7b19cabd96d8adbd95505bb908"
  "https://raw.githubusercontent.com/Winetricks/winetricks/20240105/src/winetricks 8f28985b06e9e04b5268e2aef3429b2a0f6abd4fd12c4a30472dfe66355128fa"
)

fail=0
for entry in "${FILES[@]}"; do
  url="${entry% *}"; sha="${entry##* }"
  if [[ -f "$DEST/$sha" ]] && [[ "$(shasum -a 256 "$DEST/$sha" | cut -d' ' -f1)" == "$sha" ]]; then
    echo "= $(basename "$url") (already here, checked)"; continue
  fi
  echo "→ $(basename "$url")"
  curl -fSL --retry 3 "$url" -o "$DEST/$sha.part" || { rm -f "$DEST/$sha.part"; echo "  failed: $url" >&2; fail=1; continue; }
  got="$(shasum -a 256 "$DEST/$sha.part" | cut -d' ' -f1)"
  if [[ "$got" == "$sha" ]]; then mv "$DEST/$sha.part" "$DEST/$sha"
  else rm -f "$DEST/$sha.part"; echo "  not the file expected (SHA-256 $got): $url" >&2; fail=1; fi
done
exit $fail
