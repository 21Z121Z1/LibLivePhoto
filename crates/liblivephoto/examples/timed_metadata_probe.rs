//! Test-only adapter for exact timed-metadata reference comparisons.
#[path = "../tests/common/mod.rs"]
mod common;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 3 && args[1] == "fixtures" {
        let root = std::path::Path::new(&args[2]);
        std::fs::create_dir(root)?;
        for (name, offset, edited) in [
            ("plain", 0, false),
            ("composition", 500, false),
            ("edits", 0, true),
            ("edited-composition", 500, true),
        ] {
            let raw = common::metadata_movie(
                b"it35",
                &[5, 0xb5, 0, 0x90, 0, 1],
                &[(&[0, 0], 1000), (&[0, 0x80, 3, 0xf7], 1000)],
                offset,
            );
            let bytes = if edited {
                common::metadata_edits(raw, 1000, &[(500, -1, 1, 0), (1000, 500, 1, 0)])
            } else {
                raw
            };
            std::fs::write(root.join(format!("{name}.mp4")), bytes)?;
        }
        return Ok(());
    }
    if args.len() != 3 {
        return Err("expected fixtures NEW_DIR or MOVIE TIMESTAMP_US".into());
    }
    let bytes = std::fs::read(&args[1])?;
    let time = liblivephoto::MediaTime::new(args[2].parse()?, 1_000_000).unwrap();
    let result = match liblivephoto::extract_agtm_from_video(&bytes, time) {
        Ok(samples) => {
            serde_json::json!({"success":true,"samples":samples.iter().map(|s| serde_json::json!({"index":s.sample_index,"time":s.presentation.value,"scale":s.presentation.timescale,"duration":s.duration.value,"bytes":s.bytes})).collect::<Vec<_>>()})
        }
        Err(error) => serde_json::json!({"success":false,"error":error.to_string()}),
    };
    println!("{result}");
    Ok(())
}
