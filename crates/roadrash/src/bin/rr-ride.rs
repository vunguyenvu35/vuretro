//! Ride a bike along a course of a Road Rash disc.
//! Usage: rr-ride <ROADRASH.iso or its unpacked folder> [course: NAPA CITY HIWAY CANYN MEDLY] [--rider 0..14]
//!        [--shots picture.png] [--sound-test sound.wav]
//! Keys: Up or W to speed up, Down or S to brake, Left and Right or A and D to steer, Esc quits.
//! With --shots nothing opens: a scripted ride is played and six pictures of it are written.
//! With --sound-test nothing opens either: four seconds of engine (the revs rising) and a
//! crash are mixed and written as a WAV file.
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::image::{sheet, write_png, Rgb};
use vuretro_roadrash::palette::{Order, Palette};
use vuretro_roadrash::ride::{collide, render, step, step_rivals, Bike, Input, Look, Rival};
use vuretro_roadrash::sound::{Audio, Mixer};
use vuretro_roadrash::{bob, course, exe, horizon, hud, wav};

fn main() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let shots = args
        .iter()
        .position(|a| a == "--shots")
        .map(|i| {
            let name = args
                .get(i + 1)
                .cloned()
                .ok_or("--shots needs a file name")?;
            args.drain(i..=i + 1);
            Ok::<_, String>(name)
        })
        .transpose()?;
    let sound_test = args
        .iter()
        .position(|a| a == "--sound-test")
        .map(|i| {
            let name = args
                .get(i + 1)
                .cloned()
                .ok_or("--sound-test needs a file name")?;
            args.drain(i..=i + 1);
            Ok::<_, String>(name)
        })
        .transpose()?;
    let rider = args
        .iter()
        .position(|a| a == "--rider")
        .map(|i| {
            let n = args.get(i + 1).ok_or("--rider needs a number")?;
            let n: usize = n.parse().map_err(|_| "--rider is a number from 0 to 14")?;
            args.drain(i..=i + 1);
            Ok::<_, String>(n.min(exe::RIDER_COLOURS - 1))
        })
        .transpose()?
        .unwrap_or(0);
    let disc = args
        .first()
        .ok_or("usage: rr-ride <ROADRASH.iso or folder> [course] [--shots picture.png]")?;
    let name = args.get(1).map_or("NAPA", String::as_str).to_uppercase();
    let disc = Path::new(disc);

    let disc = Disc::open(disc)?;
    let read = |p: String| disc.read(&p);
    let palette = Palette::parse(&read("DATA/PALETTE.RAW".into())?, Order::Rgbx)?;
    let riders = bob::read_frames(&read("DATA/BIKERS/HIBOB.DAT".into())?)?;
    let course = course::parse(&read(format!("DATA/COURSES/{name}.CRS"))?)?;
    let remaps = exe::rider_remaps(&read("ROADRASH.EXE".into())?)?;
    let dash = hud::panel(&read("IMAGES/BIKES/RAT/KHIDASH.BOB".into())?)?;
    let samples = course.route_samples();
    // the sounds: this bike's engine, and the knock of a crash
    let engine = wav::parse(&read("AUDIO/EFFECTS/ENGRAT.WAV".into())?)?;
    let effects = vec![wav::parse(&read("AUDIO/EFFECTS/CRASHLIT.WAV".into())?)?];
    let backdrop = horizon::load(&disc, &name, &palette)?;
    let headings = course::headings(&samples, vuretro_roadrash::road::Guess::default().turn);
    let look = Look {
        riders: &riders,
        palette: &palette,
        remaps: &remaps,
        dash: Some(&dash),
        horizon: Some(&backdrop),
        headings: &headings,
    };
    // six rivals ahead of the start, each in its own colour and lane, slower than the bike
    let mut rivals: Vec<Rival> = (0..6)
        .map(|i| Rival {
            position: 12.0 + i as f32 * 14.0,
            lateral: [-250.0, 250.0, 0.0, -125.0, 125.0, 0.0][i],
            speed: 28.0 + i as f32 * 6.0,
            colour: (rider + 1 + i) % exe::RIDER_COLOURS,
        })
        .collect();
    let dt = 1.0 / 30.0;

    if let Some(out) = sound_test {
        let mut mixer = Mixer::new(engine, effects, 22_050.0);
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
        return std::fs::write(&out, vuretro_roadrash::adpcm::wav(&samples_out, 1, 22_050))
            .map_err(|e| e.to_string());
    }

    if let Some(out) = shots {
        // a scripted ride: full throttle, held near the middle of the road by steering back
        // towards it, with a swing on top so that the bike leans both ways
        let mut bike = Bike::start();
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
    let mut bike = Bike::start();
    let mut buffer = vec![0u32; w * h];
    // the sound, if there is a sound card to play it on
    let audio = match Audio::start(engine, effects) {
        Ok(a) => Some(a),
        Err(e) => {
            eprintln!("riding without sound: {e}");
            None
        }
    };
    let mut touching = false;
    let down =
        |window: &minifb::Window, keys: &[minifb::Key]| keys.iter().any(|&k| window.is_key_down(k));
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        use minifb::Key::*;
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
        if let Some(Ok(mut m)) = audio.as_ref().map(|a| a.mixer.lock()) {
            // idle when stopped; otherwise the revs of the panel
            m.revs = if bike.speed < 0.5 {
                2500.0
            } else {
                hud::revs(bike.speed / vuretro_roadrash::ride::TOP_SPEED * 280.0)
            };
            if hit && !touching {
                m.play(0);
            }
        }
        touching = hit;
        let picture: Rgb = render(&samples, &bike, &rivals, &look, rider);
        for (pixel, rgb) in buffer.iter_mut().zip(picture.pixels.chunks_exact(3)) {
            *pixel = (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
        }
        window
            .update_with_buffer(&buffer, w, h)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
