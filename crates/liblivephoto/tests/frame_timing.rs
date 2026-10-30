mod common;
use liblivephoto::*;
use liblivephoto_format::isobmff::parse_boxes;
fn insert(data: &mut Vec<u8>, path: &[&[u8; 4]], extra: &[u8]) {
    let mut range = 0..data.len();
    let mut positions = Vec::new();
    for kind in path {
        let h = parse_boxes(data, range)
            .unwrap()
            .into_iter()
            .find(|h| h.kind.as_bytes() == *kind)
            .unwrap();
        positions.push(h.box_start);
        range = h.data_start..h.data_end;
    }
    let end = range.end;
    data.splice(end..end, extra.iter().copied());
    for pos in positions {
        let len = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap());
        data[pos..pos + 4].copy_from_slice(&(len + extra.len() as u32).to_be_bytes());
    }
    let stco = data.windows(4).position(|p| p == b"stco").unwrap() + 12;
    let old = u32::from_be_bytes(data[stco..stco + 4].try_into().unwrap());
    data[stco..stco + 4].copy_from_slice(&(old + extra.len() as u32).to_be_bytes());
}
fn edits(rate: u16) -> Vec<u8> {
    let words = [2u32, 500, u32::MAX, 0x10000, 1500, 0, u32::from(rate) << 16];
    common::boxed(
        b"edts",
        &common::full(
            b"elst",
            &words
                .into_iter()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
        ),
    )
}
#[test]
fn sample_times_include_empty_edits_and_composition_offsets() {
    let mut movie = common::movie(1000, 1);
    insert(&mut movie, &[b"moov", b"trak"], &edits(1));
    insert(
        &mut movie,
        &[b"moov", b"trak", b"mdia", b"minf", b"stbl"],
        &common::full(
            b"ctts",
            &[1u32, 1, 100]
                .into_iter()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
        ),
    );
    let frames = video_frame_times(&movie, 1, 100).unwrap();
    assert_eq!(
        frames,
        vec![FrameTime {
            sample_index: 0,
            time: MediaTime::new(600, 1000).unwrap()
        }]
    );
    assert!(matches!(
        video_frame_times(&movie, 1, 0),
        Err(MotionPhotoReadError::InputLimit)
    ));
}
#[test]
fn unsupported_rates_and_bad_composition_counts_fail_explicitly() {
    let mut movie = common::movie(1000, 1);
    insert(&mut movie, &[b"moov", b"trak"], &edits(2));
    assert!(matches!(
        video_frame_times(&movie, 1, 5),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    let mut movie = common::movie(1000, 1);
    insert(
        &mut movie,
        &[b"moov", b"trak", b"mdia", b"minf", b"stbl"],
        &common::full(
            b"ctts",
            &[1u32, 2, 100]
                .into_iter()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
        ),
    );
    assert!(video_frame_times(&movie, 1, 5).is_err());
}
#[test]
fn signed_ctts_and_fractional_media_time_stay_exact() {
    let mut movie = common::movie(3, 1);
    let mut ctts = common::full(
        b"ctts",
        &[1u32, 1, u32::MAX]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>(),
    );
    ctts[8] = 1;
    insert(
        &mut movie,
        &[b"moov", b"trak", b"mdia", b"minf", b"stbl"],
        &ctts,
    );
    assert_eq!(
        video_frame_times(&movie, 1, 5).unwrap()[0].time,
        MediaTime::new(-1, 3).unwrap()
    );
}
#[test]
fn edit_and_media_timescales_are_added_without_rounding() {
    let mut movie = common::movie(1000, 1);
    let mdhd = movie.windows(4).position(|v| v == b"mdhd").unwrap();
    movie[mdhd + 16..mdhd + 20].copy_from_slice(&3u32.to_be_bytes());
    insert(&mut movie, &[b"moov", b"trak"], &edits(1));
    insert(
        &mut movie,
        &[b"moov", b"trak", b"mdia", b"minf", b"stbl"],
        &common::full(
            b"ctts",
            &[1u32, 1, 1]
                .into_iter()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
        ),
    );
    let actual = video_frame_times(&movie, 1, 5).unwrap()[0].time;
    assert!(actual.equivalent(MediaTime::new(5, 6).unwrap()));
    assert_eq!(actual.rescale_exact(1_000_000), None);
}
