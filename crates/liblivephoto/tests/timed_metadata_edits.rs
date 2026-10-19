mod common;
use liblivephoto::*;

const CONFIG: &[u8] = &[5, 0xb5, 0, 0x90, 0, 1];
const FIRST: &[u8] = &[0, 0];
const SECOND: &[u8] = &[0, 0x80, 3, 0xf7];
use common::metadata_edits as with_edits;

fn base() -> Vec<u8> {
    common::metadata_movie(b"it35", CONFIG, &[(FIRST, 1000), (SECOND, 1000)], 250)
}
fn at(movie: &[u8], value: i64) -> Vec<AgtmSample<'_>> {
    extract_agtm_from_video(movie, MediaTime::new(value, 1000).unwrap()).unwrap()
}

#[test]
fn empty_edits_and_both_clipped_boundaries_use_movie_time_and_original_bytes() {
    let movie = with_edits(base(), 1000, &[(250, -1, 1, 0), (1000, 500, 1, 0)]);
    let before = movie.clone();
    assert!(at(&movie, 0).is_empty());
    assert!(at(&movie, 249).is_empty());
    let first = at(&movie, 250);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].sample_index, 0);
    assert_eq!(first[0].presentation, MediaTime::new(250, 1000).unwrap());
    assert_eq!(first[0].duration, MediaTime::new(750, 1000).unwrap());
    assert_eq!(first[0].bytes, FIRST);
    assert_eq!(at(&movie, 999)[0].sample_index, 0);
    let second = at(&movie, 1000);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].sample_index, 1);
    assert_eq!(second[0].presentation, MediaTime::new(1000, 1000).unwrap());
    assert_eq!(second[0].duration, MediaTime::new(250, 1000).unwrap());
    assert_eq!(second[0].bytes, SECOND);
    assert!(at(&movie, 1250).is_empty());
    assert!(at(&movie, 4000).is_empty());
    let raw = metadata_packets(&movie, 2).unwrap();
    assert_eq!(raw.len(), 2);
    assert_eq!(raw[0].presentation, MediaTime::new(250, 1000).unwrap());
    assert_eq!(raw[0].duration, MediaTime::new(1000, 1000).unwrap());
    assert_eq!(movie, before);
}

#[test]
fn repeated_media_segments_reuse_source_index_without_rewriting_payload() {
    let movie = with_edits(
        base(),
        1000,
        &[(500, 1250, 1, 0), (250, -1, 1, 0), (500, 1250, 1, 0)],
    );
    let first = at(&movie, 0);
    let repeat = at(&movie, 750);
    for result in [&first, &repeat] {
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].sample_index, 1);
        assert_eq!(result[0].bytes, SECOND);
        assert_eq!(result[0].duration, MediaTime::new(500, 1000).unwrap());
    }
    assert_eq!(first[0].presentation.value, 0);
    assert_eq!(repeat[0].presentation.value, 750);
    assert!(at(&movie, 500).is_empty());
    assert!(at(&movie, 1250).is_empty());
    assert_eq!(metadata_packets(&movie, 2).unwrap().len(), 2);
}

#[test]
fn differing_timescales_are_exact_and_nonintegral_queries_or_edits_fail() {
    let movie = with_edits(base(), 2, &[(1, -1, 1, 0), (1, 250, 1, 0)]);
    assert_eq!(
        at(&movie, 500)[0].presentation,
        MediaTime::new(500, 1000).unwrap()
    );
    assert_eq!(
        at(&movie, 999)[0].duration,
        MediaTime::new(500, 1000).unwrap()
    );
    assert!(matches!(
        extract_agtm_from_video(&movie, MediaTime::new(1, 3).unwrap()),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
    let fractional = with_edits(base(), 3, &[(1, -1, 1, 0), (1, 250, 1, 0)]);
    assert!(matches!(
        extract_agtm_from_video(&fractional, MediaTime::new(0, 1).unwrap()),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
}

#[test]
fn unsupported_rates_and_negative_media_starts_fail_even_outside_the_edit() {
    for edit in [
        (1000, 0, 2, 0),
        (1000, 0, 0, 0),
        (1000, 0, -1, 0),
        (1000, 0, 1, 0x4000),
        (1000, -2, 1, 0),
    ] {
        let movie = with_edits(base(), 1000, &[edit]);
        assert!(matches!(
            extract_agtm_from_video(&movie, MediaTime::new(9, 1).unwrap()),
            Err(MotionPhotoReadError::UnsupportedLayout(_))
        ));
        // Raw packet inspection does not pretend to interpret the edit rate.
        assert_eq!(metadata_packets(&movie, 2).unwrap()[0].bytes, FIRST);
    }
}

#[test]
fn edit_duration_media_end_and_cumulative_overflow_fail_without_rounding() {
    for edits in [
        vec![(u64::MAX, 0, 1, 0)],
        vec![(1, i64::MAX, 1, 0)],
        vec![(i64::MAX as u64, -1, 1, 0), (1, -1, 1, 0)],
    ] {
        let movie = with_edits(base(), 1000, &edits);
        assert!(matches!(
            extract_agtm_from_video(&movie, MediaTime::new(0, 1).unwrap()),
            Err(MotionPhotoReadError::InvalidPresentation)
        ));
    }
}

#[test]
fn edit_count_does_not_multiply_sample_work_or_bypass_aggregate_sample_budget() {
    let movie = common::overlapping_metadata_movie(b"it35", CONFIG, FIRST, 4097);
    let edited = with_edits(movie, 1000, &vec![(1, 0, 1, 0); 4096]);
    assert!(matches!(
        extract_agtm_from_video(&edited, MediaTime::new(0, 1).unwrap()),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let movie = common::overlapping_metadata_movie(b"it35", CONFIG, FIRST, 250);
    let edited = with_edits(movie, 1000, &vec![(1, 0, 1, 0); 4096]);
    // 1,024,000 possible sample/edit pairs, but only one interval is selected.
    assert_eq!(at(&edited, 4095).len(), 250);
    assert!(at(&edited, 4096).is_empty());
}

#[test]
fn score_payload_times_are_inspectable_but_never_silently_retimed() {
    let movie = common::metadata_movie(
        b"mett",
        b"\0application/motionphoto-image-meta\0",
        &[(b"opaque score", 2000)],
        0,
    );
    let movie = with_edits(movie, 1000, &[(500, 1000, 1, 0)]);
    assert_eq!(
        metadata_packets(&movie, 2).unwrap()[0].bytes,
        b"opaque score"
    );
    assert!(matches!(
        extract_video_metadata(&movie, PhotoTimeExpectation::Unchecked),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
}

#[test]
fn fragmented_metadata_is_an_explicit_unsupported_layout() {
    let movie = [base(), common::boxed(b"moof", b"")].concat();
    assert!(matches!(
        metadata_packets(&movie, 2),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    assert!(matches!(
        extract_agtm_from_video(&movie, MediaTime::new(0, 1).unwrap()),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
}
