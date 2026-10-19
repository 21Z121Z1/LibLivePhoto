mod common;
use liblivephoto::*;
use liblivephoto_format::{isobmff::*, FourCC};

fn oriented_jpeg(value: u16) -> Vec<u8> {
    let tiff = [
        b"II*\0\x08\0\0\0".to_vec(),
        vec![1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0],
        value.to_le_bytes().to_vec(),
        vec![0; 6],
    ]
    .concat();
    let exif = [b"Exif\0\0".to_vec(), tiff].concat();
    let icc = b"ICC_PROFILE\0\x01\x01opaque-profile";
    [
        vec![255, 216, 255, 225],
        ((exif.len() + 2) as u16).to_be_bytes().to_vec(),
        exif,
        vec![255, 226],
        ((icc.len() + 2) as u16).to_be_bytes().to_vec(),
        icc.to_vec(),
        vec![255, 217],
    ]
    .concat()
}

#[test]
fn all_exif_orientations_and_icc_survive_android_and_existing_apple_paths() {
    let movie = common::movie(1000, 1);
    for value in 0..=8 {
        let still = oriented_jpeg(value);
        let source = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: MediaTime::new(1, 4),
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let before = source.still_metadata().unwrap();
        assert_eq!(before.exif_orientation, Some(value));
        assert_eq!(before.exif_transform.is_none(), value == 0);
        assert!(
            matches!(&before.color[0],ColorDescription::Icc { profile,.. } if profile == b"opaque-profile")
        );
        let google = source
            .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
            .execute()
            .unwrap();
        let google = MotionPhoto::parse(
            Input::SingleFile(&google.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(google.still_metadata().unwrap(), before);
        let apple = source
            .plan(
                TargetProfile::ApplePair,
                WritePolicy {
                    pairing: PairingPolicy::Set("DISPLAY-CONTRACT".into()),
                    unmapped: UnmappedPolicy::PreserveSidecars,
                },
            )
            .execute()
            .unwrap();
        let apple = MotionPhoto::parse(
            Input::ApplePair {
                still: &apple.files[0].bytes,
                movie: &apple.files[1].bytes,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(apple.still_metadata().unwrap(), before);
    }
    assert_eq!(
        OrthogonalTransform::from_exif(6).unwrap().matrix,
        [0, -1, 1, 0]
    );
    assert_eq!(
        OrthogonalTransform::from_exif(7).unwrap().matrix,
        [0, -1, -1, 0]
    );
}

#[test]
fn heif_primary_property_order_hdr_signaling_and_unknown_properties_survive_build() {
    let data = common::heif(b"avif");
    let top = parse_boxes(&data, 0..data.len()).unwrap();
    let meta = &top[1];
    let children = parse_boxes(&data, meta.data_start + 4..meta.data_end).unwrap();
    let mut payload = vec![0; 4];
    let properties = [
        make_ispe_box(2, 3).unwrap(),
        make_irot_box(3).unwrap(),
        common::boxed(b"imir", &[0]),
        common::boxed(b"colr", b"nclx\0\x09\0\x10\0\x09\x80"),
        common::boxed(b"clli", &[0, 1, 0, 2]),
        common::boxed(b"zzzz", b"unknown-essential"),
    ];
    let iprp = common::boxed(
        b"iprp",
        &[
            common::boxed(b"ipco", &properties.concat()),
            make_ipma_box(
                0,
                0,
                &[IpmaEntry {
                    item_id: 1,
                    associations: (1..=6)
                        .map(|i| IpmaAssociation {
                            property_index: i,
                            essential: true,
                        })
                        .collect(),
                }],
            )
            .unwrap(),
        ]
        .concat(),
    );
    for b in children {
        payload.extend_from_slice(if b.kind == FourCC::new(*b"iprp") {
            &iprp
        } else {
            &data[b.box_range()]
        });
    }
    let still = [
        data[top[0].box_range()].to_vec(),
        common::boxed(b"meta", &payload),
    ]
    .concat();
    let movie = common::movie(1000, 1);
    let source = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: None,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let facts = source.still_metadata().unwrap();
    assert_eq!(facts.heif_transform.unwrap().matrix, [0, -1, -1, 0]);
    assert_eq!(facts.color[0].hdr_transfer(), Some(HdrTransfer::Pq));
    assert_eq!(
        facts.primary_properties.last().unwrap().bytes,
        b"unknown-essential"
    );
    let output = source
        .plan(TargetProfile::AndroidAvif, WritePolicy::default())
        .execute()
        .unwrap();
    assert!(output
        .report
        .evidence
        .contains(&PreservationEvidence::StillDisplayMetadataEqual));
    assert!(output
        .report
        .evidence
        .contains(&PreservationEvidence::StillColorMetadataEqual));
    let parsed = MotionPhoto::parse(
        Input::SingleFile(&output.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(facts, parsed.still_metadata().unwrap());
}

fn audio_movie(asc: &[u8], channels: u16) -> Vec<u8> {
    let specific = [vec![5, asc.len() as u8], asc.to_vec()].concat();
    let config = [vec![0x40, 0x15], vec![0; 11], specific].concat();
    let decoder = [vec![4, config.len() as u8], config].concat();
    let es = [vec![0, 1, 0], decoder].concat();
    let esds = common::full(b"esds", &[vec![3, es.len() as u8], es].concat());
    let mut fields = vec![0; 20];
    fields[8..10].copy_from_slice(&channels.to_be_bytes());
    fields[16..20].copy_from_slice(&(48000u32 << 16).to_be_bytes());
    let mut movie =
        common::metadata_movie(b"mp4a", &[fields, esds].concat(), &[(&[1, 2, 3], 2000)], 0);
    let p = movie.windows(4).rposition(|w| w == b"meta").unwrap();
    movie[p..p + 4].copy_from_slice(b"soun");
    movie
}

fn capabilities() -> MediaCapabilities {
    MediaCapabilities {
        video_sample_entries: vec![*b"avc1"],
        audio_sample_entries: vec![*b"mp4a"],
        aac_object_types: vec![2],
        max_audio_channels: 2,
        audio_sample_rates: vec![48000],
    }
}

#[test]
fn explicit_codec_contract_uses_aac_config_and_reports_unknown_or_transcode_requirements() {
    let still = vec![255, 216, 255, 217];
    for (asc, channels, allowed) in [
        (&[0x11, 0x90][..], 2, true),
        (&[0x11, 0xb0][..], 6, false),
        (&[0x12, 0x10][..], 2, false),
        (&[0x11, 0x80][..], 2, false),
    ] {
        let movie = audio_movie(asc, channels);
        let source = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: None,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let descriptions = source.sample_descriptions().unwrap();
        let audio = descriptions
            .iter()
            .find(|d| d.kind == TrackKind::Audio)
            .unwrap()
            .audio
            .as_ref()
            .unwrap();
        assert_eq!(audio.aac.as_ref().unwrap().raw, asc);
        let plan = source.plan_with_capabilities(
            TargetProfile::AndroidJpeg,
            WritePolicy::default(),
            &capabilities(),
        );
        assert_eq!(plan.can_execute(), allowed);
        if allowed {
            plan.execute().unwrap();
        } else {
            assert!(plan.execute().is_err());
            assert!(plan.report().resources.iter().any(|r| matches!(
                r.disposition,
                Disposition::TranscodeRequired | Disposition::Unmapped
            )));
        }
        let mut caps = capabilities();
        caps.video_sample_entries = vec![*b"hvc1"];
        assert!(!source
            .plan_with_capabilities(TargetProfile::AndroidJpeg, WritePolicy::default(), &caps)
            .can_execute());
    }
    let malformed = audio_movie(&[0x11], 2);
    let source = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &malformed,
            presentation: None,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(source.sample_descriptions().is_err());
    assert!(!source
        .plan_with_capabilities(
            TargetProfile::AndroidJpeg,
            WritePolicy::default(),
            &capabilities()
        )
        .can_execute());
}

#[test]
fn repeated_property_associations_cannot_amplify_semantic_extraction_without_bound() {
    let data = common::heif(b"avif");
    let top = parse_boxes(&data, 0..data.len()).unwrap();
    let meta = &top[1];
    let children = parse_boxes(&data, meta.data_start + 4..meta.data_end).unwrap();
    let rows = (0..17)
        .map(|_| IpmaEntry {
            item_id: 1,
            associations: vec![
                IpmaAssociation {
                    property_index: 1,
                    essential: true
                };
                255
            ],
        })
        .collect::<Vec<_>>();
    let properties = common::boxed(
        b"iprp",
        &[
            common::boxed(b"ipco", &common::boxed(b"zzzz", &[])),
            make_ipma_box(0, 0, &rows).unwrap(),
        ]
        .concat(),
    );
    let mut payload = vec![0; 4];
    for b in children {
        payload.extend_from_slice(if b.kind == FourCC::new(*b"iprp") {
            &properties
        } else {
            &data[b.box_range()]
        });
    }
    let still = [
        data[top[0].box_range()].to_vec(),
        common::boxed(b"meta", &payload),
    ]
    .concat();
    let movie = common::movie(1000, 1);
    let p = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: None,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(matches!(
        p.still_metadata(),
        Err(MotionPhotoReadError::InputLimit)
    ));
    assert!(!p
        .plan(TargetProfile::AndroidAvif, WritePolicy::default())
        .can_execute());
    assert_eq!(
        p.plan(TargetProfile::PreserveSource, WritePolicy::default())
            .execute()
            .unwrap()
            .files[0]
            .bytes,
        still
    );
}
