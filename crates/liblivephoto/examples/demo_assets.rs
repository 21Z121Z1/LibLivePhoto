//! Package project-generated synthetic still/video without altering coded media.
use liblivephoto::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("demo directory")?);
    let still = std::fs::read(root.join("cover.jpg"))?;
    let movie = std::fs::read(root.join("motion.mp4"))?;
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: MediaTime::new(13, 16),
        },
        ParseOptions::strict(),
    )?;
    let motion = photo
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .execute()?;
    std::fs::write(root.join("motion-photo.jpg"), &motion.files[0].bytes)?;
    let policy = WritePolicy {
        pairing: PairingPolicy::Set("liblivephoto-synthetic-demo-20261019".into()),
        ..WritePolicy::default()
    };
    let apple = photo.plan(TargetProfile::ApplePair, policy).execute()?;
    for f in apple.files {
        let name = if f.mime.starts_with("image/") {
            "apple-still.jpg"
        } else {
            "apple-paired.mov"
        };
        std::fs::write(root.join(name), f.bytes)?;
    }
    Ok(())
}
