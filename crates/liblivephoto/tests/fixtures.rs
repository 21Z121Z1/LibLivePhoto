mod common;
use liblivephoto::*;
use std::{fs, path::PathBuf};

#[test]
#[ignore = "requires the exact public fixture corpus; run scripts/fixtures.py first"]
fn heif_writer_declares_quicktime_for_a_caller_supplied_quicktime_movie() {
    let root = std::env::var_os("LIBLIVEPHOTO_FIXTURE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cache"));
    let original = fs::read(root.join("motion-photo/samsung/heif-ultrahdr-01.heic")).unwrap();
    let photo =
        MotionPhoto::parse(Input::SingleFile(&original), ParseOptions::compatible()).unwrap();
    // This is a synthetic association using a real still container, not a claim
    // that the source camera produced QuickTime or this presentation timestamp.
    let mut movie = common::movie(1000, 1);
    movie[8..12].copy_from_slice(b"qt  ");
    movie[16..20].copy_from_slice(b"qt  ");
    let parts = MotionPhoto::parse(
        Input::Parts {
            still: photo.still_bytes().unwrap(),
            movie: &movie,
            presentation: MediaTime::new(123_000, 1_000_000),
        },
        ParseOptions::compatible(),
    )
    .unwrap();
    let out = parts
        .plan(
            TargetProfile::AndroidHeif,
            WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                ..WritePolicy::default()
            },
        )
        .execute()
        .unwrap();
    let parsed = MotionPhoto::parse(
        Input::SingleFile(&out.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    let xmp = parsed
        .resources()
        .find(|r| r.provenance.evidence == "HEIF primary XMP item")
        .unwrap();
    let mut bytes = Vec::new();
    parsed.extract(xmp, &mut bytes).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("Item:Mime=\"video/quicktime\""));
    assert_eq!(parsed.motion_video().mime, "video/quicktime");
    assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
    assert_eq!(parsed.presentation_time(), parts.presentation_time());
}

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
        if path.contains("/vivo/") {
            let footer = inspect_camera_album_footer(&data).unwrap().unwrap();
            assert_eq!(footer.layout, CameraAlbumLayout::LengthDelimited43);
            assert_eq!(footer.prefix, CameraAlbumPrefix::VivoMediaExtInfo);
            assert_eq!(footer.version, Some(2202));
            assert_eq!(
                footer.image_time,
                Some(if path.ends_with("01.jpg") { 31 } else { 12 })
            );
            assert_eq!(
                footer.identifier_agreement,
                DeclaredIdentifierAgreement::Equal
            );
            let collection = MetadataEngine::new()
                .parse(&data, &HandlerOptions::default())
                .unwrap();
            let projected = collection
                .decoded
                .iter()
                .find(|d| d.provider == "camera_album")
                .unwrap();
            assert!(collection.verified_motion);
            assert_eq!(projected.value["pairingVerified"], false);
            let mut extracted = Vec::new();
            collection
                .extract_block(
                    projected.block_index.unwrap(),
                    &data,
                    &mut extracted,
                    131072,
                )
                .unwrap();
            assert!(
                extracted == data[footer.extent.lower_bound as usize..],
                "original footer bytes changed: {path}"
            );
            let movie = p.motion_video_bytes().unwrap();
            let boxes = liblivephoto_format::isobmff::parse_boxes(movie, 0..movie.len()).unwrap();
            for identifier in [b"vivoMediaEStream", b"vivoMediaExtInfo"] {
                assert!(
                    boxes.iter().any(|b| b.kind.as_bytes() == b"uuid"
                        && movie.get(b.data_start..b.data_start + 16) == Some(identifier)),
                    "missing original UUID association: {path}"
                );
            }
        }
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
            if path.contains("/vivo/") {
                let before = inspect_camera_album_footer(&data).unwrap().unwrap();
                let after = inspect_camera_album_footer(&output.files[0].bytes)
                    .unwrap()
                    .unwrap();
                assert!(
                    before.raw_json == after.raw_json,
                    "original footer JSON changed: {path}"
                );
                assert_eq!(before.image_time, after.image_time);
            }
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
