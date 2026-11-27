//! Ride a bike along a course of a Road Rash disc.
//! Usage: rr-ride <ROADRASH.iso or its unpacked folder> [course: NAPA CITY HIWAY CANYN MEDLY] [--rider 0..14]
//!        [--shots picture.png]
//! Keys: Up or W to speed up, Down or S to brake, Left and Right or A and D to steer, Esc quits.
//! With --shots nothing opens: a scripted ride is played and six pictures of it are written.
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::image::{sheet, write_png, Rgb};
use vuretro_roadrash::palette::{Order, Palette};
use vuretro_roadrash::ride::{collide, render, step, step_rivals, Bike, Input, Look, Rival};
use vuretro_roadrash::{bob, course, exe};

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
    let look = Look {
        riders: &riders,
        palette: &palette,
        remaps: &remaps,
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
    let samples = course.route_samples();
    let dt = 1.0 / 30.0;

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
        collide(&mut bike, &rivals);
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
