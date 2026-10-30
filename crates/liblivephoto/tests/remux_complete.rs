mod common;
use liblivephoto::{remux::*, *};
use liblivephoto_format::isobmff::parse_boxes;
#[test]
fn fast_start_promotes_offsets_preserves_samples_and_reports_compaction() {
    let movie = common::movie(1000, 2);
    let optimized = optimize(&movie, OptimizeOptions::default()).unwrap();
    let boxes = parse_boxes(&optimized.bytes, 0..optimized.bytes.len()).unwrap();
    assert_eq!(boxes[1].kind.as_bytes(), b"moov");
    assert_eq!(boxes[2].kind.as_bytes(), b"mdat");
    assert!(optimized.bytes.windows(4).any(|v| v == b"co64"));
    assert_eq!(track_summary(&optimized.bytes).unwrap().tracks.len(), 2);
    let removed = remove_tracks(&movie, &[2], OptimizeOptions::default()).unwrap();
    assert_eq!(removed.report.retained_unreferenced_media_bytes, 5);
    assert_eq!(removed.report.discarded_unreferenced_media_bytes, 0);
    let compact = remove_tracks(
        &movie,
        &[2],
        OptimizeOptions {
            compact_unreferenced_media: true,
        },
    )
    .unwrap();
    assert_eq!(compact.report.discarded_unreferenced_media_bytes, 5);
    assert_eq!(compact.bytes.len() + 5, removed.bytes.len());
    assert!(compact.report.encoded_samples_unchanged);
    assert!(remove_tracks(&movie, &[1, 2], OptimizeOptions::default()).is_err());
}
#[test]
fn import_allocates_unique_ids_and_rescales_only_movie_timeline_exactly() {
    let destination = common::movie(1000, 1);
    let source = common::movie(600, 2);
    let imported = import_tracks(&destination, &source, &[2], OptimizeOptions::default()).unwrap();
    let model = track_summary(&imported.bytes).unwrap();
    assert_eq!(
        model.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(model.tracks[1].timescale, 600);
    assert_eq!(model.tracks[1].duration, 1200);
    assert_eq!(
        imported.report.track_mappings[1],
        TrackMapping {
            source: 1,
            original_id: 2,
            output_id: 2
        }
    );
    assert!(import_tracks(&destination, &source, &[2, 2], OptimizeOptions::default()).is_err());
    let mut fractional = source.clone();
    let at = fractional.windows(4).position(|v| v == b"tkhd").unwrap() + 24;
    fractional[at..at + 4].copy_from_slice(&1u32.to_be_bytes());
    assert!(matches!(
        import_tracks(&destination, &fractional, &[1], OptimizeOptions::default()),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
}
#[test]
fn hvc1_is_supported_without_decoding_while_fragmented_and_unknown_layouts_fail() {
    let mut movie = common::movie(1000, 1);
    let at = movie.windows(4).position(|v| v == b"avc1").unwrap();
    movie[at..at + 4].copy_from_slice(b"hvc1");
    assert!(optimization_support(&movie).is_ok());
    assert!(optimize(&movie, OptimizeOptions::default()).is_ok());
    for kind in [b"moof", b"sidx", b"uuid"] {
        let data = [movie.clone(), common::boxed(kind, b"opaque")].concat();
        assert!(matches!(
            optimization_support(&data),
            Err(MotionPhotoReadError::UnsupportedLayout(_))
        ));
    }
}
#[test]
fn real_h264_b_frames_and_apple_metadata_survive_optimization() {
    let movie = include_bytes!("../../../web/public/demo/motion.mp4");
    let optimized = optimize(movie, OptimizeOptions::default()).unwrap();
    assert_eq!(
        video_frame_times(movie, 1, 1000).unwrap(),
        video_frame_times(&optimized.bytes, 1, 1000).unwrap()
    );
    let apple = include_bytes!("../../../web/public/demo/apple-paired.mov");
    let optimized = optimize(apple, OptimizeOptions::default()).unwrap();
    let still = include_bytes!("../../../web/public/demo/apple-still.jpg");
    let photo = MotionPhoto::parse(
        Input::ApplePair {
            still,
            movie: &optimized.bytes,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(photo
        .presentation_time()
        .unwrap()
        .equivalent(MediaTime::new(13, 16).unwrap()));
}
