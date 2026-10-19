//! Generate only new, synthetic binding test inputs. No metadata authoring API.
#[path = "../tests/common/mod.rs"]
mod common;
use liblivephoto::*;
use std::{fs::File, io::Write, path::PathBuf};
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("new output directory required")?,
    );
    std::fs::create_dir(&directory)?;
    // Complete version-0 shared curve with two rules (the existing decoder's
    // independent golden vector). The primary video remains real encoded H.264.
    let mut agtm = vec![0, 0x40, 0x27, 0x10, 0x2f];
    for value in [
        35400u16, 14600, 8500, 39850, 6550, 2300, 15635, 16450, 20000,
    ] {
        agtm.extend(value.to_be_bytes());
    }
    agtm.push(0xe9);
    for value in [100u16, 2000, 65535] {
        agtm.extend(value.to_be_bytes());
    }
    agtm.push(8);
    for value in [
        0u16, 1000, 0, 10000, 18000, 27000, 5000, 1000, 2000, 18000, 9000,
    ] {
        agtm.extend(value.to_be_bytes());
    }
    let metadata = common::metadata_movie(b"it35", &[5, 0xb5, 0, 0x90, 0, 1], &[(&agtm, 2000)], 0);
    let movie = remux::import_tracks(
        include_bytes!("../../../web/public/demo/motion.mp4"),
        &metadata,
        &[2],
        remux::OptimizeOptions::default(),
    )?
    .bytes;
    let still = include_bytes!("../../../web/public/demo/cover.jpg");
    let photo = MotionPhoto::parse(
        Input::Parts {
            still,
            movie: &movie,
            presentation: MediaTime::new(1, 1),
        },
        ParseOptions::strict(),
    )?;
    let output = photo
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .execute()?;
    let collection = serde_json::to_vec(
        &MetadataEngine::new()
            .parse(&output.files[0].bytes, &HandlerOptions::default())?
            .json(),
    )?;
    let camera_album = [still.to_vec(), common::camera_album_envelope(b"vivo",
        br#"{"version":2202,"com.android.camera.imageTime":31,"com.android.camera.livephoto":"synthetic-camera-id-00000000","opaque":1.234567890123456789e100}"#,
        b"synthetic-camera-id-00000000", false)].concat();
    for (name, bytes) in [
        ("agtm-video.mp4", movie.as_slice()),
        ("agtm-motion.jpg", output.files[0].bytes.as_slice()),
        ("agtm-raw.bin", agtm.as_slice()),
        ("collection.json", collection.as_slice()),
        ("camera-album.jpg", camera_album.as_slice()),
    ] {
        File::options()
            .write(true)
            .create_new(true)
            .open(directory.join(name))?
            .write_all(bytes)?;
    }
    Ok(())
}
