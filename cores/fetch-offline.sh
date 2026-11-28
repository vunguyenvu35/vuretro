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
# PLATFORM=windows/x86_64 (or linux/x86_64, apple/osx/arm64, apple/osx/x86_64) fetches the cores
# of another computer than this one, e.g. to keep them for a Windows PC.
if [[ -z "${PLATFORM:-}" ]]; then
  case "$(uname -s)" in
    Darwin)
      ARCH="$(uname -m)"; [[ "$ARCH" == "arm64" ]] || ARCH="x86_64"
      PLATFORM="apple/osx/$ARCH" ;;
    Linux) PLATFORM="linux/x86_64" ;;
    MINGW*|MSYS*|CYGWIN*) PLATFORM="windows/x86_64" ;;
    *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
fi
case "$PLATFORM" in
  apple/*) EXT="dylib" ;; linux/*) EXT="so" ;; windows/*) EXT="dll" ;;
  *) echo "unknown platform: $PLATFORM" >&2; exit 1 ;;
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
    if s.get("hardware") or s.get("runtime"):
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

# The buildbot publishes no checksums: the hash of each zip is recorded the first time it is
# fetched in cores/SHA256SUMS (committed), and a zip that no longer matches is reported, so a
# core that was swapped or damaged since is noticed.
SUMS="$ROOT/cores/SHA256SUMS"; touch "$SUMS"
bad=0
for z in "$DEST"/*.zip; do
  [[ -f "$z" ]] || continue
  rel="$PLATFORM/$(basename "$z")"
  sum="$(shasum -a 256 "$z" | cut -d' ' -f1)"
  known="$(awk -v r="$rel" '$2==r {print $1}' "$SUMS")"
  if [[ -z "$known" ]]; then echo "$sum  $rel" >> "$SUMS"
  elif [[ "$known" != "$sum" ]]; then echo "CHANGED since it was recorded: $rel" >&2; bad=1; fi
done
sort -o "$SUMS" "$SUMS"
[[ $bad == 0 ]] || echo "Delete the zips named above and fetch again (and update cores/SHA256SUMS) if the change is wanted." >&2

echo "Offline cores in $DEST:"
ls -1sh "$DEST"
