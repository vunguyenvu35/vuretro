//! Draws the road of a course as the rider sees it, at six places along the route.
//! Usage: rr-view <file.CRS> <picture.png> [turn] [lift]
use vuretro_roadrash::image::{sheet, write_png};
use vuretro_roadrash::{course, road};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (Some(file), Some(out)) = (args.next(), args.next()) else {
        return Err("usage: rr-view <file.CRS> <picture.png> [turn] [lift]".into());
    };
    let mut guess = road::Guess::default();
    if let Some(t) = args.next() {
        guess.turn = t.parse().map_err(|_| "turn is a number")?;
    }
    if let Some(l) = args.next() {
        guess.lift = l.parse().map_err(|_| "lift is a number")?;
    }
    let course = course::parse(&std::fs::read(&file).map_err(|e| e.to_string())?)?;
    let samples = course.route_samples();
    let views: Vec<_> = (0..6)
        .map(|n| road::view(&samples, n * samples.len() / 6, guess))
        .collect();
    write_png(&sheet(&views, 3), std::path::Path::new(&out))
}
