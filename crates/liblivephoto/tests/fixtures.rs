use liblivephoto::*;
use std::{fs, path::PathBuf};

#[test]
#[ignore = "requires the exact public fixture corpus; run scripts/fixtures.py first"]
fn real_corpus_parses_validates_extracts_and_preserves_all_source_bytes() {
    let root = std::env::var_os("LIBLIVEPHOTO_FIXTURE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cache"));
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/manifest.json")).unwrap();
    let rows = manifest["fixtures"].as_array().unwrap();
    assert_eq!(rows.len(), 14);
    for row in rows {
        let path = row["path"].as_str().unwrap();
        let data = fs::read(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        let p = MotionPhoto::parse(Input::SingleFile(&data), ParseOptions::compatible())
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        p.validate().unwrap();
        for r in p.resources() {
            let mut bytes = Vec::new();
            p.extract(r, &mut bytes).unwrap();
            assert_eq!(
                bytes.len() as u64,
                r.extents.iter().map(|r| r.length()).sum::<u64>(),
                "{path}"
            );
        }
        let output = p
            .plan(TargetProfile::PreserveSource, WritePolicy::default())
            .execute()
            .unwrap();
        assert_eq!(output.files[0].bytes, data, "{path}");
        if path.contains("coloros16") {
            assert!(p
                .asset()
                .auxiliary
                .iter()
                .any(|r| r.role == ResourceRole::VendorAuxiliaryVideo));
            assert!(!p
                .asset()
                .auxiliary
                .iter()
                .any(|r| r.role == ResourceRole::GeometryDepthVideo));
        }
        if p.still().mime == "image/jpeg" {
            let policy = WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                ..WritePolicy::default()
            };
            let output = p
                .plan(TargetProfile::AndroidJpeg, policy)
                .execute()
                .unwrap_or_else(|e| panic!("Android JPEG {path}: {e}"));
            let parsed = MotionPhoto::parse(
                Input::SingleFile(&output.files[0].bytes),
                ParseOptions::strict(),
            )
            .unwrap();
            assert_eq!(
                parsed.motion_video_bytes().unwrap(),
                p.motion_video_bytes().unwrap(),
                "{path}"
            );
            assert_eq!(parsed.presentation_time(), p.presentation_time(), "{path}");
        } else {
            let policy = WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                ..WritePolicy::default()
            };
            let output = p
                .plan(TargetProfile::AndroidHeif, policy)
                .execute()
                .unwrap_or_else(|e| panic!("Android HEIF {path}: {e}"));
            let parsed = MotionPhoto::parse(
                Input::SingleFile(&output.files[0].bytes),
                ParseOptions::strict(),
            )
            .unwrap();
            assert_eq!(
                parsed.motion_video_bytes().unwrap(),
                p.motion_video_bytes().unwrap(),
                "{path}"
            );
            assert_eq!(parsed.presentation_time(), p.presentation_time(), "{path}");
            for r in p
                .asset()
                .auxiliary
                .iter()
                .filter(|r| r.role == ResourceRole::AuxiliaryImage)
            {
                let new = parsed
                    .asset()
                    .auxiliary
                    .iter()
                    .find(|n| n.vendor_role == r.vendor_role)
                    .unwrap();
                let mut a = Vec::new();
                let mut b = Vec::new();
                p.extract(r, &mut a).unwrap();
                parsed.extract(new, &mut b).unwrap();
                assert_eq!(a, b, "HEIF encoded item changed: {path}");
            }
        }
        if p.presentation_time().is_some() {
            let policy = WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                pairing: PairingPolicy::Set("FIXTURE-PAIR".into()),
            };
            let plan = p.plan(TargetProfile::ApplePair, policy);
            let output = plan
                .execute()
                .unwrap_or_else(|e| panic!("Apple pair {path}: {e}"));
            let pair = MotionPhoto::parse(
                Input::ApplePair {
                    still: &output.files[0].bytes,
                    movie: &output.files[1].bytes,
                },
                ParseOptions::strict(),
            )
            .unwrap();
            assert!(
                pair.presentation_time()
                    .unwrap()
                    .equivalent(p.presentation_time().unwrap()),
                "{path}"
            );
            assert_eq!(
                apple::media_payloads(pair.motion_video_bytes().unwrap()).unwrap(),
                apple::media_payloads(p.motion_video_bytes().unwrap()).unwrap(),
                "{path}"
            );
        }
        println!("fixture verified: {path}");
    }
}
