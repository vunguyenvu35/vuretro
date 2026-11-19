//! `--headless-frames N`: run without a window or audio (CI, previews, debugging).

use crate::{core, cover, frontend};
use anyhow::Result;

pub fn run_headless(
    core: &core::Core,
    frames: u32,
    screenshot: Option<&std::path::Path>,
    cover: Option<&std::path::Path>,
    sram_path: &std::path::Path,
) -> Result<()> {
    let start = std::time::Instant::now();
    let mut best = cover::Best::default();
    let mut candidate: Option<Vec<u32>> = None;
    for i in 0..frames {
        core.run();
        // Boot screens are dull (and some games show memory as coloured tiles for a few seconds
        // while they start): look for the most colourful frame in the later part. A
        // frame counts once the next one has shown that it lasted (a glitch does not).
        if let Some(before) = candidate.take() {
            frontend::with(|fe| best.offer_stable(&before, &fe.frame, fe.width, fe.height));
        }
        if cover.is_some() && i >= frames * 2 / 5 && i % 30 == 0 {
            candidate = Some(frontend::with(|fe| fe.frame.clone()));
        }
    }
    if let Some(path) = cover {
        if best.save(path)? {
            log::info!("cover: {}", path.display());
        } else {
            log::warn!("cover: no colourful frame found");
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    log::info!(
        "ran {frames} frames in {elapsed:.2}s ({:.0} fps)",
        frames as f64 / elapsed.max(1e-9)
    );

    let (buf, w, h) = frontend::with(|fe| (fe.frame.clone(), fe.width, fe.height));
    anyhow::ensure!(w > 0 && h > 0, "core produced no video frame");
    if let Some(path) = screenshot {
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        ppm.extend(
            buf.iter()
                .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]),
        );
        std::fs::write(path, ppm)?;
        log::info!("screenshot: {}", path.display());
    }
    if let Some(state) = core.save_state() {
        anyhow::ensure!(core.load_state(&state), "save state round-trip failed");
        log::info!("save state round-trip ok ({} bytes)", state.len());
    }
    let n = core.save_sram(sram_path)?;
    if n > 0 {
        log::info!("saved {n} bytes of save RAM to {}", sram_path.display());
    }
    Ok(())
}
