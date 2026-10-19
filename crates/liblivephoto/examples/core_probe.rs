//! A local differential-test adapter. This is not a complete publishing CLI.
use liblivephoto::*;
use serde_json::json;
use std::{fs::File, path::Path};
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: core_probe INPUT NEW_OUTPUT_DIRECTORY [ASSOCIATED_MOVIE]".into());
    }
    let bytes = std::fs::read(&args[0])?;
    let movie = args.get(2).map(std::fs::read).transpose()?;
    let input = movie
        .as_ref()
        .map_or(Input::SingleFile(&bytes), |movie| Input::Parts {
            still: &bytes,
            movie,
            presentation: MediaTime::new(1, 2),
        });
    let photo = match MotionPhoto::parse(input, ParseOptions::compatible()) {
        Ok(p) => p,
        Err(e) => {
            println!("{}", json!({"recognized":false,"error":e.to_string()}));
            return Ok(());
        }
    };
    let opened = MotionPhoto::open(
        args.get(2)
            .map_or(PathInput::SingleFile(Path::new(&args[0])), |movie| {
                PathInput::Parts {
                    still: Path::new(&args[0]),
                    movie: Path::new(movie),
                    presentation: MediaTime::new(1, 2),
                }
            }),
        ParseOptions::compatible(),
    )?;
    assert_eq!(photo.asset(), opened.asset());
    for (name, r) in [
        ("rust-still.bin", photo.still()),
        ("rust-video.bin", photo.motion_video()),
    ] {
        photo.extract(
            r,
            File::options()
                .write(true)
                .create_new(true)
                .open(Path::new(&args[1]).join(name))?,
        )?;
    }
    let time = photo
        .presentation_time()
        .unwrap_or(MediaTime::new(0, 1).unwrap());
    let (agtm,agtm_error) = match photo.agtm_at(time) {
        Ok(samples) => (samples.iter().map(|s| json!({"track_id":s.track_id,"sample_index":s.sample_index,"length":s.bytes.len(),"hdrReferenceWhite":s.header.hdr_reference_white_nits,"hasAdaptiveToneMap":s.header.has_adaptive_tone_map,"baselineHdrHeadroom":s.header.baseline_hdr_headroom_log2})).collect::<Vec<_>>(),None),
        Err(e) => (Vec::new(),Some(e.to_string())),
    };
    let target = match photo.still().mime.as_str() {
        "image/jpeg" => TargetProfile::AndroidJpeg,
        "image/avif" => TargetProfile::AndroidAvif,
        _ => TargetProfile::AndroidHeif,
    };
    let build = photo
        .plan(
            target,
            WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                ..WritePolicy::default()
            },
        )
        .execute();
    let (rebuilt, build_error) = match build {
        Ok(output) => {
            let name = match target {
                TargetProfile::AndroidJpeg => "rebuilt.jpg",
                TargetProfile::AndroidAvif => "rebuilt.avif",
                _ => "rebuilt.heic",
            };
            let path = Path::new(&args[1]).join(name);
            use std::io::Write;
            File::options()
                .write(true)
                .create_new(true)
                .open(&path)?
                .write_all(&output.files[0].bytes)?;
            (Some(path), None)
        }
        Err(e) => (None, Some(e.to_string())),
    };
    let descriptions = photo.sample_descriptions();
    println!(
        "{}",
        json!({"recognized":true,"path_matches_memory":true,"still_mime":photo.still().mime,"video_mime":photo.motion_video().mime,"video_length":photo.motion_video_bytes()?.len(),"time_us":photo.presentation_time().and_then(|t| t.rescale_exact(1_000_000)).map_or(-1,|t|t.value),"resources":photo.resources().count(),"tracks":photo.asset().containers.iter().map(|c|c.tracks.len()).sum::<usize>(),"xmp_length":photo.xmp_bytes()?.map(|p|p.len()),"agtm":agtm,"agtm_error":agtm_error,"rebuilt":rebuilt,"build_error":build_error,"media_facts_error":descriptions.as_ref().err().map(ToString::to_string)})
    );
    Ok(())
}
