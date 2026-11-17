<p align="center"><img src="logo-mark-256.png" alt="vuretro" width="88"></p>

# vuretro brand

Concept: the **V** of the Vu family (Vutty, Vutidy) turned 90° into a **play** button. Two round-capped strokes in the same style as its siblings, split into magenta (top) and amber (bottom) that meet in a clean seam at the tip. The white rounded block sits in the mouth of the play symbol like a cartridge being inserted, or a single pixel: the one thing you came to run. Flat, two colours plus white, no glow on the mark; it reads at 1024 px and at 16 px.

Tagline: **PLAY · SAVE · RESUME**.

| File | Use |
| --- | --- |
| `logo-mark.svg` / `logo-mark-1024.png` / `logo-mark-256.png` | App tile, avatars, social preview |
| `logo-horizontal.svg` / `.png` | Dark backgrounds (README dark mode, slides) |
| `logo-horizontal-light.svg` / `.png` | Light backgrounds |
| `favicon.svg` / `favicon-64.png` / `favicon-16.png` | Mark tuned for small sizes (thicker strokes, larger block) |
| `appicon.png` | App icon source (1024 px with a 100 px safe margin) |
| `AppIcon.icns` | macOS icon, built from `appicon.png` with `sips` + `iconutil` |
| `render.mjs` | Renders the SVGs to PNG with Chromium (Playwright); set `CHROME` to a Chrome binary to reuse an installed browser |

Regenerate PNGs: `CHROME=".../Google Chrome" PLAYWRIGHT=/path/to/playwright/index.mjs node render.mjs '<jobs json>'` (job format at the top of `render.mjs`).

## Palette

| Token | Hex | Use |
| --- | --- | --- |
| Night top | `#1B1530` | tile background (top of the vertical gradient) |
| Night bottom | `#0B0916` | tile background (bottom) |
| Magenta | `#FF4F8B` | top arm, "Vu" on dark |
| Amber | `#FFB93B` | bottom arm |
| Mist | `#F4F1FF` | the block, "retro" on dark |
| Magenta on light | `#E0316F` | "Vu" on light backgrounds |
| Ink | `#150F26` | "retro" on light backgrounds |

## Rules

- Clear space of at least the width of the block around the mark.
- Minimum 24 px for the mark; use `favicon.svg` below 48 px.
- Do not swap the arm colours, add glow/gradients to the strokes, rotate, or outline the mark.
- Do not use the name or artwork of any console maker or emulator. vuretro never ships ROMs, BIOS files or third-party branding.
