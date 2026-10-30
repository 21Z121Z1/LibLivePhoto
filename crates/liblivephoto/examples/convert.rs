//! A standalone consumer of the public API. Publication policy belongs to callers.
use liblivephoto::{
    MotionPhoto, PairingPolicy, ParseOptions, PathInput, TargetProfile, UnmappedPolicy, WritePolicy,
};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let target = match args.next().as_deref() {
        Some("preserve") => TargetProfile::PreserveSource,
        Some("google-jpeg") => TargetProfile::AndroidJpeg,
        Some("google-heif") => TargetProfile::AndroidHeif,
        Some("google-avif") => TargetProfile::AndroidAvif,
        Some("apple") => TargetProfile::ApplePair,
        Some("samsung-jpeg-sef") => TargetProfile::SamsungJpegSef,
        _ => return Err("usage: convert PROFILE NEW_OUTPUT_DIR STILL [MOVIE] [--pair-id ID] [--sidecars|--allow-drop]".into()),
    };
    let output = PathBuf::from(args.next().ok_or("new output directory required")?);
    let still = PathBuf::from(args.next().ok_or("still input required")?);
    let mut movie = None;
    let mut policy = WritePolicy::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pair-id" => {
                policy.pairing = PairingPolicy::Set(args.next().ok_or("pair ID required")?)
            }
            "--sidecars" if policy.unmapped == UnmappedPolicy::Reject => {
                policy.unmapped = UnmappedPolicy::PreserveSidecars;
            }
            "--allow-drop" if policy.unmapped == UnmappedPolicy::Reject => {
                policy.unmapped = UnmappedPolicy::DropExplicitly;
            }
            value if !value.starts_with('-') && movie.is_none() => movie = Some(PathBuf::from(arg)),
            _ => return Err("unknown or conflicting argument".into()),
        }
    }
    let input = movie
        .as_ref()
        .map_or(PathInput::SingleFile(&still), |movie| {
            PathInput::ApplePair {
                still: &still,
                movie,
            }
        });
    let photo = MotionPhoto::open(input, ParseOptions::compatible())?;
    println!(
        "source format={:?} dialect={:?} time={:?}",
        photo.format(),
        photo.asset().dialect,
        photo.presentation_time()
    );
    for resource in photo.resources() {
        println!(
            "resource={} role={:?} mime={}",
            resource.id.0, resource.role, resource.mime
        );
        photo.extract(resource, std::io::sink())?;
    }
    let plan = photo.plan(target, policy);
    // Disclose blockers and dispositions before generating or publishing any bytes.
    println!(
        "compatibility={:#?}\nplan={:#?}",
        plan.compatibility(),
        plan.report()
    );
    let composed = plan.execute()?;

    // A new directory prevents overwriting an input or an existing conversion.
    fs::create_dir(&output)?;
    let publish = || -> Result<(), Box<dyn Error>> {
        for (index, file) in composed.files.iter().enumerate() {
            let extension = match file.mime.as_str() {
                "image/jpeg" => "jpg",
                "image/heic" => "heic",
                "video/quicktime" => "mov",
                "video/mp4" => "mp4",
                _ => "bin",
            };
            let name = match file.sidecar_for {
                Some(id) => format!("sidecar-{}-{index}.{extension}", id.0),
                None => format!("resource-{index}.{extension}"),
            };
            fs::write(output.join(name), &file.bytes)?;
        }
        fs::write(
            output.join("report.txt"),
            format!("{:#?}\n", composed.report),
        )?;
        Ok(())
    };
    if let Err(error) = publish() {
        // This directory was created by this invocation and contains only its output.
        fs::remove_dir_all(&output)?;
        return Err(error);
    }
    println!("validated format output written to {}", output.display());
    Ok(())
}
