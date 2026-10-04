use liblivephoto::*;
use std::{fs, path::PathBuf};
#[test]
#[ignore = "analysis-only external samples; do not redistribute"]
fn external_originals_validate_with_resource_and_time_preservation() {
    let root = std::env::var_os("LIBLIVEPHOTO_PUBLIC_SAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/private/public-samples")
        });
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../evidence/public-samples.json")).unwrap();
    let still = fs::read(root.join("Apple Live Photo/Apple iPhone 15 - photo.HEIC")).unwrap();
    let movie = fs::read(root.join("Apple Live Photo/Apple iPhone 15 - video.MOV")).unwrap();
    let apple = MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let preserved = apple
        .plan(TargetProfile::PreserveSource, WritePolicy::default())
        .execute()
        .unwrap();
    assert_eq!(preserved.files[0].bytes, still);
    assert_eq!(preserved.files[1].bytes, movie);
    assert_eq!(apple.presentation_time(), MediaTime::new(820, 600));
    let rekey = apple
        .plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing: PairingPolicy::Set("REKEY".into()),
                unmapped: UnmappedPolicy::PreserveSidecars,
            },
        )
        .execute()
        .unwrap();
    let new = MotionPhoto::parse(
        Input::ApplePair {
            still: &rekey.files[0].bytes,
            movie: &rekey.files[1].bytes,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(new.asset().pairing.identifier.as_deref(), Some("REKEY"));
    assert!(new
        .presentation_time()
        .unwrap()
        .equivalent(apple.presentation_time().unwrap()));
    for row in manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| !r["path"].as_str().unwrap().starts_with("Apple"))
    {
        let path = row["path"].as_str().unwrap();
        let data = fs::read(root.join(path)).unwrap();
        let p = MotionPhoto::parse(Input::SingleFile(&data), ParseOptions::compatible())
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        p.validate().unwrap();
        for r in p.resources() {
            p.extract(r, std::io::sink()).unwrap();
        }
        let preserved = p
            .plan(TargetProfile::PreserveSource, WritePolicy::default())
            .execute()
            .unwrap();
        assert_eq!(preserved.files[0].bytes, data);
        let target = if p.still().mime == "image/jpeg" {
            TargetProfile::AndroidJpeg
        } else {
            TargetProfile::AndroidHeif
        };
        let out = p
            .plan(
                target,
                WritePolicy {
                    unmapped: UnmappedPolicy::PreserveSidecars,
                    ..WritePolicy::default()
                },
            )
            .execute()
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        let new = MotionPhoto::parse(
            Input::SingleFile(&out.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(
            p.motion_video_bytes().unwrap(),
            new.motion_video_bytes().unwrap()
        );
        assert_eq!(p.presentation_time(), new.presentation_time());
        println!("external original verified: {path}");
    }
}
