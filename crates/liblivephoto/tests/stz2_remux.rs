mod common;
use liblivephoto::{remux::*, *};
use liblivephoto_format::isobmff::parse_boxes;

fn rewrite(d: &[u8], f: &mut impl FnMut(&[u8; 4], &[u8]) -> Option<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    for b in parse_boxes(d, 0..d.len()).unwrap() {
        let kind = b.kind.as_bytes();
        let p = &d[b.payload_range()];
        let bytes = if let Some(v) = f(kind, p) {
            v
        } else if matches!(kind, b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl") {
            common::boxed(kind, &rewrite(p, f))
        } else {
            d[b.box_range()].to_vec()
        };
        out.extend(bytes);
    }
    out
}
fn compact_table(p: &[u8], width: u8) -> Vec<u8> {
    let n = u32::from_be_bytes(p[8..12].try_into().unwrap()) as usize;
    let constant = u32::from_be_bytes(p[4..8].try_into().unwrap());
    let sizes = (0..n)
        .map(|i| {
            if constant != 0 {
                constant
            } else {
                u32::from_be_bytes(p[12 + i * 4..16 + i * 4].try_into().unwrap())
            }
        })
        .collect::<Vec<_>>();
    assert!(sizes.iter().all(|v| *v < (1 << width)));
    let mut q = vec![0; 8];
    q[7] = width;
    q.extend((n as u32).to_be_bytes());
    match width {
        4 => {
            for pair in sizes.chunks(2) {
                q.push(((pair[0] as u8) << 4) | pair.get(1).map(|n| *n as u8).unwrap_or(0x0a));
            }
        }
        8 => q.extend(sizes.iter().map(|n| *n as u8)),
        16 => q.extend(sizes.iter().flat_map(|n| (*n as u16).to_be_bytes())),
        _ => panic!("unsupported authored width"),
    }
    common::boxed(b"stz2", &q)
}
fn convert(d: &[u8], width: u8) -> Vec<u8> {
    rewrite(d, &mut |k, p| {
        (k == b"stsz").then(|| compact_table(p, width))
    })
}
type PacketFacts = (usize, i64, u32, i64, u32, Vec<u8>);
fn snapshot(d: &[u8], id: u32) -> Vec<PacketFacts> {
    metadata_packets(d, id)
        .unwrap()
        .iter()
        .map(|p| {
            (
                p.sample_index,
                p.presentation.value,
                p.presentation.timescale,
                p.duration.value,
                p.duration.timescale,
                p.bytes.to_vec(),
            )
        })
        .collect()
}
fn tables(d: &[u8], kind: &[u8; 4]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    rewrite(d, &mut |k, p| {
        if k == kind {
            out.push(p.to_vec());
        }
        None
    });
    out
}
#[test]
fn all_compact_widths_preserve_each_sample_cts_dts_and_relocated_index_targets() {
    for (width, sizes) in [(4, [1, 2, 3]), (8, [20, 100, 255]), (16, [300, 1, 700])] {
        let payloads = sizes.map(|n| vec![0x31; n]);
        let samples = [
            (payloads[0].as_slice(), 2),
            (payloads[1].as_slice(), 5),
            (payloads[2].as_slice(), 3),
        ];
        let old = common::metadata_movie(b"zzzz", &[], &samples, -1);
        let indexed = convert(&old, width);
        let before = snapshot(&indexed, 2);
        assert_eq!(before, snapshot(&old, 2));
        // Independently enumerated DTS0,2,7 plus ctts=-1 gives CTS-1,1,6.
        assert_eq!(before.iter().map(|p| p.1).collect::<Vec<_>>(), [-1, 1, 6]);
        assert_eq!(before.iter().map(|p| p.3).collect::<Vec<_>>(), [2, 5, 3]);
        for compact_unreferenced_media in [false, true] {
            let out = optimize(
                &indexed,
                OptimizeOptions {
                    compact_unreferenced_media,
                },
            )
            .unwrap();
            assert!(out.report.encoded_samples_unchanged);
            assert_eq!(snapshot(&out.bytes, 2), before);
            assert_eq!(tables(&out.bytes, b"stts"), tables(&indexed, b"stts")); // Exact DTS construction unchanged.
            assert_eq!(tables(&out.bytes, b"ctts"), tables(&indexed, b"ctts"));
            assert_eq!(tables(&out.bytes, b"stz2"), tables(&indexed, b"stz2")); // Includes the unused odd nibble.
            assert!(out.bytes.windows(4).any(|w| w == b"co64"));
            assert_eq!(
                video_frame_times(&out.bytes, 1, 100).unwrap(),
                video_frame_times(&old, 1, 100).unwrap()
            );
        }
    }
}
#[test]
fn import_and_removal_keep_compact_sizes_and_raw_unknown_sample_entries() {
    let old = common::metadata_movie(
        b"zzzz",
        b"uninterpreted",
        &[(&[1, 2], 9), (&[3, 4, 5], 11)],
        4,
    );
    let indexed = convert(&old, 8);
    let before = snapshot(&indexed, 2);
    let imported = import_tracks(
        &common::movie(1000, 1),
        &indexed,
        &[2],
        OptimizeOptions::default(),
    )
    .unwrap();
    let id = imported
        .report
        .track_mappings
        .iter()
        .find(|m| m.source == 1)
        .unwrap()
        .output_id;
    assert_eq!(snapshot(&imported.bytes, id), before);
    assert!(metadata_packets(&imported.bytes, id)
        .unwrap()
        .iter()
        .all(|p| p.sample_entry == *b"zzzz"));
    for out in [
        remove_tracks(&indexed, &[2], OptimizeOptions::default())
            .unwrap()
            .bytes,
        remove_metadata_tracks(&indexed, &[2]).unwrap().bytes,
    ] {
        assert_eq!(track_summary(&out).unwrap().tracks.len(), 1);
        assert_eq!(
            video_frame_times(&out, 1, 100).unwrap(),
            video_frame_times(&indexed, 1, 100).unwrap()
        );
    }
}
#[test]
fn malformed_tables_and_unknown_reference_combinations_do_not_get_relaxed() {
    let base = convert(
        &common::metadata_movie(b"zzzz", &[], &[(&[1], 1), (&[2, 3], 2), (&[4], 3)], 0),
        4,
    );
    for (at, bytes) in [
        (0, vec![1]),
        (3, vec![1]),
        (4, vec![1]),
        (7, vec![7]),
        (8, 1_000_001u32.to_be_bytes().to_vec()),
        (8, 4u32.to_be_bytes().to_vec()),
    ] {
        let bad = rewrite(&base, &mut |k, p| {
            if k != b"stz2" {
                return None;
            }
            let mut q = p.to_vec();
            q[at..at + bytes.len()].copy_from_slice(&bytes);
            Some(common::boxed(k, &q))
        });
        assert!(track_summary(&bad).is_err(), "field{at}");
        assert!(optimize(&bad, OptimizeOptions::default()).is_err());
    }
    for extra in [true, false] {
        let bad = rewrite(&base, &mut |k, p| {
            if k != b"stz2" {
                return None;
            }
            let mut q = p.to_vec();
            if extra {
                q.push(0);
            } else {
                q.pop();
            }
            Some(common::boxed(k, &q))
        });
        assert!(track_summary(&bad).is_err());
    }
    let duplicate = rewrite(&base, &mut |k, p| {
        (k == b"stz2").then(|| [common::boxed(k, p), common::boxed(k, p)].concat())
    });
    assert!(matches!(
        track_summary(&duplicate),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
    let unknown = rewrite(&base, &mut |k, p| {
        (k == b"stz2").then(|| [common::boxed(k, p), common::boxed(b"nope", b"opaque")].concat())
    });
    assert!(matches!(
        optimization_support(&unknown),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    for kind in [b"sidx", b"moof", b"mfra", b"uuid"] {
        let d = [base.clone(), common::boxed(kind, b"opaque")].concat();
        assert!(matches!(
            optimization_support(&d),
            Err(MotionPhotoReadError::UnsupportedLayout(_))
        ));
    }
}
