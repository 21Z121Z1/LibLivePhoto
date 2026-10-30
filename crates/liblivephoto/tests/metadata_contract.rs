mod common;
use liblivephoto::*;

fn tag20(tag: &str) -> Vec<u8> {
    let mut p = tag.as_bytes().to_vec();
    p.resize(20, 0);
    p
}
fn tlv(data: &[u8], tag: &str) -> Vec<u8> {
    [
        data.to_vec(),
        (data.len() as u32).to_le_bytes().to_vec(),
        tag20(tag),
    ]
    .concat()
}

#[test]
fn honor_footer_time_and_unknown_tlv_resources_are_explicit() {
    let movie = common::movie(1000, 1);
    for cover in [None, Some("450")] {
        let input = [
            vec![255, 216, 255, 217],
            movie.clone(),
            cover.map_or_else(Vec::new, tag20),
            tag20(&format!("LIVE_{}", movie.len())),
        ]
        .concat();
        let photo =
            MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
        assert_eq!(photo.asset().dialect, VendorDialect::Honor);
        assert_eq!(photo.motion_video_bytes().unwrap(), movie);
        assert_eq!(
            photo.presentation_time(),
            cover.and_then(|_| MediaTime::new(450_000, 1_000_000))
        );
        assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
        assert!(!photo
            .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
            .can_execute());
        assert_eq!(
            photo
                .plan(TargetProfile::PreserveSource, WritePolicy::default())
                .execute()
                .unwrap()
                .files[0]
                .bytes,
            input
        );
    }
    let json = br#"{"common":{"sceneMode":73}}"#;
    let offline = [(json.len() as u32).to_le_bytes().to_vec(), json.to_vec()].concat();
    let input = [
        vec![255, 216, 255, 217],
        tlv(&movie, "HiHonor_1000000"),
        tlv(&offline, "HiHonor_OfflineData"),
        tlv(b"opaque", "HiHonor_CIID"),
    ]
    .concat();
    let metadata = inspect_honor_metadata(&input).unwrap().unwrap();
    assert_eq!(metadata.scene_mode, Some(73));
    let photo = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
    assert_eq!(photo.motion_video_bytes().unwrap(), movie);
    let out = photo
        .plan(
            TargetProfile::AndroidJpeg,
            WritePolicy {
                unmapped: UnmappedPolicy::PreserveSidecars,
                ..WritePolicy::default()
            },
        )
        .execute()
        .unwrap();
    for r in photo.resources().filter(|r| {
        r.parent.is_none() && r.id != photo.still().id && r.id != photo.motion_video().id
    }) {
        let mut original = Vec::new();
        photo.extract(r, &mut original).unwrap();
        assert_eq!(
            original,
            out.files
                .iter()
                .find(|f| f.sidecar_for == Some(r.id))
                .unwrap()
                .bytes
        );
    }
}

#[test]
fn honor_metadata_only_ambiguity_and_malformed_trailers_do_not_become_motion_photos() {
    let metadata_only = [vec![255, 216, 255, 217], tlv(b"CIID", "HiHonor_CIID")].concat();
    assert!(inspect_honor_metadata(&metadata_only)
        .unwrap()
        .unwrap()
        .video
        .is_none());
    assert!(MotionPhoto::parse(
        Input::SingleFile(&metadata_only),
        ParseOptions::compatible()
    )
    .is_err());
    let movie = common::movie(1000, 1);
    let two = [
        vec![255, 216, 255, 217],
        tlv(&movie, "HiHonor_1000000"),
        tlv(&movie, "HiHonor_1000001"),
    ]
    .concat();
    assert!(matches!(
        inspect_honor_metadata(&two),
        Err(MotionPhotoReadError::AmbiguousRecovery)
    ));
    let overflow = [
        vec![255, 216, 255, 217],
        movie.clone(),
        tag20("9223372036854775807"),
        tag20(&format!("LIVE_{}", movie.len())),
    ]
    .concat();
    assert!(matches!(
        inspect_honor_metadata(&overflow),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
    for input in [
        [
            vec![255, 216, 255, 217],
            u32::MAX.to_le_bytes().to_vec(),
            tag20("HiHonor_CIID"),
        ]
        .concat(),
        [vec![255, 216, 255, 217], tag20("LIVE_999999999999")].concat(),
    ] {
        assert!(inspect_honor_metadata(&input).is_err());
    }
    for end in 0..two.len() {
        let _ = inspect_honor_metadata(&two[..end]);
    }
}

#[test]
fn agtm_samples_use_exact_intervals_and_composition_time_without_raw_scanning() {
    let video = common::metadata_movie(
        b"it35",
        &[5, 0xb5, 0, 0x90, 0, 1],
        &[(&[0, 0], 1000), (&[0, 0x80, 3, 0xf7], 1000)],
        500,
    );
    let still = vec![255, 216, 255, 217];
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &video,
            presentation: MediaTime::new(1, 1),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(photo
        .agtm_at(MediaTime::new(0, 1).unwrap())
        .unwrap()
        .is_empty());
    let first = photo.agtm_at(MediaTime::new(1, 2).unwrap()).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].bytes, &[0, 0]);
    assert_eq!(first[0].header.hdr_reference_white_nits, 203.0);
    let second = photo.agtm_at(MediaTime::new(3, 2).unwrap()).unwrap();
    assert_eq!(second[0].sample_index, 1);
    assert_eq!(second[0].header.hdr_reference_white_nits, 203.0);
    assert!(photo
        .agtm_at(MediaTime::new(5, 2).unwrap())
        .unwrap()
        .is_empty());
    assert!(matches!(
        photo.agtm_at(MediaTime::new(1, 3).unwrap()),
        Err(MotionPhotoReadError::InvalidPresentation)
    ));
    let unrelated = common::metadata_movie(b"mett", b"unrelated", &[(&[0, 0], 1000)], 0);
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &unrelated,
            presentation: None,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(photo
        .agtm_at(MediaTime::new(0, 1).unwrap())
        .unwrap()
        .is_empty());
}

#[test]
fn agtm_truncation_and_unknown_version_never_fabricate_baseline_metadata() {
    for p in [
        &[][..],
        &[0][..],
        &[0, 0x80][..],
        &[0, 0x40, 0, 1][..],
        &[0x20, 0][..],
        &[0xb5, 0, 0x90][..],
    ] {
        assert!(decode_agtm_header(p).is_err());
    }
    assert_eq!(
        decode_agtm_header(&[5, 0xb5, 0, 0x90, 0, 1, 0, 0])
            .unwrap()
            .hdr_reference_white_nits,
        203.0
    );
    assert_eq!(
        decode_agtm_header(&[0, 0x40, 0x27, 0x10, 0x80])
            .unwrap()
            .baseline_hdr_headroom_log2,
        Some(1.0)
    );
}

#[test]
fn metadata_removal_is_opt_in_and_preserves_every_encoded_media_byte() {
    let video = common::metadata_movie(b"it35", &[5, 0xb5, 0, 0x90, 0, 1], &[(&[0, 0], 2000)], 0);
    let still = vec![255, 216, 255, 217];
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &video,
            presentation: MediaTime::new(1, 1),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let mut extracted = Vec::new();
    photo.extract(photo.motion_video(), &mut extracted).unwrap();
    assert_eq!(extracted, video);
    let output = photo
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .execute()
        .unwrap();
    let roundtrip = MotionPhoto::parse(
        Input::SingleFile(&output.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(roundtrip.motion_video_bytes().unwrap(), video);
    assert_eq!(
        roundtrip
            .agtm_at(MediaTime::new(1, 1).unwrap())
            .unwrap()
            .len(),
        1
    );
    let pair = photo
        .plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing: PairingPolicy::Set("KEEP-AGTM".into()),
                ..WritePolicy::default()
            },
        )
        .execute()
        .unwrap();
    let pair = MotionPhoto::parse(
        Input::ApplePair {
            still: &pair.files[0].bytes,
            movie: &pair.files[1].bytes,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(
        pair.agtm_at(MediaTime::new(1, 1).unwrap()).unwrap()[0].bytes,
        &[0, 0]
    );
    let stripped = remux::remove_metadata_tracks(&video, &[2]).unwrap();
    assert_eq!(stripped.removed_track_ids, vec![2]);
    assert_eq!(stripped.retained_track_ids, vec![1]);
    assert!(stripped.unreferenced_media_retained);
    assert_eq!(
        apple::media_payloads(&video).unwrap(),
        apple::media_payloads(&stripped.bytes).unwrap()
    );
    let parsed = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &stripped.bytes,
            presentation: None,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(parsed
        .agtm_at(MediaTime::new(0, 1).unwrap())
        .unwrap()
        .is_empty());
    assert!(remux::remove_metadata_tracks(&video, &[1]).is_err());
    assert!(remux::remove_metadata_tracks(&video, &[2, 2]).is_err());
    assert_eq!(
        remux::remove_metadata_tracks(&video, &[]).unwrap().bytes,
        video
    );
}
