mod common;
use liblivephoto::*;
fn sample() -> Vec<u8> {
    common::sef(&[
        (0x0a33, "MotionPhoto_AutoPlay", common::movie(1000, 1)),
        (0x1234, "Unspecified", b"opaque".to_vec()),
        (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
    ])
}
#[test]
fn sef_roles_are_not_inferred_from_vendor_names_and_payloads_survive_round_trip() {
    let input = sample();
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
    assert_eq!(p.asset().dialect, VendorDialect::SamsungSef);
    assert_eq!(p.presentation_time(), None);
    let r = p
        .resources()
        .find(|r| r.vendor_role.as_deref() == Some("MotionPhoto_AutoPlay"))
        .unwrap();
    assert_eq!(r.role, ResourceRole::UnknownVideo);
    assert_eq!(r.relationship.as_ref().unwrap().target, p.motion_video().id);
    assert!(p
        .asset()
        .containers
        .iter()
        .any(|c| c.resource == r.id && c.tracks.len() == 1));
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
    let policy = WritePolicy {
        unmapped: UnmappedPolicy::PreserveSidecars,
        ..WritePolicy::default()
    };
    let output = p
        .plan(TargetProfile::SamsungJpegSef, policy)
        .execute()
        .unwrap();
    let new = MotionPhoto::parse(
        Input::SingleFile(&output.files[0].bytes),
        ParseOptions::compatible(),
    )
    .unwrap();
    assert_eq!(
        new.motion_video_bytes().unwrap(),
        p.motion_video_bytes().unwrap()
    );
    for name in ["MotionPhoto_AutoPlay", "Unspecified"] {
        let old = p
            .resources()
            .find(|r| r.vendor_role.as_deref() == Some(name))
            .unwrap();
        let r = new
            .resources()
            .find(|r| r.vendor_role.as_deref() == Some(name))
            .unwrap();
        let mut a = Vec::new();
        let mut b = Vec::new();
        p.extract(old, &mut a).unwrap();
        new.extract(r, &mut b).unwrap();
        assert_eq!(a, b);
    }
}
#[test]
fn malformed_sef_lengths_offsets_overlap_and_duplicate_primary_fail() {
    let valid = sample();
    let size =
        u32::from_le_bytes(valid[valid.len() - 8..valid.len() - 4].try_into().unwrap()) as usize;
    let start = valid.len() - 8 - size;
    for (offset, bytes) in [
        (valid.len() - 8, vec![255; 4]),
        (start + 8, vec![255; 4]),
        (start + 16, vec![255; 4]),
        (start + 20, vec![255; 4]),
        (start + 12, vec![1, 0, 0, 0]),
    ] {
        let mut d = valid.clone();
        d[offset..offset + bytes.len()].copy_from_slice(&bytes);
        assert!(MotionPhoto::parse(Input::SingleFile(&d), ParseOptions::compatible()).is_err());
    }
    let duplicated = common::sef(&[
        (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
        (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
    ]);
    assert!(
        MotionPhoto::parse(Input::SingleFile(&duplicated), ParseOptions::compatible()).is_err()
    );
}

#[test]
fn sef_pointer_cannot_alias_an_unknown_record_with_the_primary_type_number() {
    let video = common::movie(1000, 1);
    let mut pointer = b"mpv2".to_vec();
    pointer.extend_from_slice(&0_u32.to_be_bytes());
    pointer.extend_from_slice(&(video.len() as u32).to_be_bytes());
    let mut input = common::sef(&[
        (0x0a30, "MotionPhoto_Data", pointer),
        (0x0a30, "UnspecifiedVideo", video),
    ]);
    let video_start = input.windows(4).position(|b| b == b"ftyp").unwrap() - 4;
    let pointer_start = 4 + 8 + "MotionPhoto_Data".len();
    input[pointer_start + 4..pointer_start + 8]
        .copy_from_slice(&(video_start as u32).to_be_bytes());
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).is_err());
}
#[test]
fn metadata_only_sef_and_both_inline_mpv_headers_are_read_separately_from_movie_validation() {
    let metadata = common::sef(&[(0x1234, "Opaque", b"metadata".to_vec())]);
    let m = inspect_samsung_metadata(&metadata).unwrap().unwrap();
    assert!(m.video.is_none());
    assert_eq!(m.records[0].name, "Opaque");
    assert!(!MetadataEngine::new()
        .parse(&metadata, &HandlerOptions::default())
        .unwrap()
        .declares_motion(&HandlerOptions::default()));
    let movie = common::movie(1000, 1);
    for magic in [b"mpv2", b"mpv3"] {
        let payload = [
            magic.to_vec(),
            100u32.to_be_bytes().to_vec(),
            (movie.len() as u32).to_be_bytes().to_vec(),
            movie.clone(),
        ]
        .concat();
        let input = common::sef(&[(0x0a30, "MotionPhoto_Data", payload)]);
        let m = inspect_samsung_metadata(&input).unwrap().unwrap();
        assert_eq!(m.motion_photo_data_magic, Some(*magic));
        let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
        assert_eq!(p.motion_video_bytes().unwrap(), movie);
    }
}
#[test]
fn heif_sefd_before_or_after_mpvd_supports_mpv3_pointer_and_empty_stub() {
    let still = common::heif(b"heic");
    let movie = common::movie(1000, 1);
    for before in [false, true] {
        for stub in [false, true] {
            let placeholder = if stub {
                vec![]
            } else {
                [b"mpv3".to_vec(), vec![0; 8]].concat()
            };
            let temporary = common::sef(&[(0x0a30, "MotionPhoto_Data", placeholder)]);
            let mut payload = temporary[4..].to_vec();
            let sefd_size = payload.len() + 8;
            let movie_start = still.len() + if before { sefd_size } else { 0 } + 8;
            if !stub {
                let p = 8 + "MotionPhoto_Data".len();
                payload[p + 4..p + 8].copy_from_slice(&(movie_start as u32).to_be_bytes());
                payload[p + 8..p + 12].copy_from_slice(&(movie.len() as u32).to_be_bytes());
            }
            let sefd = common::boxed(b"sefd", &payload);
            let mpvd = common::boxed(b"mpvd", &movie);
            let input = if before {
                [still.clone(), sefd, mpvd].concat()
            } else {
                [still.clone(), mpvd, sefd].concat()
            };
            let m = inspect_samsung_metadata(&input).unwrap().unwrap();
            assert_eq!(m.video.unwrap().lower_bound, movie_start as u64);
            let p =
                MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
            assert_eq!(p.motion_video_bytes().unwrap(), movie);
            assert_eq!(p.asset().dialect, VendorDialect::SamsungSef);
        }
    }
}
