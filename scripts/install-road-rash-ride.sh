#!/usr/bin/env bash
# Put the Road Rash ride (rr-ride, written in this repository) where vuretro finds it, so it
# shows up under Native games: <data dir>/native/road-rash-ride/. It plays from the disc image
# (.iso) of a Road Rash PC disc that you own, read in place; an unpacked folder (DATA, IMAGES,
# ROADRASH.EXE ...) works too. Vuretro does not include the game or its data.
# One entry is made for each course (Napa, City, Hiway, Canyn, Medly), or only for the one given.
# Usage: scripts/install-road-rash-ride.sh /path/to/ROADRASH.iso [COURSE: NAPA CITY HIWAY CANYN MEDLY]
set -euo pipefail
cd "$(dirname "$0")/.."

DISC="${1:?usage: $0 /path/to/ROADRASH.iso [COURSE]}"
COURSES="${2:-NAPA CITY HIWAY CANYN MEDLY}"
if [ -d "$DISC" ]; then
  [ -f "$DISC/ROADRASH.EXE" ] && [ -d "$DISC/DATA" ] || { echo "$DISC is not an unpacked Road Rash disc"; exit 1; }
  DISC="$(cd "$DISC" && pwd)"
else
  [ -f "$DISC" ] || { echo "$DISC is not a file or a folder"; exit 1; }
  DISC="$(cd "$(dirname "$DISC")" && pwd)/$(basename "$DISC")"
fi

case "$(uname -s)" in
  Darwin) DATA="$HOME/Library/Application Support/dev.vuvn.vuretro" ;;
  *) DATA="${XDG_DATA_HOME:-$HOME/.local/share}/vuretro" ;;
esac

cargo build --release -p vuretro-roadrash --bin rr-ride
for COURSE in $COURSES; do
  OUT="$DATA/native/road-rash-ride-$(echo "$COURSE" | tr 'A-Z' 'a-z')"
  mkdir -p "$OUT"
  cp target/release/rr-ride "$OUT/rr-ride"
  python3 - "$OUT/game.json" "$DISC" "$COURSE" <<'PY'
import json, sys
path, disc, course = sys.argv[1:4]
json.dump({"name": f"Road Rash ride ({course.title()})", "exe": "rr-ride", "args": [disc, course]}, open(path, "w"))
PY
  echo "Installed $COURSE in $OUT"
done
# the entry that an earlier version of this script made
rm -rf "$DATA/native/road-rash-ride"
echo "Open vuretro, Native games: the rides are offered there."
