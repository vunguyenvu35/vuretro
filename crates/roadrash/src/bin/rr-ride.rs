//! Ride a bike along a course of a Road Rash disc.
//! Usage: rr-ride <ROADRASH.iso or its unpacked folder> [course: NAPA CITY HIWAY CANYN MEDLY]
//!        [--rider 0..14] [--music NAME] [--shots picture.png] [--sound-test sound.wav] [--busted-test picture.png]
//! Keys: Up or W to speed up, Down or S to brake, Left and Right or A and D to steer, Z and X
//! to hit a rider on the left or on the right, Esc to quit. After a while the police come
//! after you: stay fast, or the BUSTED video plays and the ride starts again.
//! With --shots nothing opens: a scripted ride is played and six pictures of it are written.
//! With --sound-test nothing opens either: four seconds of engine (the revs rising) and a
//! crash are mixed and written as a WAV file.
//! With --busted-test the BUSTED video is played to its middle without a window and a picture
//! of it, as the ride would show it, is written.
use std::path::Path;
use std::time::Instant;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::image::{sheet, write_png, Rgb};
use vuretro_roadrash::palette::{Order, Palette};
use vuretro_roadrash::ride::{
    collide, punch, render, step, step_rivals, Bike, Chase, Chased, Input, Look, Rival, TOP_SPEED,
};
use vuretro_roadrash::sound::{Audio, Mixer};
use vuretro_roadrash::{adpcm, avi, bob, cinepak, course, exe, horizon, hud, wav};

/// Take `--flag value` out of the arguments.
fn take(args: &mut Vec<String>, flag: &str) -> Result<Option<String>, String> {
    let Some(i) = args.iter().position(|a| a == flag) else {
        return Ok(None);
    };
    let value = args
        .get(i + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))?;
    args.drain(i..=i + 1);
    Ok(Some(value))
}

/// Six rivals ahead of the start, each in its own colour and lane, slower than the bike.
fn spawn_rivals(rider: usize) -> Vec<Rival> {
    (0..6)
        .map(|i| Rival {
            position: 12.0 + i as f32 * 14.0,
            lateral: [-250.0, 250.0, 0.0, -125.0, 125.0, 0.0][i],
            speed: 28.0 + i as f32 * 6.0,
            colour: (rider + 1 + i) % exe::RIDER_COLOURS,
        })
        .collect()
}

/// The sound of a movie as a clip (mono), when it is IMA ADPCM as on the disc.
fn movie_sound(movie: &avi::Movie) -> Option<wav::Clip> {
    let format = movie.sound.as_ref().filter(|f| f.tag == 0x0011)?;
    let mut samples = Vec::new();
    for chunk in &movie.sound_chunks {
        for block in chunk.chunks(format.block_bytes as usize) {
            samples.extend(adpcm::decode_block(block, format.channels as usize).ok()?);
        }
    }
    let channels = format.channels as usize;
    Some(wav::Clip {
        rate: format.rate as f32,
        samples: samples
            .chunks(channels)
            .map(|f| f.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32)
            .collect(),
    })
}

/// A picture of the video twice as big in the middle of 640x480.
fn letterbox(decoder: &cinepak::Decoder, width: usize, height: usize) -> Rgb {
    let mut pixels = vec![0u8; 640 * 480 * 3];
    let top = (480usize.saturating_sub(height * 2)) / 2;
    for y in 0..(height * 2).min(480) {
        for x in 0..(width * 2).min(640) {
            let from = ((y / 2) * width + x / 2) * 3;
            let to = ((top + y).min(479) * 640 + x) * 3;
            pixels[to..to + 3].copy_from_slice(&decoder.picture()[from..from + 3]);
        }
    }
    Rgb {
        width: 640,
        height: 480,
        pixels,
    }
}

fn to_buffer(picture: &Rgb, buffer: &mut [u32]) {
    for (pixel, rgb) in buffer.iter_mut().zip(picture.pixels.chunks_exact(3)) {
        *pixel = (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
    }
}

/// The video that plays when the police catch the rider.
struct Busted {
    movie: avi::Movie,
    decoder: cinepak::Decoder,
    shown: usize,
    started: Instant,
}

fn main() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let shots = take(&mut args, "--shots")?;
    let sound_test = take(&mut args, "--sound-test")?;
    let busted_test = take(&mut args, "--busted-test")?;
    let music_name = take(&mut args, "--music")?;
    let rider = match take(&mut args, "--rider")? {
        Some(n) => n
            .parse::<usize>()
            .map_err(|_| "--rider is a number from 0 to 14")?
            .min(exe::RIDER_COLOURS - 1),
        None => 0,
    };
    let disc_path = args
        .first()
        .ok_or("usage: rr-ride <ROADRASH.iso or folder> [course] [--rider n]")?;
    let name = args.get(1).map_or("NAPA", String::as_str).to_uppercase();

    let disc = Disc::open(Path::new(disc_path))?;
    let read = |p: String| disc.read(&p);
    let palette = Palette::parse(&read("DATA/PALETTE.RAW".into())?, Order::Rgbx)?;
    let riders = bob::read_frames(&read("DATA/BIKERS/HIBOB.DAT".into())?)?;
    let course = course::parse(&read(format!("DATA/COURSES/{name}.CRS"))?)?;
    let remaps = exe::rider_remaps(&read("ROADRASH.EXE".into())?)?;
    let dash = hud::panel(&read("IMAGES/BIKES/RAT/KHIDASH.BOB".into())?)?;
    let samples = course.route_samples();
    // the sounds: this bike's engine, and the effects 0 crash, 1 siren, 2 punch
    let engine = wav::parse(&read("AUDIO/EFFECTS/ENGRAT.WAV".into())?)?;
    let effects = ["CRASHLIT", "SIREN", "PUNCHLIG"]
        .iter()
        .map(|n| wav::parse(&read(format!("AUDIO/EFFECTS/{n}.WAV"))?))
        .collect::<Result<Vec<_>, String>>()?;
    // the music: a track of the disc (a WAV file of its own), one for each course unless
    // another is asked for with --music
    let track = music_name.unwrap_or_else(|| {
        match name.as_str() {
            "CITY" => "SG_KICKS",
            "HIWAY" => "SG_SUPER",
            "MEDLY" => "SG_OUTSH",
            "CANYN" => "HB_PASS",
            _ => "SG_RUSTY",
        }
        .to_string()
    });
    let music = read(format!("AUDIO/MUSIC/{track}.RRA"))
        .ok()
        .and_then(|f| wav::parse(&f).ok());
    let backdrop = horizon::load(&disc, &name, &palette)?;
    let headings = course::headings(&samples, vuretro_roadrash::road::Guess::default().turn);
    let mut look = Look {
        riders: &riders,
        palette: &palette,
        remaps: &remaps,
        dash: Some(&dash),
        horizon: Some(&backdrop),
        headings: &headings,
        police: None,
    };
    let mut rivals = spawn_rivals(rider);
    let dt = 1.0 / 30.0;

    if let Some(out) = busted_test {
        let movie = avi::parse(&read("VIDEO/BUSTED1.AVI".into())?)?;
        let mut decoder = cinepak::Decoder::new(movie.width as usize, movie.height as usize);
        for frame in &movie.frames[..movie.frames.len() / 2] {
            decoder.frame(frame)?;
        }
        let clip = movie_sound(&movie).ok_or("the video has no sound this reads")?;
        println!(
            "{} frames, sound {:.1} s at {} Hz",
            movie.frames.len(),
            clip.samples.len() as f32 / clip.rate,
            clip.rate
        );
        let picture = letterbox(&decoder, movie.width as usize, movie.height as usize);
        return write_png(&picture, Path::new(&out));
    }

    if let Some(out) = sound_test {
        match &music {
            Some(m) => println!(
                "music {track}: {:.0} s at {} Hz",
                m.samples.len() as f32 / m.rate,
                m.rate
            ),
            None => println!("music {track}: not read"),
        }
        let mut mixer = Mixer::new(engine, effects, 22_050.0);
        mixer.set_music(music.clone());
        let mut samples_out: Vec<i16> = Vec::new();
        for frame in 0..4 * 30 {
            mixer.revs = 3000.0 + 7000.0 * frame as f32 / 120.0;
            if frame == 60 {
                mixer.play(0);
            }
            let mut chunk = vec![0.0f32; 22_050 / 30];
            mixer.fill(&mut chunk);
            samples_out.extend(chunk.iter().map(|v| (v * 32767.0) as i16));
        }
        let rms = (samples_out.iter().map(|&v| (v as f64).powi(2)).sum::<f64>()
            / samples_out.len() as f64)
            .sqrt();
        println!("{} samples, rms {rms:.0}", samples_out.len());
        return std::fs::write(&out, adpcm::wav(&samples_out, 1, 22_050))
            .map_err(|e| e.to_string());
    }

    if let Some(out) = shots {
        // a scripted ride: full throttle, held near the middle of the road by steering back
        // towards it, with a swing on top so that the bike leans both ways; the police come
        // after six seconds
        let mut bike = Bike::start();
        let mut chase = Chase {
            elapsed: vuretro_roadrash::ride::POLICE_AFTER - 6.0,
            cop: None,
        };
        let mut views = Vec::new();
        for frame in 0..30 * 12 {
            let t = frame as f32 * dt;
            let input = Input {
                throttle: true,
                brake: false,
                steer: (-bike.lateral / 150.0 + 0.4 * (t * 0.9).sin()).clamp(-1.0, 1.0),
            };
            step(&mut bike, input, &samples, dt);
            step_rivals(&mut rivals, &samples, dt);
            collide(&mut bike, &rivals);
            look.police = match chase.step(&bike, dt) {
                Chased::Behind(gap) => Some(gap),
                Chased::Busted => {
                    chase = Chase::default();
                    None
                }
                Chased::Clear => None,
            };
            if frame % 60 == 59 {
                views.push(render(&samples, &bike, &rivals, &look, rider));
            }
        }
        return write_png(&sheet(&views, 3), Path::new(&out));
    }

    let (w, h) = (640, 480);
    let mut window = minifb::Window::new(
        &format!("Road Rash: {name}"),
        w,
        h,
        minifb::WindowOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    window.set_target_fps(30);
    let mut buffer = vec![0u32; w * h];
    // the sound, if there is a sound card to play it on
    let audio = match Audio::start(engine, effects) {
        Ok(a) => {
            if let Ok(mut m) = a.mixer.lock() {
                m.set_music(music);
            }
            Some(a)
        }
        Err(e) => {
            eprintln!("riding without sound: {e}");
            None
        }
    };
    let mut bike = Bike::start();
    let mut chase = Chase::default();
    let mut busted: Option<Busted> = None;
    let mut busts = 0usize;
    let mut touching = false;
    let mut punch_wait = 0.0f32;
    let down =
        |window: &minifb::Window, keys: &[minifb::Key]| keys.iter().any(|&k| window.is_key_down(k));
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        use minifb::Key::*;
        if let Some(b) = &mut busted {
            // the video of the arrest, until it ends; then everything starts again
            let at = (b.started.elapsed().as_micros() / b.movie.frame_us.max(1) as u128) as usize;
            while b.shown <= at && b.shown < b.movie.frames.len() {
                let _ = b.decoder.frame(&b.movie.frames[b.shown]);
                b.shown += 1;
            }
            let picture = letterbox(&b.decoder, b.movie.width as usize, b.movie.height as usize);
            to_buffer(&picture, &mut buffer);
            window
                .update_with_buffer(&buffer, w, h)
                .map_err(|e| e.to_string())?;
            if at >= b.movie.frames.len() {
                busted = None;
                bike = Bike::start();
                chase = Chase::default();
                rivals = spawn_rivals(rider);
                look.police = None;
            }
            continue;
        }

        let steer =
            down(&window, &[Right, D]) as i32 as f32 - down(&window, &[Left, A]) as i32 as f32;
        let input = Input {
            throttle: down(&window, &[Up, W]),
            brake: down(&window, &[Down, S]),
            steer,
        };
        step(&mut bike, input, &samples, dt);
        step_rivals(&mut rivals, &samples, dt);
        let hit = collide(&mut bike, &rivals);
        punch_wait = (punch_wait - dt).max(0.0);
        let side = down(&window, &[X]) as i32 as f32 - down(&window, &[Z]) as i32 as f32;
        let punched = side != 0.0 && punch_wait == 0.0 && {
            punch_wait = 0.4;
            punch(&bike, &mut rivals, side)
        };
        let chased = chase.step(&bike, dt);
        look.police = match chased {
            Chased::Behind(gap) => Some(gap),
            _ => None,
        };
        if let Some(Ok(mut m)) = audio.as_ref().map(|a| a.mixer.lock()) {
            // idle when stopped; otherwise the revs of the panel
            m.revs = if bike.speed < 0.5 {
                2500.0
            } else {
                hud::revs(bike.speed / TOP_SPEED * 280.0)
            };
            if hit && !touching {
                m.play(0);
            }
            if punched {
                m.play(2);
            }
            m.set_loop(match chased {
                Chased::Behind(gap) => Some((1, (1.0 - gap / 40.0).clamp(0.1, 1.0) * 0.5)),
                _ => None,
            });
        }
        touching = hit;
        if chased == Chased::Busted {
            // the arrest: its video and its sound
            busts += 1;
            let file = format!("VIDEO/BUSTED{}.AVI", (busts - 1) % 6 + 1);
            if let Some(movie) = read(file).ok().and_then(|f| avi::parse(&f).ok()) {
                if let Some(Ok(mut m)) = audio.as_ref().map(|a| a.mixer.lock()) {
                    m.revs = 0.0;
                    m.set_loop(None);
                    if let Some(clip) = movie_sound(&movie) {
                        let n = m.add_effect(clip);
                        m.play(n);
                    }
                }
                busted = Some(Busted {
                    decoder: cinepak::Decoder::new(movie.width as usize, movie.height as usize),
                    movie,
                    shown: 0,
                    started: Instant::now(),
                });
                continue;
            }
            chase = Chase::default(); // no video to show: the police just let go
        }
        to_buffer(&render(&samples, &bike, &rivals, &look, rider), &mut buffer);
        window
            .update_with_buffer(&buffer, w, h)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
