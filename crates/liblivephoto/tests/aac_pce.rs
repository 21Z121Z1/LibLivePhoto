mod common;
use liblivephoto::*;

fn put(bits: &mut Vec<bool>, value: u32, width: usize) {
    for n in (0..width).rev() {
        bits.push(value & (1 << n) != 0);
    }
}
fn asc(
    front: &[(bool, u8)],
    side: &[(bool, u8)],
    back: &[(bool, u8)],
    lfe: &[u8],
    profile: u8,
    rate: u8,
) -> Vec<u8> {
    let mut b = Vec::new();
    for (v, n) in [
        (2, 5),
        (3, 4),
        (0, 4),
        (0, 3),
        (7, 4),
        (profile as u32, 2),
        (rate as u32, 4),
        (front.len() as u32, 4),
        (side.len() as u32, 4),
        (back.len() as u32, 4),
        (lfe.len() as u32, 2),
        (1, 3),
        (1, 4),
        (0, 3),
    ] {
        put(&mut b, v, n);
    }
    for &(pair, tag) in front.iter().chain(side).chain(back) {
        put(&mut b, pair as u32, 1);
        put(&mut b, tag as u32, 4);
    }
    for &tag in lfe {
        put(&mut b, tag as u32, 4);
    }
    put(&mut b, 3, 4);
    put(&mut b, 1, 1);
    put(&mut b, 9, 4); // DSE and coupling do not add output channels.
    while !b.len().is_multiple_of(8) {
        b.push(false);
    }
    put(&mut b, 3, 8);
    for v in [0, 255, 42] {
        put(&mut b, v, 8);
    }
    b.chunks(8)
        .map(|x| x.iter().fold(0, |v, b| (v << 1) | u8::from(*b)))
        .collect()
}
fn descriptor(tag: u8, data: &[u8]) -> Vec<u8> {
    let mut n = data.len();
    let mut bytes = vec![(n & 127) as u8];
    n >>= 7;
    while n != 0 {
        bytes.push((n & 127) as u8 | 128);
        n >>= 7;
    }
    [vec![tag], bytes.into_iter().rev().collect(), data.to_vec()].concat()
}
fn esds(raw: &[u8]) -> Vec<u8> {
    let c = [vec![0x40, 0x15], vec![0; 11], descriptor(5, raw)].concat();
    common::full(
        b"esds",
        &descriptor(3, &[vec![0, 1, 0], descriptor(4, &c)].concat()),
    )
}
fn wave(raw: &[u8]) -> Vec<u8> {
    common::boxed(
        b"wave",
        &[
            common::boxed(b"frma", b"mp4a"),
            common::boxed(b"mp4a", &[0; 4]),
            esds(raw),
            common::boxed(&[0; 4], &[]),
        ]
        .concat(),
    )
}
fn movie(children: &[u8], version: u16) -> Vec<u8> {
    let mut fields = vec![0; if version == 1 { 36 } else { 20 }];
    fields[..2].copy_from_slice(&version.to_be_bytes());
    fields[8..10].copy_from_slice(&2u16.to_be_bytes());
    fields[16..20].copy_from_slice(&(48000u32 << 16).to_be_bytes());
    let mut m = common::metadata_movie(
        b"mp4a",
        &[fields, children.to_vec()].concat(),
        &[(&[1, 2, 3], 2000)],
        0,
    );
    let at = m.windows(4).rposition(|w| w == b"meta").unwrap();
    m[at..at + 4].copy_from_slice(b"soun");
    m
}
fn inspect(movie: &[u8]) -> std::result::Result<AacConfiguration, MotionPhotoReadError> {
    let p = MotionPhoto::parse(
        Input::Parts {
            still: &[255, 216, 255, 217],
            movie,
            presentation: MediaTime::new(0, 1),
        },
        ParseOptions::strict(),
    )?;
    p.sample_descriptions()?
        .into_iter()
        .find(|d| d.kind == TrackKind::Audio)
        .and_then(|d| d.audio)
        .and_then(|a| a.aac)
        .ok_or(MotionPhotoReadError::InvalidResource)
}

#[test]
fn lc_pce_counts_channels_separately_from_data_coupling_and_preserves_raw() {
    for (front, side, back, lfe, expected) in [
        (vec![(true, 0)], vec![], vec![], vec![], 2),
        (
            vec![(false, 0), (true, 0)],
            vec![(true, 1)],
            vec![(false, 1)],
            vec![0],
            7,
        ),
    ] {
        let raw = asc(&front, &side, &back, &lfe, 1, 3);
        let c = inspect(&movie(&esds(&raw), 0)).unwrap();
        assert_eq!(c.channels, Some(expected));
        assert_eq!(c.channel_configuration, 0);
        assert_eq!(c.raw, raw);
        let p = c.program_config.unwrap();
        assert_eq!(p.instance_tag, 7);
        assert_eq!(p.comment, vec![0, 255, 42]);
        assert_eq!(p.associated_data_tags, vec![3]);
        assert_eq!(p.coupling_elements, vec![(true, 9)]);
        assert_eq!(p.lfe_tags, lfe);
        assert_eq!(p.front.len(), front.len());
    }
}
#[test]
fn qt_wave_v0_v1_match_direct_esds_without_rewriting_the_original() {
    let raw = asc(&[(true, 1)], &[], &[], &[], 1, 3);
    let expected = inspect(&movie(&esds(&raw), 0)).unwrap();
    for version in [0, 1] {
        let bytes = movie(&wave(&raw), version);
        let before = bytes.clone();
        assert_eq!(inspect(&bytes).unwrap(), expected);
        assert_eq!(bytes, before);
        let p = MotionPhoto::parse(
            Input::Parts {
                still: &[255, 216, 255, 217],
                movie: &bytes,
                presentation: MediaTime::new(1, 2),
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let built = p
            .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
            .execute()
            .unwrap();
        let reopened = MotionPhoto::parse(
            Input::SingleFile(&built.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(reopened.motion_video_bytes().unwrap(), bytes);
    }
}
#[test]
fn pce_conflicts_truncation_unsupported_ga_and_input_budget_fail_explicitly() {
    let valid = asc(&[(true, 0)], &[], &[], &[], 1, 3);
    for n in 0..valid.len() {
        assert!(inspect(&movie(&esds(&valid[..n]), 0)).is_err(), "prefix{n}");
    }
    for raw in [
        asc(&[(true, 0)], &[], &[], &[], 0, 3),
        asc(&[(true, 0)], &[], &[], &[], 1, 4),
        asc(&[(false, 0)], &[], &[(false, 0)], &[], 1, 3),
        asc(&[(true, 0)], &[], &[], &[], 1, 15),
    ] {
        assert!(inspect(&movie(&esds(&raw), 0)).is_err());
    }
    for flag in [1, 2] {
        let mut raw = valid.clone();
        raw[1] |= flag;
        assert!(matches!(
            inspect(&movie(&esds(&raw), 0)),
            Err(MotionPhotoReadError::UnsupportedLayout(_))
        ));
    }
    let mut large = valid;
    large.resize(64 * 1024 + 1, 0);
    assert!(matches!(
        inspect(&movie(&esds(&large), 0)),
        Err(MotionPhotoReadError::InputLimit)
    ));
}
#[test]
fn wrapper_conflicts_unknown_children_versions_and_truncation_are_rejected() {
    let raw = asc(&[(true, 0)], &[], &[], &[], 1, 3);
    let base = wave(&raw);
    assert!(inspect(&movie(&[esds(&raw), base.clone()].concat(), 0)).is_err());
    for children in [
        [
            common::boxed(b"frma", b"avc1"),
            esds(&raw),
            common::boxed(&[0; 4], &[]),
        ]
        .concat(),
        [
            common::boxed(b"frma", b"mp4a"),
            esds(&raw),
            esds(&raw),
            common::boxed(&[0; 4], &[]),
        ]
        .concat(),
        [
            common::boxed(b"frma", b"mp4a"),
            common::boxed(b"nope", &[]),
            esds(&raw),
            common::boxed(&[0; 4], &[]),
        ]
        .concat(),
        [common::boxed(b"frma", b"mp4a"), esds(&raw)].concat(),
    ] {
        assert!(inspect(&movie(&common::boxed(b"wave", &children), 0)).is_err());
    }
    let mut bad = esds(&raw);
    bad[8] = 1;
    assert!(matches!(
        inspect(&movie(&bad, 0)),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    for end in 0..base.len() {
        assert!(inspect(&movie(&base[..end], 1)).is_err() || end == 0);
    }
}

#[test]
fn lc_sync_suffix_and_descriptor_budgets_are_explicit() {
    let valid = asc(&[(true, 0)], &[], &[], &[], 1, 3);
    let no_sbr = [valid.clone(), vec![0x56, 0xe5, 0]].concat();
    assert_eq!(
        inspect(&movie(&esds(&no_sbr), 0)).unwrap().channels,
        Some(2)
    );
    for suffix in [
        vec![0x56, 0xe5, 0x80],
        vec![0x56, 0xe5],
        vec![0xff, 0xff, 0xff],
        vec![0],
    ] {
        assert!(inspect(&movie(&esds(&[valid.clone(), suffix].concat()), 0)).is_err());
    }
    let config = [vec![0x40, 0x15], vec![0; 11], descriptor(5, &valid)].concat();
    let tail = [6, 0].repeat(4097);
    let too_many = common::full(
        b"esds",
        &descriptor(3, &[vec![0, 1, 0], descriptor(4, &config), tail].concat()),
    );
    assert!(matches!(
        inspect(&movie(&too_many, 0)),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn matching_44100_declarations_are_retained_and_public_44khz_ambiguity_stays_unsupported() {
    let mut raw = asc(&[(true, 0)], &[], &[], &[], 1, 4);
    raw[0] = (raw[0] & 0xf8) | 2;
    raw[1] &= 0x7f; // ASC and PCE both declare index4:44100.
    let bytes = movie(&wave(&raw), 0);
    let c = inspect(&bytes).unwrap();
    assert_eq!(c.sample_rate, 44100);
    assert_eq!(c.raw, raw);
    let report = validate_google_video_container(
        &bytes,
        GoogleVideoTracks {
            primary: 1,
            secondary: None,
        },
    )
    .unwrap();
    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == "G08.audio-rate" && i.kind == FormatIssueKind::Unsupported));
    let unknown = movie(&wave(&asc(&[(true, 0)], &[], &[], &[], 1, 3)), 2);
    let report = validate_google_video_container(
        &unknown,
        GoogleVideoTracks {
            primary: 1,
            secondary: None,
        },
    )
    .unwrap();
    assert!(report
        .issues
        .iter()
        .any(|i| i.kind == FormatIssueKind::Unsupported && i.message.contains("QuickTime v2")));
}
