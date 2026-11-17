# vuretro patch to minifb 0.27.0

Copy of the `minifb` crate with one change in `src/native/macosx/MacMiniFB.m`: the
`MTKView` no longer free-runs its own display-link draws (`paused = YES`,
`enableSetNeedsDisplay = NO`); `mfb_update_with_buffer` draws once with `[view draw]`.

Why: on macOS the free-running draws beat against the runner's own frame pacing and
sometimes queue two draws behind one `update`, stalling it for ~32 ms (visible stutter).
Measured on an Intel MacBook Pro: ~300-475 slow frames per 18 s before, 0-2 after.

Wired in through `[patch.crates-io]` in the workspace `Cargo.toml`.
