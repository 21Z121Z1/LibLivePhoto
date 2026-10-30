mod common;
use liblivephoto::*;
use liblivephoto_format::isobmff::{parse_boxes, BoxHeader};

fn selected() -> GoogleVideoTracks {
    GoogleVideoTracks {
        primary: 1,
        secondary: None,
    }
}
fn path(movie: &[u8], track: usize, tail: &[&[u8; 4]]) -> Vec<BoxHeader> {
    let moov = parse_boxes(movie, 0..movie.len())
        .unwrap()
        .into_iter()
        .find(|b| b.kind.as_bytes() == b"moov")
        .unwrap();
    let trak = parse_boxes(movie, moov.payload_range())
        .unwrap()
        .into_iter()
        .filter(|b| b.kind.as_bytes() == b"trak")
        .nth(track)
        .unwrap();
    let mut parents = vec![moov, trak];
    for kind in tail {
        let b = parse_boxes(movie, parents.last().unwrap().payload_range())
            .unwrap()
            .into_iter()
            .find(|b| b.kind.as_bytes() == *kind)
            .unwrap();
        parents.push(b);
    }
    parents
}
fn replace_entries(movie: &mut Vec<u8>, track: usize, entries: &[Vec<u8>]) {
    let parents = path(movie, track, &[b"mdia", b"minf", b"stbl", b"stsd"]);
    let stsd = parents.last().unwrap();
    let new = [
        (entries.len() as u32).to_be_bytes().to_vec(),
        entries.concat(),
    ]
    .concat();
    let old = stsd.data_end - stsd.data_start - 4;
    movie.splice(stsd.data_start + 4..stsd.data_end, new.iter().copied());
    for b in parents {
        let len = (b.data_end - b.box_start) as i64 + new.len() as i64 - old as i64;
        movie[b.box_start..b.box_start + 4].copy_from_slice(&(len as u32).to_be_bytes());
    }
}
fn visual(codec: &[u8; 4], width: u16, height: u16) -> Vec<u8> {
    let mut data = vec![0; 78];
    data[24..26].copy_from_slice(&width.to_be_bytes());
    data[26..28].copy_from_slice(&height.to_be_bytes());
    common::boxed(codec, &data)
}
fn esds(asc: &[u8], duplicate: bool) -> Vec<u8> {
    let specific = [vec![5, asc.len() as u8], asc.to_vec()].concat();
    let config = [
        vec![0x40, 0x15],
        vec![0; 11],
        specific.clone(),
        if duplicate { specific } else { Vec::new() },
    ]
    .concat();
    let decoder = [vec![4, config.len() as u8], config].concat();
    let es = [vec![0, 1, 0], decoder].concat();
    common::full(b"esds", &[vec![3, es.len() as u8], es].concat())
}
fn audio_entry(asc: &[u8], version: u16, size: u16, wave: bool, duplicate: bool) -> Vec<u8> {
    let mut data = vec![0; 28];
    data[8..10].copy_from_slice(&version.to_be_bytes());
    data[16..18].copy_from_slice(&2u16.to_be_bytes());
    data[18..20].copy_from_slice(&size.to_be_bytes());
    data[24..28].copy_from_slice(&(48000u32 << 16).to_be_bytes());
    let configuration = esds(asc, false);
    data.extend(if wave {
        common::boxed(b"wave", &configuration)
    } else {
        configuration.clone()
    });
    if duplicate {
        data.extend(configuration);
    }
    common::boxed(b"mp4a", &data)
}
fn audio_movie(entry: Vec<u8>) -> Vec<u8> {
    let mut movie = common::metadata_movie(b"mp4a", &[0; 20], &[(&[1, 2, 3], 2000)], 0);
    let hdlr = path(&movie, 1, &[b"mdia", b"hdlr"]).pop().unwrap();
    movie[hdlr.data_start + 8..hdlr.data_start + 12].copy_from_slice(b"soun");
    replace_entries(&mut movie, 1, &[entry]);
    movie
}
fn check(movie: &[u8]) -> FormatConformanceReport {
    validate_google_video_container(movie, selected()).unwrap()
}
fn has(report: &FormatConformanceReport, rule: &str, kind: FormatIssueKind) -> bool {
    report
        .issues
        .iter()
        .any(|i| i.rule == rule && i.kind == kind)
}
fn add_ctts(movie: &mut Vec<u8>, track: usize, count: u32, offset: u32) {
    let parents = path(movie, track, &[b"mdia", b"minf", b"stbl"]);
    let end = parents.last().unwrap().data_end;
    let table = common::full(
        b"ctts",
        &[
            1u32.to_be_bytes(),
            count.to_be_bytes(),
            offset.to_be_bytes(),
        ]
        .concat(),
    );
    movie.splice(end..end, table.iter().copied());
    for b in parents {
        let len = (b.data_end - b.box_start + table.len()) as u32;
        movie[b.box_start..b.box_start + 4].copy_from_slice(&len.to_be_bytes());
    }
}

#[test]
fn codec_declarations_and_observed_dimensions_do_not_claim_decoding_or_fixed_resolution() {
    for codec in [b"avc1", b"avc3", b"hvc1", b"hev1", b"av01"] {
        let mut movie = common::many_frame_movie(3, 2);
        replace_entries(&mut movie, 0, &[visual(codec, 1920, 1080)]);
        replace_entries(&mut movie, 1, &[visual(codec, 320, 200)]);
        let report = validate_google_video_container(
            &movie,
            GoogleVideoTracks {
                primary: 1,
                secondary: Some(2),
            },
        )
        .unwrap();
        assert!(report.checked_rules_satisfied(), "{:?}", report.issues);
        assert!(report.issues.is_empty());
        let dimensions = inspect_video_sample_descriptions(&movie).unwrap();
        assert_eq!(
            dimensions[0].dimensions,
            Some(VideoDimensions {
                width: 1920,
                height: 1080
            })
        );
        assert_eq!(
            dimensions[1].dimensions,
            Some(VideoDimensions {
                width: 320,
                height: 200
            })
        );
        // The public section leaves frame resolution undefined; no made-up
        // minimum or secondary>primary pixel rule is used to reject this file.
        let before = movie.clone();
        let still = [255, 216, 255, 217];
        let photo = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: None,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let written = photo
            .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
            .execute()
            .unwrap();
        let reopened = MotionPhoto::parse(
            Input::SingleFile(&written.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(reopened.motion_video_bytes().unwrap(), before);
    }
}

#[test]
fn all_sample_descriptions_are_checked_and_default_preservation_is_independent() {
    let mut movie = common::many_frame_movie(1, 1);
    replace_entries(
        &mut movie,
        0,
        &[visual(b"avc1", 2, 3), visual(b"vp09", 2, 3)],
    );
    let report = check(&movie);
    assert!(has(&report, "G08.video-codec", FormatIssueKind::Violation));
    assert!(!report.checked_rules_satisfied());
    replace_entries(&mut movie, 0, &[visual(b"zzzz", 2, 3)]);
    let report = check(&movie);
    assert!(has(
        &report,
        "G08.video-codec",
        FormatIssueKind::Unsupported
    ));
    assert!(!report
        .issues
        .iter()
        .any(|i| i.kind == FormatIssueKind::Violation));
    let still = [255, 216, 255, 217];
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: None,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(photo
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .can_execute());
    assert!(!photo
        .google_video_conformance(selected())
        .unwrap()
        .checked_rules_satisfied());
}

#[test]
fn aac_rate_channels_and_sample_size_have_separate_declared_contracts() {
    for asc in [&[0x11, 0x90][..], &[0x11, 0x88], &[0x10, 0x10]] {
        let movie = audio_movie(audio_entry(asc, 0, 16, false, false));
        let report = check(&movie);
        assert!(report.checked_rules_satisfied(), "{:?}", report.issues);
        let facts = inspect_video_sample_descriptions(&movie).unwrap();
        let audio = facts[1].audio.as_ref().unwrap();
        assert_eq!(audio.sample_entry_sample_size, 16);
        assert_eq!(audio.aac.as_ref().unwrap().raw, asc);
        // AAC96kHz is not rejected by the legacy16.16 field's integer width.
    }
    for (asc, size, rule) in [
        (&[0x11, 0xb0][..], 16, "G08.audio-channels"),
        (&[0x12, 0x90][..], 16, "G08.audio-rate"),
        (&[0x11, 0x90][..], 24, "G08.audio-depth"),
    ] {
        let report = check(&audio_movie(audio_entry(asc, 0, size, false, false)));
        assert!(has(&report, rule, FormatIssueKind::Violation));
    }
    let report = check(&audio_movie(audio_entry(
        &[0x12, 0x10],
        0,
        16,
        false,
        false,
    )));
    assert!(has(&report, "G08.audio-rate", FormatIssueKind::Unsupported));
    assert!(!has(&report, "G08.audio-rate", FormatIssueKind::Violation));
}

#[test]
fn pce_quicktime_v2_wave_and_duplicate_declarations_never_become_false_passes() {
    // This declares PCE without carrying one: now a known truncation error,
    // rather than the old parser's unknown-channel result. It must never pass.
    let truncated = audio_movie(audio_entry(&[0x11, 0x80], 0, 16, false, false));
    assert!(matches!(
        validate_google_video_container(&truncated, selected()),
        Err(MotionPhotoReadError::InvalidResource)
    ));
    for entry in [
        audio_entry(&[0x11, 0x90], 2, 16, false, false),
        audio_entry(&[0x11, 0x90], 0, 16, true, false),
        audio_entry(&[0x11, 0x90], 0, 16, false, true),
    ] {
        let report = check(&audio_movie(entry));
        assert!(!report.checked_rules_satisfied());
        assert!(report
            .issues
            .iter()
            .any(|i| i.kind == FormatIssueKind::Unsupported));
        assert!(!report
            .issues
            .iter()
            .any(|i| i.kind == FormatIssueKind::Violation));
    }
    let mut entry = audio_entry(&[0x11, 0x90], 0, 16, false, false);
    entry.truncate(36);
    entry.extend(esds(&[0x11, 0x90], true));
    let len = entry.len() as u32;
    entry[..4].copy_from_slice(&len.to_be_bytes());
    assert!(has(
        &check(&audio_movie(entry)),
        "G08.declarations",
        FormatIssueKind::Unsupported
    ));
}

#[test]
fn primary_presence_track_counts_roles_and_order_are_explicit() {
    let mut movie = common::many_frame_movie(1, 1);
    let hdlr = path(&movie, 0, &[b"mdia", b"hdlr"]).pop().unwrap();
    movie[hdlr.data_start + 8..hdlr.data_start + 12].copy_from_slice(b"meta");
    assert!(has(
        &check(&movie),
        "G08.primary-video",
        FormatIssueKind::Violation
    ));
    let movie = common::many_frame_movie(2, 3);
    let report = validate_google_video_container(
        &movie,
        GoogleVideoTracks {
            primary: 1,
            secondary: Some(2),
        },
    )
    .unwrap();
    assert!(has(&report, "G09.video-count", FormatIssueKind::Violation));
    assert!(has(
        &report,
        "G09.track-association",
        FormatIssueKind::Unsupported
    ));
    let movie = common::many_frame_movie(2, 2);
    assert!(has(
        &check(&movie),
        "G09.track-association",
        FormatIssueKind::Unsupported
    ));
    let report = validate_google_video_container(
        &movie,
        GoogleVideoTracks {
            primary: 2,
            secondary: Some(1),
        },
    )
    .unwrap();
    assert!(has(
        &report,
        "G09.secondary-order",
        FormatIssueKind::Violation
    ));
    for selected in [
        GoogleVideoTracks {
            primary: 1,
            secondary: Some(1),
        },
        GoogleVideoTracks {
            primary: 9,
            secondary: None,
        },
    ] {
        assert!(matches!(
            validate_google_video_container(&movie, selected),
            Err(MotionPhotoReadError::InvalidResource)
        ));
    }
}

#[test]
fn secondary_timestamp_correspondence_is_recommended_and_never_rounds_or_selects_nearest() {
    let mut movie = common::many_frame_movie(3, 2);
    add_ctts(&mut movie, 1, 3, 1);
    let report = validate_google_video_container(
        &movie,
        GoogleVideoTracks {
            primary: 1,
            secondary: Some(2),
        },
    )
    .unwrap();
    assert!(has(
        &report,
        "G09.frame-correspondence-recommendation",
        FormatIssueKind::Recommendation
    ));
    assert!(report.checked_rules_satisfied());
    assert!(report.issues[0].message.starts_with("1 secondary frames"));
    // Rational1/3 timestamps must compare exactly without integer-us conversion.
    let mut movie = common::many_frame_movie(3, 2);
    for index in 0..2 {
        let mdhd = path(&movie, index, &[b"mdia", b"mdhd"]).pop().unwrap();
        movie[mdhd.data_start + 12..mdhd.data_start + 16].copy_from_slice(&3u32.to_be_bytes());
    }
    let report = validate_google_video_container(
        &movie,
        GoogleVideoTracks {
            primary: 1,
            secondary: Some(2),
        },
    )
    .unwrap();
    assert!(report.issues.is_empty());
}

#[test]
fn matching_uses_edited_movie_times_and_exact_cross_timescale_equivalence() {
    let mut movie = common::many_frame_movie(3, 2);
    let mdhd = path(&movie, 1, &[b"mdia", b"mdhd"]).pop().unwrap();
    movie[mdhd.data_start + 12..mdhd.data_start + 16].copy_from_slice(&10000u32.to_be_bytes());
    movie[mdhd.data_start + 16..mdhd.data_start + 20].copy_from_slice(&30u32.to_be_bytes());
    let stts = path(&movie, 1, &[b"mdia", b"minf", b"stbl", b"stts"])
        .pop()
        .unwrap();
    movie[stts.data_start + 12..stts.data_start + 16].copy_from_slice(&10u32.to_be_bytes());
    let selected = GoogleVideoTracks {
        primary: 1,
        secondary: Some(2),
    };
    assert!(validate_google_video_container(&movie, selected)
        .unwrap()
        .issues
        .is_empty());
    let movie = common::movie_track_edits(movie, 1, 1000, &[(1, -1, 1, 0), (3, 0, 1, 0)]);
    let report = validate_google_video_container(&movie, selected).unwrap();
    assert!(has(
        &report,
        "G09.frame-correspondence-recommendation",
        FormatIssueKind::Recommendation
    ));
}

#[test]
fn fragmented_nonunit_and_aggregate_frame_limits_stay_explicit() {
    let movie = [common::many_frame_movie(1, 1), common::boxed(b"moof", b"")].concat();
    let report = check(&movie);
    assert!(has(
        &report,
        "G09.frame-correspondence-recommendation",
        FormatIssueKind::Unsupported
    ));
    assert!(!report.checked_rules_satisfied());
    let movie = common::movie_track_edits(common::many_frame_movie(1, 1), 0, 1000, &[(1, 0, 2, 0)]);
    assert!(!check(&movie).checked_rules_satisfied());
    let movie = common::many_frame_movie(50001, 2);
    assert!(matches!(
        validate_google_video_container(
            &movie,
            GoogleVideoTracks {
                primary: 1,
                secondary: Some(2)
            }
        ),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn multiple_audio_tracks_and_known_non_aac_have_distinct_failures() {
    let mut movie = audio_movie(audio_entry(&[0x11, 0x90], 0, 16, false, false));
    let moov = path(&movie, 1, &[])[0].clone();
    let trak = path(&movie, 1, &[])[1].clone();
    let mut third = movie[trak.box_range()].to_vec();
    let tkhd = parse_boxes(&third, 8..third.len())
        .unwrap()
        .into_iter()
        .find(|b| b.kind.as_bytes() == b"tkhd")
        .unwrap();
    third[tkhd.data_start + 12..tkhd.data_start + 16].copy_from_slice(&3u32.to_be_bytes());
    movie.extend(&third);
    movie[moov.box_start..moov.box_start + 4]
        .copy_from_slice(&((moov.data_end - moov.box_start + third.len()) as u32).to_be_bytes());
    assert!(has(
        &check(&movie),
        "G08.audio-count",
        FormatIssueKind::Violation
    ));
    let mut entry = audio_entry(&[0x11, 0x90], 0, 16, false, false);
    entry[4..8].copy_from_slice(b"alac");
    assert!(has(
        &check(&audio_movie(entry)),
        "G08.audio-codec",
        FormatIssueKind::Violation
    ));
}

#[test]
fn property_payload_and_count_budgets_are_aggregate_across_sample_descriptions() {
    let mut movie = common::many_frame_movie(1, 1);
    let mut entry = visual(b"avc1", 2, 3);
    entry.extend(common::boxed(b"free", &vec![0; 8 * 1024 * 1024 + 1]));
    let len = entry.len() as u32;
    entry[..4].copy_from_slice(&len.to_be_bytes());
    replace_entries(&mut movie, 0, &[entry.clone(), entry]);
    assert!(matches!(
        inspect_video_sample_descriptions(&movie),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let mut entry = visual(b"avc1", 2, 3);
    entry.extend(common::boxed(b"free", &[]).repeat(32769));
    let len = entry.len() as u32;
    entry[..4].copy_from_slice(&len.to_be_bytes());
    let mut movie = common::many_frame_movie(1, 1);
    replace_entries(&mut movie, 0, &[entry.clone(), entry]);
    assert!(matches!(
        inspect_video_sample_descriptions(&movie),
        Err(MotionPhotoReadError::InputLimit)
    ));
}
