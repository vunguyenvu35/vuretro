//! Decodes a Road Rash AVI: writes a few frames as PNG and the sound as a WAV file.
//! Usage: rr-video <file.AVI> <output folder>
use std::path::Path;
use vuretro_roadrash::image::{write_png, Rgb};
use vuretro_roadrash::{adpcm, avi, cinepak};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (Some(file), Some(out)) = (args.next(), args.next()) else {
        return Err("usage: rr-video <file.AVI> <output folder>".into());
    };
    let out = Path::new(&out);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let movie = avi::parse(&std::fs::read(&file).map_err(|e| e.to_string())?)?;
    println!(
        "{}x{}, {} frames at {:.1} fps, sound: {:?}",
        movie.width,
        movie.height,
        movie.frames.len(),
        1_000_000.0 / movie.frame_us.max(1) as f64,
        movie.sound
    );
    let mut decoder = cinepak::Decoder::new(movie.width as usize, movie.height as usize);
    let keep = [
        0,
        movie.frames.len() / 4,
        movie.frames.len() / 2,
        movie.frames.len() - 1,
    ];
    for (n, frame) in movie.frames.iter().enumerate() {
        decoder
            .frame(frame)
            .map_err(|e| format!("frame {n}: {e}"))?;
        if keep.contains(&n) {
            let picture = Rgb {
                width: movie.width,
                height: movie.height,
                pixels: decoder.picture().to_vec(),
            };
            write_png(&picture, &out.join(format!("frame{n:04}.png")))?;
        }
    }
    if let Some(sound) = &movie.sound {
        if sound.tag == 0x0011 {
            let mut samples = Vec::new();
            for block in &movie.sound_chunks {
                for part in block.chunks(sound.block_bytes as usize) {
                    samples.extend(adpcm::decode_block(part, sound.channels as usize)?);
                }
            }
            std::fs::write(
                out.join("sound.wav"),
                adpcm::wav(&samples, sound.channels, sound.rate),
            )
            .map_err(|e| e.to_string())?;
            println!("sound: {} samples", samples.len());
        }
    }
    Ok(())
}
