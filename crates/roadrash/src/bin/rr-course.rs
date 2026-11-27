//! Reads a course and prints what it holds; with a picture name it also draws the route.
//! Usage: rr-course <file.CRS> [picture.png] [turn]
use vuretro_roadrash::course;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: rr-course <file.CRS> [picture.png] [turn]")?;
    let picture = args.next();
    let turn: f64 = args
        .next()
        .map_or(Ok(0.0008), |t| t.parse())
        .map_err(|_| "turn is a number")?;
    let course = course::parse(&std::fs::read(&path).map_err(|e| e.to_string())?)?;
    let samples: usize = course.sections.iter().map(|s| s.bend.len()).sum();
    println!(
        "{} sections ({samples} samples), {} nodes",
        course.sections.len(),
        course.nodes.len()
    );
    let agree = course
        .sections
        .iter()
        .filter(|s| s.summed_elevation() == s.end_elevation as i32)
        .count();
    println!(
        "elevation at the end equals start + rises in {agree} of {} sections",
        course.sections.len()
    );
    let bends = course
        .sections
        .iter()
        .flat_map(|s| s.bend.iter().map(|&b| b as i32));
    let (lo, hi) = bends.fold((0, 0), |(l, h), b| (l.min(b), h.max(b)));
    println!("bend runs from {lo} to {hi}");
    let route = course.route();
    println!(
        "the route takes {} sections, {} samples",
        route.len(),
        route.iter().map(|r| r.1).sum::<usize>()
    );
    if let Some(png) = picture {
        let image = course::draw_route(&course, turn);
        vuretro_roadrash::image::write_png(&image, std::path::Path::new(&png))?;
        let total: i64 = route
            .iter()
            .map(|&(s, l)| {
                course.sections[s].bend[..l]
                    .iter()
                    .map(|&b| b as i64)
                    .sum::<i64>()
            })
            .sum();
        println!("sum of the bends along the route: {total}");
    }
    Ok(())
}
