#!/usr/bin/env bash
# Smoke test of vuretro-runner against the fake test core (no ROM or display needed).
# Checks: video + screenshot, save-state round trip, SRAM persistence, core options.
# Usage: scripts/smoke.sh <path-to-test-core> [out-dir]
set -euo pipefail

CORE="${1:?usage: smoke.sh <test-core> [out-dir]}"
OUT="${2:-target/smoke}"
rm -rf "$OUT" && mkdir -p "$OUT"
echo test > "$OUT/game.test"

run() {
  cargo run -q -p vuretro-runner -- --core "$CORE" --rom "$OUT/game.test" \
    --system-dir "$OUT/system" --save-dir "$OUT/saves" "$@" 2>&1
}
dark_pixels() { # count 0x101010 pixels (the controllable square) in a binary PPM
  tail -c +$(( $(head -3 "$1" | wc -c) + 1 )) "$1" | od -An -v -tx1 | tr -d ' \n' | fold -w6 | grep -c '^101010$' || true
}

echo "1/4 video + screenshot + save state"
run --headless-frames 120 --screenshot "$OUT/shot.ppm" | grep -q "save state round-trip ok"
test -s "$OUT/shot.ppm"

echo "2/4 SRAM persists across runs"
rm -f "$OUT/saves/game.srm"
run --headless-frames 100 > /dev/null
run --headless-frames 100 > /dev/null
first_byte=$(od -An -tu1 -N1 "$OUT/saves/game.srm" | tr -d ' ')
[ "$first_byte" = "200" ] || { echo "expected SRAM[0]=200, got $first_byte"; exit 1; }

echo "3/4 core options template"
run --list-options | grep -q "vuretro_test_square = small"

echo "4/4 core options change behaviour"
echo "vuretro_test_square = large" > "$OUT/large.cfg"
run --headless-frames 5 --screenshot "$OUT/small.ppm" > /dev/null
run --options "$OUT/large.cfg" --headless-frames 5 --screenshot "$OUT/large.ppm" > /dev/null
small=$(dark_pixels "$OUT/small.ppm"); large=$(dark_pixels "$OUT/large.ppm")
[ "$large" -gt "$small" ] || { echo "option had no effect ($small vs $large)"; exit 1; }

echo "OK: smoke tests passed (square $small px -> $large px)"
