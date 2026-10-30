mod common;
use liblivephoto::*;
use liblivephoto_format::isobmff::parse_boxes;

fn descriptor(tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut len = payload.len();
    let mut encoded = vec![(len & 127) as u8];
    len >>= 7;
    while len != 0 {
        encoded.push((len & 127) as u8 | 128);
        len >>= 7;
    }
    encoded.reverse();
    [vec![tag], encoded, payload.to_vec()].concat()
}
fn frame(bits: u32, time: i64) -> Vec<u8> {
    descriptor(
        0xc3,
        &[bits.to_be_bytes().to_vec(), time.to_be_bytes().to_vec()].concat(),
    )
}
fn scores(primary: i64, high: &[i64]) -> VideoMetadata {
    let track = descriptor(
        0xc2,
        &[
            (high.len() as u32).to_be_bytes().to_vec(),
            high.iter()
                .flat_map(|t| frame(0.5f32.to_bits(), *t))
                .collect(),
        ]
        .concat(),
    );
    let raw = descriptor(
        0xc0,
        &[
            descriptor(
                0xc1,
                &[
                    7u32.to_be_bytes().to_vec(),
                    frame(0x7fc01234, primary),
                    track,
                ]
                .concat(),
            ),
            descriptor(0xd0, b"opaque"),
        ]
        .concat(),
    );
    decode_video_metadata(&raw).unwrap()
}
fn tracks() -> ScoringMediaTracks {
    ScoringMediaTracks {
        primary: 1,
        high_resolution: Some(1),
    }
}
fn edit(movie: Vec<u8>, scale: u32, edits: &[(u64, i64, i16, i16)]) -> Vec<u8> {
    common::movie_track_edits(movie, 0, scale, edits)
}
fn ctts(movie: &mut Vec<u8>, count: u32, offset: i32) {
    let mut range = 0..movie.len();
    let mut parents = Vec::new();
    for kind in [b"moov", b"trak", b"mdia", b"minf", b"stbl"] {
        let b = parse_boxes(movie, range)
            .unwrap()
            .into_iter()
            .find(|b| b.kind.as_bytes() == kind)
            .unwrap();
        parents.push(b.box_start);
        range = b.payload_range();
    }
    let mut table = common::full(
        b"ctts",
        &[
            1u32.to_be_bytes(),
            count.to_be_bytes(),
            offset.to_be_bytes(),
        ]
        .concat(),
    );
    table[8] = 1;
    movie.splice(range.end..range.end, table.iter().copied());
    for pos in parents {
        let old = u32::from_be_bytes(movie[pos..pos + 4].try_into().unwrap());
        movie[pos..pos + 4].copy_from_slice(&(old + table.len() as u32).to_be_bytes());
    }
}

#[test]
fn empty_trim_and_repeat_project_each_source_field_without_changing_bytes() {
    let movie = edit(
        common::many_frame_movie(4, 1),
        1000,
        &[(1, -1, 1, 0), (2, 1, 1, 0), (1, -1, 1, 0), (2, 1, 1, 0)],
    );
    let before = movie.clone();
    let metadata = scores(1000, &[0, 1000, 2000, 3000]);
    let raw = metadata.raw_bytes.clone();
    let mapped = project_scoring_media_times(&movie, &metadata, tracks(), 6).unwrap();
    let primary = mapped.primary.as_ref().unwrap();
    assert_eq!(
        primary
            .movie_times
            .iter()
            .map(|t| t.value)
            .collect::<Vec<_>>(),
        [1000, 4000]
    );
    assert_eq!(primary.source.encoded_score, 0x7fc01234);
    assert!(primary.source.score.is_nan());
    assert!(std::ptr::eq(
        primary.source,
        metadata.primary_score.as_ref().unwrap()
    ));
    let high = mapped.high_resolution.unwrap();
    assert_eq!(high[0].disposition, ScoreTimeDisposition::Trimmed);
    assert_eq!(high[3].disposition, ScoreTimeDisposition::Trimmed);
    assert_eq!(
        high[2]
            .movie_times
            .iter()
            .map(|t| t.value)
            .collect::<Vec<_>>(),
        [2000, 5000]
    );
    assert_eq!(metadata.raw_bytes, raw);
    assert_eq!(metadata.unknown[0].payload, b"opaque");
    assert_eq!(movie, before);
    assert!(matches!(
        project_scoring_media_times(&movie, &metadata, tracks(), 5),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn payload_cts_is_matched_before_edits_without_adding_ctts_twice() {
    let mut movie = common::many_frame_movie(2, 1);
    ctts(&mut movie, 2, 1);
    let movie = edit(movie, 1000, &[(1, -1, 1, 0), (2, 1, 1, 0)]);
    let metadata = scores(1000, &[2000]);
    let mapped = project_scoring_media_times(&movie, &metadata, tracks(), 2).unwrap();
    assert_eq!(
        mapped.primary.unwrap().movie_times,
        [MediaTime::new(1000, 1000000).unwrap()]
    );
    assert_eq!(
        mapped.high_resolution.unwrap()[0].movie_times[0].value,
        2000
    );
    assert!(matches!(
        project_scoring_media_times(&movie, &scores(0, &[]), tracks(), 2),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
}

#[test]
fn raw_google_scoring_uses_declared_global_time_even_with_metadata_edits() {
    let source = scores(812500, &[]);
    let movie = common::metadata_movie(
        b"mett",
        b"\0application/motionphoto-image-meta\0",
        &[(&source.raw_bytes, 2000)],
        250,
    );
    // The metadata track is shifted/trimmed independently of its embedded frame time.
    let movie = common::metadata_edits(movie, 1000, &[(500, -1, 1, 0), (500, 1000, 1, 0)]);
    let actual = extract_video_metadata(
        &movie,
        PhotoTimeExpectation::Known(MediaTime::new(13, 16).unwrap()),
    )
    .unwrap();
    assert_eq!(actual[0].metadata.raw_bytes, source.raw_bytes);
    assert_eq!(
        actual[0]
            .metadata
            .primary_score
            .as_ref()
            .unwrap()
            .timestamp_us,
        812500
    );
    assert!(!actual[0]
        .issues
        .contains(&VideoMetadataIssue::PhotoTimeMismatch));
    assert!(actual[0]
        .issues
        .contains(&VideoMetadataIssue::ScoreOutOfRange { index: None }));
    assert_eq!(
        metadata_packets(&movie, 2).unwrap()[0].presentation.value,
        250
    );
}

#[test]
fn unspecified_absent_and_missing_association_are_explicit() {
    let movie = common::many_frame_movie(1, 2);
    let metadata = scores(-1, &[-1]);
    let mapped = project_scoring_media_times(&movie, &metadata, tracks(), 0).unwrap();
    assert_eq!(
        mapped.primary.unwrap().disposition,
        ScoreTimeDisposition::Unspecified
    );
    assert!(mapped.high_resolution.unwrap()[0].movie_times.is_empty());
    let empty = decode_video_metadata(&[0xc0, 0]).unwrap();
    let mapped = project_scoring_media_times(&movie, &empty, tracks(), 0).unwrap();
    assert!(mapped.primary.is_none());
    assert!(mapped.high_resolution.is_none());
    assert!(matches!(
        project_scoring_media_times(
            &movie,
            &scores(0, &[0]),
            ScoringMediaTracks {
                primary: 1,
                high_resolution: None
            },
            2
        ),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    assert!(matches!(
        project_scoring_media_times(
            &movie,
            &scores(0, &[]),
            ScoringMediaTracks {
                primary: 99,
                high_resolution: None
            },
            2
        ),
        Err(MotionPhotoReadError::InvalidResource)
    ));
}

#[test]
fn two_score_domains_use_the_explicit_track_ids() {
    let movie = edit(
        common::many_frame_movie(2, 2),
        1000,
        &[(1, -1, 1, 0), (2, 0, 1, 0)],
    );
    let metadata = scores(1000, &[1000]);
    let mapped = project_scoring_media_times(
        &movie,
        &metadata,
        ScoringMediaTracks {
            primary: 1,
            high_resolution: Some(2),
        },
        2,
    )
    .unwrap();
    assert_eq!(mapped.primary.unwrap().movie_times[0].value, 2000);
    assert_eq!(
        mapped.high_resolution.unwrap()[0].movie_times[0].value,
        1000
    );
}

#[test]
fn non_unit_rates_invalid_times_and_overflow_are_rejected() {
    let metadata = scores(0, &[]);
    for (integer, fraction) in [(2, 0), (0, 0), (-1, 0), (1, 0x4000)] {
        let movie = edit(
            common::many_frame_movie(1, 1),
            1000,
            &[(1, 0, integer, fraction)],
        );
        assert!(matches!(
            project_scoring_media_times(&movie, &metadata, tracks(), 3),
            Err(MotionPhotoReadError::UnsupportedLayout(_))
        ));
    }
    let movie = edit(common::many_frame_movie(1, 1), 1000, &[(1, -2, 1, 0)]);
    assert!(matches!(
        project_scoring_media_times(&movie, &metadata, tracks(), 3),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    for edits in [
        vec![(u64::MAX, 0, 1, 0)],
        vec![(1, i64::MAX, 1, 0)],
        vec![(i64::MAX as u64, -1, 1, 0), (1, -1, 1, 0)],
    ] {
        let movie = edit(common::many_frame_movie(1, 1), 1000, &edits);
        assert!(matches!(
            project_scoring_media_times(&movie, &metadata, tracks(), 3),
            Err(MotionPhotoReadError::InvalidPresentation)
        ));
    }
    let movie = common::many_frame_movie(1, 1);
    assert!(matches!(
        project_scoring_media_times(&movie, &scores(-2, &[]), tracks(), 1),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
    assert!(matches!(
        project_scoring_media_times(&movie, &scores(i64::MAX, &[]), tracks(), 1),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
}

#[test]
fn fractional_movie_microseconds_are_never_rounded() {
    let movie = edit(
        common::many_frame_movie(1, 1),
        3,
        &[(1, -1, 1, 0), (1, 0, 1, 0)],
    );
    assert!(matches!(
        project_scoring_media_times(&movie, &scores(0, &[]), tracks(), 1),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
    let movie = common::many_frame_movie(2, 1);
    // 1us is not an integral tick of this track's 1000Hz media clock.
    assert!(matches!(
        project_scoring_media_times(&movie, &scores(1, &[]), tracks(), 1),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
}

#[test]
fn repeated_payload_fields_cannot_multiply_the_output_budget() {
    let movie = edit(
        common::many_frame_movie(1, 1),
        1000,
        &vec![(1, 0, 1, 0); 257],
    );
    let metadata = scores(0, &vec![0; 256]);
    assert!(matches!(
        project_scoring_media_times(&movie, &metadata, tracks(), usize::MAX),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let large = common::many_frame_movie(100001, 1);
    assert!(matches!(
        project_scoring_media_times(&large, &scores(0, &[]), tracks(), 1),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let movie = edit(
        common::many_frame_movie(1, 1),
        1000,
        &vec![(1, 0, 1, 0); 4096],
    );
    let metadata = scores(0, &[]);
    assert_eq!(
        project_scoring_media_times(&movie, &metadata, tracks(), 4096)
            .unwrap()
            .primary
            .unwrap()
            .movie_times
            .len(),
        4096
    );
}

#[test]
fn fragmented_video_projection_is_not_guessed_from_classic_tables() {
    let movie = [common::many_frame_movie(1, 1), common::boxed(b"moof", b"")].concat();
    assert!(matches!(
        project_scoring_media_times(&movie, &scores(0, &[]), tracks(), 1),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
}
