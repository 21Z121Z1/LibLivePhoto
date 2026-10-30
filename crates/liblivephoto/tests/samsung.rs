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
fn catalogue_uses_explicit_record_indices_without_inventing_a_primary_or_role() {
    let first = common::movie(1000, 1);
    let second = common::movie(2000, 1);
    for primaries in 0..=2 {
        let mut records = vec![
            (0x0a33, "MotionPhoto_AutoPlay", first.clone()),
            (0x1234, "Unspecified", b"opaque\0unknown".to_vec()),
        ];
        for _ in 0..primaries {
            records.push((0x0a30, "MotionPhoto_Data", second.clone()));
        }
        let input = common::sef(&records);
        let before = input.clone();
        let inventory = inspect_samsung_videos(&input).unwrap().unwrap();
        assert_eq!(inventory.videos.len(), primaries + 1);
        assert!(inventory.diagnostics.is_empty());
        assert_eq!(inventory.select(Some(0)).unwrap().bytes, first);
        assert!(inventory.select(Some(1)).is_err());
        assert!(inventory.select(Some(usize::MAX)).is_err());
        match primaries {
            0 => assert!(matches!(
                inventory.select(None),
                Err(MotionPhotoReadError::Unrecognized)
            )),
            1 => assert_eq!(inventory.select(None).unwrap().record_index, 2),
            _ => {
                assert!(matches!(
                    inventory.select(None),
                    Err(MotionPhotoReadError::AmbiguousRecovery)
                ));
                assert!(matches!(
                    inspect_samsung_metadata(&input),
                    Err(MotionPhotoReadError::AmbiguousRecovery)
                ));
                assert!(
                    MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible())
                        .is_err()
                );
                assert_eq!(inventory.select(Some(2)).unwrap().bytes, second);
                assert_eq!(inventory.select(Some(3)).unwrap().bytes, second);
            }
        }
        for (i, (_, name, payload)) in records.iter().enumerate() {
            assert_eq!(inventory.record_payload(i).unwrap(), payload);
            let raw = inventory.raw_record(i).unwrap();
            assert_eq!(&raw[8..8 + name.len()], name.as_bytes());
            assert!(raw.ends_with(payload));
        }
        assert_eq!(input, before);
    }
}

#[test]
fn catalogue_reports_invalid_candidates_keeps_opaque_bytes_and_never_falls_back() {
    let good = common::movie(1000, 1);
    let bad = [
        b"mpv2".to_vec(),
        u32::MAX.to_be_bytes().to_vec(),
        10u32.to_be_bytes().to_vec(),
    ]
    .concat();
    let ftyp_only = common::boxed(b"ftyp", b"mp42\0\0\0\0mp42");
    let input = common::sef(&[
        (0x0a30, "MotionPhoto_Data", bad.clone()),
        (0x0a33, "MotionPhoto_AutoPlay", good.clone()),
        (0x3456, "UnspecifiedVideo", ftyp_only.clone()),
    ]);
    let inventory = inspect_samsung_videos(&input).unwrap().unwrap();
    assert_eq!(inventory.videos.len(), 1);
    assert_eq!(inventory.select(Some(1)).unwrap().bytes, good);
    assert!(inventory.select(None).is_err());
    assert_eq!(
        inventory
            .diagnostics
            .iter()
            .map(|d| d.record_index)
            .collect::<Vec<_>>(),
        [0, 2]
    );
    assert!(inventory.diagnostics.iter().all(|d| !d.message.is_empty()));
    assert_eq!(inventory.record_payload(0).unwrap(), bad);
    assert_eq!(inventory.record_payload(2).unwrap(), ftyp_only);
    assert!(inspect_samsung_metadata(&input).is_err());
}

#[test]
fn catalogue_rejects_directory_aliases_and_has_a_distinct_candidate_budget() {
    let valid = sample();
    let size =
        u32::from_le_bytes(valid[valid.len() - 8..valid.len() - 4].try_into().unwrap()) as usize;
    let dir = valid.len() - 8 - size;
    let mut bad = valid.clone();
    bad[dir + 24..dir + 36].copy_from_slice(&valid[dir + 12..dir + 24]);
    assert!(inspect_samsung_videos(&bad).is_err());
    let records = vec![(0x0a33, "OpaqueVideo", common::movie(1000, 1)); 65];
    assert!(matches!(
        inspect_samsung_videos(&common::sef(&records)),
        Err(MotionPhotoReadError::InputLimit)
    ));
    assert_eq!(
        inspect_samsung_videos(&common::sef(&records[..64]))
            .unwrap()
            .unwrap()
            .videos
            .len(),
        64
    );
}

#[test]
fn catalogue_extraction_checks_caller_mutated_ranges_without_truncation_or_panics() {
    let input = sample();
    let mut inventory = inspect_samsung_videos(&input).unwrap().unwrap();
    for range in [
        ByteRange {
            lower_bound: 5,
            upper_bound: 4,
        },
        ByteRange {
            lower_bound: 0,
            upper_bound: u64::MAX,
        },
        ByteRange {
            lower_bound: u64::MAX,
            upper_bound: u64::MAX,
        },
    ] {
        inventory.records[0].payload = range;
        inventory.records[0].raw = range;
        assert!(inventory.record_payload(0).is_err());
        assert!(inventory.raw_record(0).is_err());
    }
}
#[test]
fn catalogue_retains_pointer_aliases_but_counts_unique_movie_ranges_for_its_budget() {
    let still = common::heif(b"heic");
    let movie = common::movie(1000, 1);
    let pointer = [
        b"mpv2".to_vec(),
        ((still.len() + 8) as u32).to_be_bytes().to_vec(),
        (movie.len() as u32).to_be_bytes().to_vec(),
    ]
    .concat();
    let records = vec![(0x0a30, "MotionPhoto_Data", pointer); 65];
    let sef = common::sef(&records);
    let input = [
        still,
        common::boxed(b"mpvd", &movie),
        common::boxed(b"sefd", &sef[4..]),
    ]
    .concat();
    let inventory = inspect_samsung_videos(&input).unwrap().unwrap();
    assert_eq!(inventory.videos.len(), 65);
    assert!(inventory.diagnostics.is_empty());
    assert!(matches!(
        inventory.select(None),
        Err(MotionPhotoReadError::AmbiguousRecovery)
    ));
    assert!(inventory
        .videos
        .iter()
        .all(|v| v.extent == inventory.videos[0].extent && v.bytes == movie));
}

#[test]
fn inventory_projection_obeys_the_samsung_provider_filter_and_does_not_declare_a_movie() {
    let input = common::sef(&[(0x0a33, "MotionPhoto_AutoPlay", common::movie(1000, 1))]);
    let metadata = MetadataEngine::new()
        .parse(&input, &HandlerOptions::default())
        .unwrap();
    assert!(!metadata.verified_motion);
    let catalogue = metadata
        .decoded
        .iter()
        .find(|d| d.provider == "samsung_video_inventory")
        .unwrap();
    assert!(!catalogue.motion_declaration);
    assert!(catalogue.video_extents.is_empty());
    let metadata = MetadataEngine::new()
        .parse(
            &input,
            &HandlerOptions {
                disable_third_party: false,
                enabled_third_party: vec!["vivo_media".into()],
            },
        )
        .unwrap();
    assert!(!metadata
        .decoded
        .iter()
        .any(|d| d.provider.starts_with("samsung")));
    let metadata = MetadataEngine::new()
        .parse(
            &input,
            &HandlerOptions {
                disable_third_party: false,
                enabled_third_party: vec!["samsung_video_inventory".into()],
            },
        )
        .unwrap();
    assert!(metadata
        .decoded
        .iter()
        .any(|d| d.provider == "samsung_video_inventory"));
    assert!(!metadata.decoded.iter().any(|d| d.provider == "samsung_sef"));
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
            let inventory = inspect_samsung_videos(&input).unwrap().unwrap();
            assert_eq!(inventory.select(None).unwrap().bytes, movie);
            assert_eq!(inventory.select(Some(0)).unwrap().extent, m.video.unwrap());
            let p =
                MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
            assert_eq!(p.motion_video_bytes().unwrap(), movie);
            assert_eq!(p.asset().dialect, VendorDialect::SamsungSef);
        }
    }
}
