use liblivephoto::{Input, MotionPhoto, ParseOptions};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = std::env::args().skip(1).collect::<Vec<_>>();
    if paths.is_empty() || paths.len() > 2 {
        return Err("usage: inspect STILL [MOVIE]".into());
    }
    let still = std::fs::read(&paths[0])?;
    let movie = paths.get(1).map(std::fs::read).transpose()?;
    let input = movie
        .as_ref()
        .map_or(Input::SingleFile(&still), |m| Input::ApplePair {
            still: &still,
            movie: m,
        });
    let photo = MotionPhoto::parse(input, ParseOptions::compatible())?;
    println!(
        "layout={:?} format={:?} dialect={:?} resources={} tracks={} time={:?}",
        photo.asset().layout,
        photo.format(),
        photo.asset().dialect,
        photo.resources().count(),
        photo
            .asset()
            .containers
            .iter()
            .map(|c| c.tracks.len())
            .sum::<usize>(),
        photo.presentation_time()
    );
    photo.validate()?;
    for r in photo.resources() {
        let mut data = Vec::new();
        photo.extract(r, &mut data)?;
        println!(
            "resource={} role={:?} bytes={} parent={:?}",
            r.id.0,
            r.role,
            data.len(),
            r.parent
        );
    }
    Ok(())
}
