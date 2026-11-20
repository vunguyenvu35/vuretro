#!/usr/bin/env bash
# Download the zips of the libretro cores for the current OS into cores/offline/<platform>/,
# so Vuretro can install them with no internet (Cores & BIOS, or Play). The app looks in
# cores/offline next to it, in <data dir>/cores-offline, and in $VURETRO_CORES_OFFLINE.
#
# Usage: ./cores/fetch-offline.sh            the default core of every system (not 3D ones)
#        ./cores/fetch-offline.sh --all      also the alternative cores (MAME 2003, FBNeo…)
#        ./cores/fetch-offline.sh fbneo mgba only these
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="https://buildbot.libretro.com/nightly"
case "$(uname -s)" in
  Darwin)
    ARCH="$(uname -m)"; [[ "$ARCH" == "arm64" ]] || ARCH="x86_64"
    PLATFORM="apple/osx/$ARCH"; EXT="dylib" ;;
  Linux) PLATFORM="linux/x86_64"; EXT="so" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows/x86_64"; EXT="dll" ;;
  *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac
DEST="$ROOT/cores/offline/$PLATFORM"
mkdir -p "$DEST"

if [[ $# -gt 0 && "$1" != "--all" ]]; then
  CORES=("$@")
else
  ALT=false; [[ "${1:-}" == "--all" ]] && ALT=true
  CORES=()
  while IFS= read -r c; do CORES+=("$c"); done < <(python3 - "$ROOT/crates/core/data/systems.json" "$ALT" <<'PY'
import json, sys
systems = json.load(open(sys.argv[1]))
alt = sys.argv[2] == "true"
seen = []
for s in systems:
    if s.get("hardware"):
        continue
    for c in [s["core"]] + (s.get("alt_cores", []) if alt else []):
        c = c.removesuffix("_libretro")
        if c not in seen:
            seen.append(c)
print("\n".join(seen))
PY
)
fi

for core in "${CORES[@]}"; do
  core="${core%_libretro}"
  file="${core}_libretro.${EXT}.zip"
  if [[ -f "$DEST/$file" ]]; then echo "= $file (already here)"; continue; fi
  echo "→ $file"
  curl -fSL --retry 3 "$BASE/$PLATFORM/latest/$file" -o "$DEST/$file.part" && mv "$DEST/$file.part" "$DEST/$file" \
    || { rm -f "$DEST/$file.part"; echo "  not available: $file" >&2; }
done

echo "Offline cores in $DEST:"
ls -1sh "$DEST"
