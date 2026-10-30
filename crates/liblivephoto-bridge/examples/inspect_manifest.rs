use liblivephoto::{Input, MediaTime};
use std::ops::ControlFlow;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let still = std::fs::read(args.first().ok_or("still file required")?)?;
    let movie = args.get(1).map(std::fs::read).transpose()?;
    let input = if let Some(ref movie) = movie {
        if args.get(2).is_some_and(|s| s == "--parts") {
            Input::Parts {
                still: &still,
                movie,
                presentation: MediaTime::new(13, 16),
            }
        } else {
            Input::ApplePair {
                still: &still,
                movie,
            }
        }
    } else {
        Input::SingleFile(&still)
    };
    let inspection = liblivephoto_bridge::inspect(input, |_| ControlFlow::Continue(()))?;
    println!("{}", inspection.manifest());
    Ok(())
}
