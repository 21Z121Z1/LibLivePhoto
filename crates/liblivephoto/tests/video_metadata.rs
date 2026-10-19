mod common;
use liblivephoto::*;
fn descr(tag: u8, p: &[u8]) -> Vec<u8> {
    let mut length = p.len();
    let mut encoded = vec![(length & 127) as u8];
    length >>= 7;
    while length != 0 {
        encoded.push((length & 127) as u8 | 128);
        length >>= 7;
    }
    encoded.reverse();
    [vec![tag], encoded, p.to_vec()].concat()
}
fn score(value: f32, time: i64) -> Vec<u8> {
    descr(
        0xc3,
        &[
            value.to_bits().to_be_bytes().to_vec(),
            time.to_be_bytes().to_vec(),
        ]
        .concat(),
    )
}
fn payload() -> Vec<u8> {
    let scores = descr(
        0xc1,
        &[
            7u32.to_be_bytes().to_vec(),
            score(0.8, 812500),
            descr(
                0xc2,
                &[
                    2u32.to_be_bytes().to_vec(),
                    score(0.3, 0),
                    score(0.9, 1000000),
                ]
                .concat(),
            ),
        ]
        .concat(),
    );
    let flags = descr(0xc4, &[descr(0xc5, &[0x81]), descr(0xc5, &[1])].concat());
    descr(0xc0, &[scores, flags, descr(0xd0, b"unknown")].concat())
}
#[test]
fn complete_existing_scores_and_stabilization_are_observed_without_authoring() {
    let raw = payload();
    let m = decode_video_metadata(&raw).unwrap();
    assert_eq!(m.raw_bytes, raw);
    assert_eq!(m.model_version, Some(7));
    assert_eq!(
        m.primary_score.as_ref().unwrap().encoded_score,
        0.8f32.to_bits()
    );
    assert_eq!(m.high_resolution_scores.as_ref().unwrap().len(), 2);
    assert_eq!(m.low_resolution_stabilized(), Some(true));
    assert_eq!(m.high_resolution_stabilized(), Some(false));
    assert_eq!(m.low_resolution_flags, Some(0x81));
    assert_eq!(m.unknown[0].payload, b"unknown");
    assert!(m
        .validate(PhotoTimeExpectation::Known(MediaTime::new(13, 16).unwrap()))
        .is_empty());
    assert!(m
        .validate(PhotoTimeExpectation::Known(MediaTime::new(1, 3).unwrap()))
        .contains(&VideoMetadataIssue::PhotoTimeMismatch));
    for prefix in 0..raw.len() {
        assert!(
            decode_video_metadata(&raw[..prefix]).is_err(),
            "accepted {prefix}"
        );
    }
}
#[test]
fn absent_fields_remain_unknown_and_unknown_extensions_stay_raw() {
    let empty = decode_video_metadata(&[0xc0, 0]).unwrap();
    assert_eq!(empty.primary_score, None);
    assert_eq!(empty.low_resolution_stabilized(), None);
    assert_eq!(
        empty.validate(PhotoTimeExpectation::Known(MediaTime::new(1, 3).unwrap())),
        [VideoMetadataIssue::MissingPrimaryTimestamp]
    );
    let mut raw = payload();
    let len = raw[1];
    raw.splice(1..2, [0x80, 0x80, 0x80, len]);
    raw.extend(b"future");
    let m = decode_video_metadata(&raw).unwrap();
    assert_eq!(m.trailing_bytes, b"future");
    assert_eq!(m.raw_bytes, raw);
    assert!(decode_video_metadata(&[0xc0, 0x80, 0x80, 0x80, 0x80]).is_err());
}
#[test]
fn tiny_unknown_descriptors_have_a_count_budget_as_well_as_a_byte_budget() {
    let maximum = descr(0xc0, &[0xd0, 0].repeat(16_383));
    assert_eq!(
        decode_video_metadata(&maximum).unwrap().unknown.len(),
        16_383
    );
    let dense = descr(0xc0, &[0xd0, 0].repeat(16_384));
    assert!(dense.len() < 40_000);
    assert!(matches!(
        decode_video_metadata(&dense),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn scoring_extraction_bounds_total_samples_records_and_copied_payload() {
    let config = b"\0application/motionphoto-image-meta\0";
    let empty = [0xc0, 0];
    let samples = vec![(empty.as_slice(), 1); 4097];
    let many_samples = common::metadata_movie(b"mett", config, &samples, 0);
    assert!(matches!(
        extract_video_metadata(&many_samples, PhotoTimeExpectation::Unchecked),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let dense = descr(0xc0, &[0xd0, 0].repeat(10_000));
    let samples = vec![(dense.as_slice(), 1); 7];
    let many_records = common::metadata_movie(b"mett", config, &samples, 0);
    assert!(matches!(
        extract_video_metadata(&many_records, PhotoTimeExpectation::Unchecked),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let mut large = empty.to_vec();
    large.resize(1024 * 1024, 0xaa); // Opaque trailing bytes are preserved too.
    let samples = vec![(large.as_slice(), 1); 17];
    let many_bytes = common::metadata_movie(b"mett", config, &samples, 0);
    assert!(matches!(
        extract_video_metadata(&many_bytes, PhotoTimeExpectation::Unchecked),
        Err(MotionPhotoReadError::InputLimit)
    ));
}
#[test]
fn semantic_validation_reports_unsorted_scores_nans_and_unknown_photo_time() {
    let raw = descr(
        0xc0,
        &descr(
            0xc1,
            &[
                0u32.to_be_bytes().to_vec(),
                score(f32::NAN, -2),
                descr(
                    0xc2,
                    &[2u32.to_be_bytes().to_vec(), score(2., 8), score(0.5, 3)].concat(),
                ),
            ]
            .concat(),
        ),
    );
    let m = decode_video_metadata(&raw).unwrap();
    let issues = m.validate(PhotoTimeExpectation::Unknown);
    assert!(issues.contains(&VideoMetadataIssue::UnsortedScores));
    assert!(issues.contains(&VideoMetadataIssue::ScoreOutOfRange { index: None }));
    assert!(issues.contains(&VideoMetadataIssue::ScoreOutOfRange { index: Some(0) }));
    assert!(issues.contains(&VideoMetadataIssue::InvalidTimestamp { index: None }));
    assert!(issues.contains(&VideoMetadataIssue::PhotoTimeMismatch));
}

#[test]
fn scoring_samples_use_declared_mime_and_actual_sample_association() {
    let raw = payload();
    let config = b"\0application/motionphoto-image-meta\0";
    let movie = common::metadata_movie(b"mett", config, &[(&raw, 2000)], 0);
    let packets = metadata_packets(&movie, 2).unwrap();
    assert_eq!(packets[0].bytes, raw);
    assert_eq!(packets[0].sample_index, 0);
    let decoded = extract_video_metadata(
        &movie,
        PhotoTimeExpectation::Known(MediaTime::new(13, 16).unwrap()),
    )
    .unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].metadata.model_version, Some(7));
    assert!(decoded[0].issues.is_empty());
    let other =
        common::metadata_movie(b"mett", b"\0application/octet-stream\0", &[(&raw, 2000)], 0);
    assert!(
        extract_video_metadata(&other, PhotoTimeExpectation::Unchecked)
            .unwrap()
            .is_empty()
    );
    let apple = include_bytes!("../../../web/public/demo/apple-paired.mov");
    assert!(
        extract_video_metadata(apple, PhotoTimeExpectation::Unchecked)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn composition_rejects_known_scoring_time_conflicts_but_preserve_copy_remains_exact() {
    let raw = payload();
    let movie = common::metadata_movie(
        b"mett",
        b"\0application/motionphoto-image-meta\0",
        &[(&raw, 2000)],
        0,
    );
    let still = vec![255, 216, 255, 217];
    let good = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: MediaTime::new(13, 16),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(good
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .can_execute());
    let conflict = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: MediaTime::new(1, 2),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(!conflict
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .can_execute());
    assert!(conflict
        .plan(TargetProfile::PreserveSource, WritePolicy::default())
        .can_execute());
}
