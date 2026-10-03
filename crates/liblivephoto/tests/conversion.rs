mod common;
use liblivephoto::*;

#[test]
fn source_preservation_is_byte_verified_for_every_resource() {
    let video = [common::movie(1000, 2), b"vendor-unknown".to_vec()].concat();
    let data = [common::jpeg(&common::xmp(video.len(), 123_000, "")), video].concat();
    let p = MotionPhoto::parse(Input::SingleFile(&data), ParseOptions::compatible()).unwrap();
    let output = p
        .plan(TargetProfile::PreserveSource, WritePolicy::default())
        .execute()
        .unwrap();
    assert_eq!(output.files[0].bytes, data);
    assert_eq!(
        output.report.evidence,
        vec![PreservationEvidence::SourceBytesEqual]
    );
    assert!(output
        .report
        .resources
        .iter()
        .all(|r| r.disposition == Disposition::Preserved));
}

#[test]
fn android_writer_retains_opaque_xmp_and_video_and_exact_time() {
    let video = common::movie(1000, 2);
    let xml = common::xmp(video.len(), 1_234_567, "").replace(
        "Camera:MotionPhoto=\"1\"",
        "Camera:FutureProperty=\"keep\" Camera:MotionPhoto=\"1\"",
    );
    let data = [common::jpeg(&xml), video].concat();
    let p = MotionPhoto::parse(Input::SingleFile(&data), ParseOptions::strict()).unwrap();
    let output = p
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .execute()
        .unwrap();
    let parsed = MotionPhoto::parse(
        Input::SingleFile(&output.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(parsed.presentation_time(), p.presentation_time());
    assert_eq!(
        parsed.motion_video_bytes().unwrap(),
        p.motion_video_bytes().unwrap()
    );
    assert!(String::from_utf8_lossy(parsed.still_bytes().unwrap())
        .contains("Vendor:opaque=\"retain &amp; verify\""));
    assert_eq!(parsed.asset().containers[1].tracks.len(), 2);
    assert!(String::from_utf8_lossy(parsed.still_bytes().unwrap())
        .contains("Camera:FutureProperty=\"keep\""));
}

#[test]
fn apple_pair_writer_preserves_media_payloads_and_exact_presentation() {
    let data = common::input(1_234_000, 1);
    let p = MotionPhoto::parse(Input::SingleFile(&data), ParseOptions::strict()).unwrap();
    let policy = WritePolicy {
        pairing: PairingPolicy::Set("TEST-PAIR".into()),
        ..WritePolicy::default()
    };
    let output = p.plan(TargetProfile::ApplePair, policy).execute().unwrap();
    let pair = MotionPhoto::parse(
        Input::ApplePair {
            still: &output.files[0].bytes,
            movie: &output.files[1].bytes,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(pair.asset().layout, PhysicalLayout::PairedFiles);
    assert_eq!(pair.asset().sources.len(), 2);
    assert!(pair
        .presentation_time()
        .unwrap()
        .equivalent(p.presentation_time().unwrap()));
    assert_eq!(
        apple::media_payloads(pair.motion_video_bytes().unwrap()).unwrap(),
        apple::media_payloads(p.motion_video_bytes().unwrap()).unwrap()
    );
    let copy = pair
        .plan(TargetProfile::ApplePair, WritePolicy::default())
        .execute()
        .unwrap();
    assert_eq!(copy.files[0].bytes, output.files[0].bytes);
    assert_eq!(copy.files[1].bytes, output.files[1].bytes);
}

#[test]
fn exact_writer_rescales_fractional_ticks_and_rejects_overflow() {
    let data = common::input(123, 1);
    let p = MotionPhoto::parse(Input::SingleFile(&data), ParseOptions::strict()).unwrap();
    let movie = apple::remux_movie(
        p.motion_video_bytes().unwrap(),
        "EXACT",
        p.presentation_time().unwrap(),
    )
    .unwrap();
    assert!(apple::read_presentation(&movie)
        .unwrap()
        .unwrap()
        .equivalent(p.presentation_time().unwrap()));
    assert_eq!(
        apple::media_payloads(&movie).unwrap(),
        apple::media_payloads(p.motion_video_bytes().unwrap()).unwrap()
    );
    assert!(apple::remux_movie(
        p.motion_video_bytes().unwrap(),
        "EXACT",
        MediaTime::new(i64::MAX, 1).unwrap()
    )
    .is_err());
    assert!(MediaTime::new(1, 3)
        .unwrap()
        .rescale_exact(1_000_000)
        .is_none());
    assert!(MediaTime::new(i64::MAX, 1)
        .unwrap()
        .rescale_exact(u32::MAX)
        .is_none());
    assert!(MediaTime::new(0, 0).is_none());
}

#[test]
fn presentation_at_or_beyond_movie_end_is_rejected_before_writing() {
    for value in [2_000_000, 2_000_001] {
        let input = common::input(value, 1);
        let photo = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
        assert!(photo
            .diagnostics()
            .contains(&Diagnostic::PresentationOutsideMovieHeader));
        let plan = photo.plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing: PairingPolicy::Set("BOUNDARY".into()),
                ..WritePolicy::default()
            },
        );
        assert!(!plan.can_execute());
        assert!(plan.execute().is_err());
        assert!(apple::remux_movie(
            &common::movie(1000, 1),
            "BOUNDARY",
            MediaTime::new(value, 1_000_000).unwrap()
        )
        .is_err());
    }
}

#[test]
fn unknown_suffix_requires_an_explicit_sidecar_or_drop_policy() {
    let v = [common::movie(1000, 1), b"unknown-vendor-data".to_vec()].concat();
    let input = [common::jpeg(&common::xmp(v.len(), 123_000, "")), v].concat();
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
    let plan = p.plan(TargetProfile::AndroidJpeg, WritePolicy::default());
    assert!(!plan.can_execute());
    assert!(plan.execute().is_err());
    let output = p
        .plan(
            TargetProfile::AndroidJpeg,
            WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                ..WritePolicy::default()
            },
        )
        .execute()
        .unwrap();
    assert_eq!(output.files.last().unwrap().bytes, b"unknown-vendor-data");
    assert!(output.files.last().unwrap().sidecar_for.is_some());
    let output = p
        .plan(
            TargetProfile::AndroidJpeg,
            WritePolicy {
                unmapped: UnmappedPolicy::DropExplicitly,
                ..WritePolicy::default()
            },
        )
        .execute()
        .unwrap();
    assert!(output
        .report
        .resources
        .iter()
        .any(|r| r.disposition == Disposition::Dropped));
}

#[test]
fn missing_pair_identity_or_time_and_transcode_targets_are_reported() {
    let input = common::input(-1, 1);
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    let plan = p.plan(TargetProfile::ApplePair, WritePolicy::default());
    assert!(!plan.can_execute());
    assert_eq!(plan.compatibility().blockers.len(), 2);
    let plan = p.plan(TargetProfile::AndroidHeif, WritePolicy::default());
    assert!(plan
        .report()
        .resources
        .iter()
        .any(|r| r.disposition == Disposition::TranscodeRequired));
    assert_eq!(
        plan.compatibility().target_consumer,
        SupportState::Unverified
    );
}

#[test]
fn near_capacity_vendor_exif_survives_in_explicit_sidecar_with_raw_orientation() {
    let movie = common::movie(1000, 1);
    let mut jpeg = common::jpeg(&common::xmp(movie.len(), 123_000, ""));
    let mut tiff = b"II*\0\x08\0\0\0".to_vec();
    tiff.extend_from_slice(&1u16.to_le_bytes());
    tiff.extend_from_slice(&[0x12, 1, 3, 0, 1, 0, 0, 0, 0, 0, 0, 0]); // Raw orientation 0.
    tiff.extend_from_slice(&[0; 4]);
    tiff.resize(65_400, 0x5a);
    let exif = [b"Exif\0\0".as_slice(), tiff.as_slice()].concat();
    let segment = [
        vec![255, 225],
        ((exif.len() + 2) as u16).to_be_bytes().to_vec(),
        exif.clone(),
    ]
    .concat();
    jpeg.splice(2..2, segment);
    let input = [jpeg, movie].concat();
    let photo = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    let pairing = PairingPolicy::Set("CAPACITY".into());
    assert!(!photo
        .plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing: pairing.clone(),
                ..WritePolicy::default()
            }
        )
        .can_execute());
    let output = photo
        .plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing,
                unmapped: UnmappedPolicy::PreserveSidecars,
            },
        )
        .execute()
        .unwrap();
    let resource = photo
        .resources()
        .find(|r| r.vendor_role.as_deref() == Some("JPEG EXIF"))
        .unwrap();
    assert_eq!(
        output
            .files
            .iter()
            .find(|f| f.sidecar_for == Some(resource.id))
            .unwrap()
            .bytes,
        exif
    );
    assert!(output
        .report
        .evidence
        .contains(&PreservationEvidence::EncodedStillPayloadsEqual));
    let rewritten = liblivephoto_format::jpeg_exif_tiff(&output.files[0].bytes)
        .unwrap()
        .unwrap();
    assert_eq!(&rewritten[18..20], &[0, 0]);
    let parsed = MotionPhoto::parse(
        Input::ApplePair {
            still: &output.files[0].bytes,
            movie: &output.files[1].bytes,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(parsed
        .presentation_time()
        .unwrap()
        .equivalent(photo.presentation_time().unwrap()));
}
