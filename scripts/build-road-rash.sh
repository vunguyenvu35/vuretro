#!/usr/bin/env bash
# Build the native Road Rash port (github.com/arkits/road-rash) and put it where vuretro finds
# it: <data dir>/native/road-rash/rr-game. With a path to your own Road Rash PC disc image the
# game data is imported too; without one the game asks for the disc the first time it starts.
# Usage: scripts/build-road-rash.sh [/path/to/ROADRASH.iso]
# vuretro does not include the game or its data: the source is fetched from its author and
# the disc is yours.
set -euo pipefail

case "$(uname -s)" in
  Darwin) DATA="$HOME/Library/Application Support/dev.vuvn.vuretro" ;;
  *) DATA="${XDG_DATA_HOME:-$HOME/.local/share}/vuretro" ;;
esac
OUT="$DATA/native/road-rash"
SRC="$DATA/native/road-rash-src"
ISO="${1:-}"

command -v cmake >/dev/null || { echo "cmake is needed (macOS: brew install cmake sdl3)"; exit 1; }
command -v git >/dev/null || { echo "git is needed"; exit 1; }
mkdir -p "$OUT"

if [ -d "$SRC/.git" ]; then git -C "$SRC" pull --ff-only; else git clone --depth 1 https://github.com/arkits/road-rash "$SRC"; fi
cmake -S "$SRC" -B "$SRC/build" -DCMAKE_BUILD_TYPE=Release
cmake --build "$SRC/build" --target rr-game rr-import --parallel
cp "$SRC/build/rr-game" "$OUT/rr-game"

if [ -n "$ISO" ]; then
  "$SRC/build/rr-import" "$ISO" "$OUT/ROADRASH"
fi
# vuretro finds any folder under native/ that holds a game.json like this one
if [ -d "$OUT/ROADRASH" ]; then
  printf '{"name":"Road Rash","exe":"rr-game","args":["--assets","{dir}/ROADRASH"]}\n' > "$OUT/game.json"
else
  printf '{"name":"Road Rash","exe":"rr-game"}\n' > "$OUT/game.json"
fi
echo "Road Rash is ready in $OUT. Open vuretro, Native games: it is offered there."
