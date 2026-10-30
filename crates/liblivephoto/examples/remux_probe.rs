use liblivephoto::remux::*;
use serde_json::json;
use std::{fs::File, io::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(
            "remux_probe optimize|compact|remove|import INPUT NEW_OUTPUT [SOURCE ID...]".into(),
        );
    }
    let input = std::fs::read(&args[1])?;
    let output = PathBuf::from(&args[2]);
    let result = match args[0].as_str() {
        "optimize" => optimize(&input, OptimizeOptions::default())?,
        "compact" => optimize(
            &input,
            OptimizeOptions {
                compact_unreferenced_media: true,
            },
        )?,
        "remove" => {
            let ids = args[3..]
                .iter()
                .map(|s| s.parse())
                .collect::<Result<Vec<_>, _>>()?;
            remove_tracks(&input, &ids, OptimizeOptions::default())?
        }
        "import" => {
            let source = std::fs::read(args.get(3).ok_or("source required")?)?;
            let ids = args[4..]
                .iter()
                .map(|s| s.parse())
                .collect::<Result<Vec<_>, _>>()?;
            import_tracks(&input, &source, &ids, OptimizeOptions::default())?
        }
        _ => return Err("unknown operation".into()),
    };
    File::options()
        .write(true)
        .create_new(true)
        .open(&output)?
        .write_all(&result.bytes)?;
    for (i, sidecar) in result.sidecars.iter().enumerate() {
        File::options()
            .write(true)
            .create_new(true)
            .open(output.with_extension(format!("sidecar-{i}.bin")))?
            .write_all(&sidecar.bytes)?;
    }
    println!(
        "{}",
        json!({"mappings":result.report.track_mappings.iter().map(|m|json!({"source":m.source,"old":m.original_id,"new":m.output_id})).collect::<Vec<_>>(),"discardedMediaBytes":result.report.discarded_unreferenced_media_bytes,"retainedUnknownMediaBytes":result.report.retained_unreferenced_media_bytes,"discardedPaddingBytes":result.report.discarded_padding_bytes,"sidecars":result.sidecars.len()})
    );
    Ok(())
}
