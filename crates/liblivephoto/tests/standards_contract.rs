mod common;
use liblivephoto::*;
use liblivephoto_format::{isobmff::*, FourCC};

#[test]
fn primary_length_omission_is_supported_but_positive_length_is_a_profile_limit() {
    let movie = common::movie(1000, 1);
    let original = common::xmp(movie.len(), 500_000, "");
    for xml in [
        original.clone(),
        original.replacen(" Item:Length=\"0\"", "", 1),
    ] {
        let input = [common::jpeg(&xml), movie.clone()].concat();
        let photo = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
        assert_eq!(photo.motion_video_bytes().unwrap(), movie);
    }
    let positive = original.replacen("Item:Length=\"0\"", "Item:Length=\"1\"", 1);
    let input = [common::jpeg(&positive), movie].concat();
    // The specification expects zero but does not make it a universal MUST.
    // This is our supported declaration subset, not a full conformance verdict.
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
}

#[test]
fn directory_rewrite_emits_padding_only_on_primary_and_refuses_secondary_padding() {
    let doc = XmpDocument::motion("image/jpeg", "video/mp4", 100, MediaTime::new(1, 2)).unwrap();
    let mut fields = doc.fields().clone();
    fields.directory[0].padding = 12;
    let rewritten = doc.rewrite(&fields).unwrap();
    let xml = String::from_utf8_lossy(rewritten.as_bytes());
    assert_eq!(xml.matches("Item:Padding=").count(), 1);
    assert_eq!(rewritten.fields().directory[0].padding, 12);
    assert_eq!(rewritten.fields().directory[1].padding, 0);
    assert!(rewritten.validate_motion().is_ok());
    fields.directory[1].padding = 1;
    assert!(doc.rewrite(&fields).is_err());
    assert_eq!(doc.fields().directory[0].padding, 0);
}

#[test]
fn directory_padding_is_preserved_and_secondary_nonzero_padding_is_rejected() {
    let movie = common::movie(1000, 1);
    let xml = common::xmp(movie.len(), 500_000, "").replacen(
        "Item:Length=\"0\"",
        "Item:Length=\"0\" Item:Padding=\"7\"",
        1,
    );
    let input = [common::jpeg(&xml), vec![0xa5; 7], movie.clone()].concat();
    let photo = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    assert_eq!(photo.motion_video_bytes().unwrap(), movie);
    assert_eq!(
        photo
            .plan(TargetProfile::PreserveSource, WritePolicy::default())
            .execute()
            .unwrap()
            .files[0]
            .bytes,
        input
    );
    let bad = xml.replace(
        "Item:Semantic=\"MotionPhoto\"",
        "Item:Semantic=\"MotionPhoto\" Item:Padding=\"1\"",
    );
    let invalid = [common::jpeg(&bad), movie].concat();
    assert!(MotionPhoto::parse(Input::SingleFile(&invalid), ParseOptions::strict()).is_err());
}

use common::{
    heif_with_properties as with_properties, replace_heif_meta_child as replace_meta_child,
};

#[test]
fn heif_mirror_axes_and_order_match_asymmetric_coordinate_landmarks() {
    // Coordinates are centered x-right/y-down. Axis0 exchanges top/bottom,
    // axis1 left/right; composition follows item-property association order.
    for (properties, expected) in [
        (vec![common::boxed(b"imir", &[0])], [1, 0, 0, -1]),
        (vec![common::boxed(b"imir", &[1])], [-1, 0, 0, 1]),
        (
            vec![make_irot_box(1).unwrap(), common::boxed(b"imir", &[0])],
            [0, 1, 1, 0],
        ),
        (
            vec![common::boxed(b"imir", &[0]), make_irot_box(1).unwrap()],
            [0, -1, -1, 0],
        ),
    ] {
        let mut boxes = vec![make_ispe_box(2, 3).unwrap()];
        boxes.extend(properties);
        let still = with_properties(&boxes);
        let movie = common::movie(1000, 1);
        let photo = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: None,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let transform = photo.still_metadata().unwrap().heif_transform.unwrap();
        assert_eq!(transform.matrix, expected);
        let output = photo
            .plan(TargetProfile::AndroidAvif, WritePolicy::default())
            .execute()
            .unwrap();
        let parsed = MotionPhoto::parse(
            Input::SingleFile(&output.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(
            parsed.still_metadata().unwrap().heif_transform,
            Some(transform)
        );
    }
    for (kind, payload) in [(b"imir", vec![2]), (b"irot", vec![4]), (b"imir", vec![])] {
        let still = with_properties(&[make_ispe_box(2, 3).unwrap(), common::boxed(kind, &payload)]);
        let movie = common::movie(1000, 1);
        let photo = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: None,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        assert!(matches!(
            photo.still_metadata(),
            Err(MotionPhotoReadError::UnsupportedLayout(_))
        ));
        assert!(!photo
            .plan(TargetProfile::AndroidAvif, WritePolicy::default())
            .can_execute());
    }
}

#[test]
fn heif_extent_resolution_supports_methods_zero_and_one_but_blocks_other_sources() {
    let still = common::heif(b"avif");
    let top = parse_boxes(&still, 0..still.len()).unwrap();
    let parsed = parse_meta_box(&still, &top[1]).unwrap();
    let idat_start = parsed.idat.as_ref().unwrap().data_start as u64;
    let mut entry = parsed.iloc.entries[0].clone();
    let movie = common::movie(1000, 1);
    for (method, external, allowed) in [(0, 0, true), (1, 0, true), (2, 0, false), (1, 1, false)] {
        entry.construction_method = method;
        entry.data_reference_index = external;
        entry.extents[0].offset = if method == 0 { idat_start } else { 0 };
        let iloc = make_iloc_box(1, 4, 4, 0, 0, &[entry.clone()]).unwrap();
        let input = replace_meta_child(&still, b"iloc", &iloc);
        let parsed = MotionPhoto::parse(
            Input::Parts {
                still: &input,
                movie: &movie,
                presentation: MediaTime::new(1, 2),
            },
            ParseOptions::strict(),
        );
        if allowed {
            let source = parsed.unwrap();
            let output = source
                .plan(TargetProfile::AndroidAvif, WritePolicy::default())
                .execute()
                .unwrap();
            let after = MotionPhoto::parse(
                Input::SingleFile(&output.files[0].bytes),
                ParseOptions::strict(),
            )
            .unwrap();
            let item = after
                .resources()
                .find(|r| r.container_item_id == Some(1))
                .unwrap();
            let mut bytes = Vec::new();
            after.extract(item, &mut bytes).unwrap();
            assert_eq!(bytes, b"synthetic-encoded-item");
        } else {
            assert!(parsed.is_err());
        }
    }
}

#[test]
fn heif_motion_photo_rejects_zero_sized_mpvd_even_when_generic_bmff_allows_it() {
    let still = common::heif(b"avif");
    let movie = common::movie(1000, 1);
    let source = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: MediaTime::new(1, 2),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let mut encoded = source
        .plan(TargetProfile::AndroidAvif, WritePolicy::default())
        .execute()
        .unwrap()
        .files
        .remove(0)
        .bytes;
    let mpvd = parse_boxes(&encoded, 0..encoded.len())
        .unwrap()
        .into_iter()
        .find(|b| b.kind == FourCC::new(*b"mpvd"))
        .unwrap();
    encoded[mpvd.box_start..mpvd.box_start + 4].fill(0);
    assert!(parse_boxes(&encoded, 0..encoded.len()).is_ok());
    assert!(MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).is_err());
}

#[test]
fn heif_idat_extents_cannot_escape_into_other_boxes_that_fit_the_file() {
    let still = common::heif(b"avif");
    let top = parse_boxes(&still, 0..still.len()).unwrap();
    let parsed = parse_meta_box(&still, &top[1]).unwrap();
    let mut entry = parsed.iloc.entries[0].clone();
    entry.extents[0].offset = entry.extents[0].length - 1;
    entry.extents[0].length = 2;
    let iloc = make_iloc_box(1, 4, 4, 0, 0, &[entry]).unwrap();
    let input = [
        replace_meta_child(&still, b"iloc", &iloc),
        common::boxed(b"free", &[0xa5; 64]),
    ]
    .concat();
    let movie = common::movie(1000, 1);
    assert!(MotionPhoto::parse(
        Input::Parts {
            still: &input,
            movie: &movie,
            presentation: None
        },
        ParseOptions::strict()
    )
    .is_err());
}

fn nclx(primaries: u16, transfer: u16) -> Vec<u8> {
    [
        b"nclx".to_vec(),
        primaries.to_be_bytes().to_vec(),
        transfer.to_be_bytes().to_vec(),
        9u16.to_be_bytes().to_vec(),
        vec![0x80],
    ]
    .concat()
}
fn colors(values: &[Vec<u8>]) -> Vec<u8> {
    let mut properties = vec![make_ispe_box(2, 3).unwrap()];
    properties.extend(values.iter().map(|p| common::boxed(b"colr", p)));
    with_properties(&properties)
}

#[test]
fn heif_color_count_and_legal_pairs_are_checked_without_changing_color_or_coded_bytes() {
    let icc = b"rICCopaque-profile".to_vec();
    let prof = b"profother-opaque-profile".to_vec();
    for declarations in [
        vec![],
        vec![nclx(9, 16)],
        vec![icc.clone()],
        vec![prof.clone()],
        vec![icc, nclx(2, 2)],
        vec![nclx(2, 2), prof],
    ] {
        let still = colors(&declarations);
        let original = still.clone();
        let report = validate_heif_color_information(&still).unwrap();
        assert!(report.checked_rules_satisfied(), "{report:?}");
        let movie = common::movie(1000, 1);
        let photo = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: MediaTime::new(1, 2),
            },
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(photo.heif_color_conformance().unwrap(), report);
        let before = photo.still_metadata().unwrap();
        let output = photo
            .plan(TargetProfile::AndroidAvif, WritePolicy::default())
            .execute()
            .unwrap();
        let parsed = MotionPhoto::parse(
            Input::SingleFile(&output.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(parsed.still_metadata().unwrap(), before);
        assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
        let image = parsed
            .resources()
            .find(|r| r.container_item_id == Some(1))
            .unwrap();
        let mut bytes = Vec::new();
        parsed.extract(image, &mut bytes).unwrap();
        assert_eq!(bytes, b"synthetic-encoded-item");
        assert_eq!(parsed.heif_color_conformance().unwrap(), report);
        assert_eq!(still, original);
    }
}

#[test]
fn heif_color_violations_are_explicit_and_never_repaired_by_preservation_writers() {
    let icc = b"rICCopaque".to_vec();
    let prof = b"profopaque".to_vec();
    for (declarations, rule) in [
        (
            vec![icc.clone(), prof.clone(), nclx(2, 2)],
            "H05.colr-count",
        ),
        (vec![icc.clone(), prof], "H05.colr-pair"),
        (vec![nclx(2, 2), nclx(2, 2)], "H05.colr-pair"),
        (vec![icc.clone(), nclx(1, 2)], "H05.paired-nclx-unspecified"),
        (vec![icc.clone(), nclx(2, 1)], "H05.paired-nclx-unspecified"),
        (vec![icc, nclx(9, 16)], "H05.paired-nclx-unspecified"),
    ] {
        let still = colors(&declarations);
        let original = still.clone();
        let report = validate_heif_color_information(&still).unwrap();
        assert!(!report.checked_rules_satisfied());
        assert!(report
            .issues
            .iter()
            .all(|i| i.kind == FormatIssueKind::Violation));
        assert!(report
            .issues
            .iter()
            .any(|i| i.rule == rule && i.item_id == Some(1)));
        let movie = common::movie(1000, 1);
        let photo = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: MediaTime::new(1, 2),
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let output = photo
            .plan(TargetProfile::AndroidAvif, WritePolicy::default())
            .execute()
            .unwrap();
        let after = MotionPhoto::parse(
            Input::SingleFile(&output.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(
            photo.still_metadata().unwrap(),
            after.still_metadata().unwrap()
        );
        assert_eq!(after.heif_color_conformance().unwrap(), report);
        assert_eq!(still, original);
    }
}

#[test]
fn unknown_color_syntax_and_duplicate_associations_are_unsupported_not_guessed() {
    for payload in [
        b"zzzzopaque".to_vec(),
        b"nclc\0\x09\0\x10\0\x09".to_vec(),
        b"nclx\0\x02".to_vec(),
        vec![],
    ] {
        let still = colors(&[payload]);
        let before = still.clone();
        let report = validate_heif_color_information(&still).unwrap();
        assert!(!report.checked_rules_satisfied());
        assert!(report
            .issues
            .iter()
            .all(|i| i.kind == FormatIssueKind::Unsupported));
        assert_eq!(still, before);
    }
    let source = colors(&[nclx(9, 16)]);
    let row = IpmaEntry {
        item_id: 1,
        associations: vec![
            IpmaAssociation {
                property_index: 2,
                essential: false
            };
            2
        ],
    };
    let iprp = common::boxed(
        b"iprp",
        &[
            common::boxed(
                b"ipco",
                &[
                    make_ispe_box(2, 3).unwrap(),
                    common::boxed(b"colr", &nclx(9, 16)),
                ]
                .concat(),
            ),
            make_ipma_box(0, 0, &[row]).unwrap(),
        ]
        .concat(),
    );
    let still = replace_meta_child(&source, b"iprp", &iprp);
    let report = validate_heif_color_information(&still).unwrap();
    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == "H05.association-layout" && i.kind == FormatIssueKind::Unsupported));
    assert!(!report
        .issues
        .iter()
        .any(|i| i.kind == FormatIssueKind::Violation));
    let iinf = make_iinf_box(1, &[make_infe_box(1, FourCC::new(*b"zzzz"), 0).unwrap()]).unwrap();
    let unknown = replace_meta_child(&source, b"iinf", &iinf);
    let report = validate_heif_color_information(&unknown).unwrap();
    assert!(report
        .issues
        .iter()
        .all(|i| i.kind == FormatIssueKind::Unsupported));
    assert_eq!(report.issues[0].rule, "H05.item-kind");
}

#[test]
fn auxiliary_image_colors_are_merged_across_ipma_boxes_and_checked_per_item() {
    for (primaries, valid) in [(2, true), (9, false)] {
        let properties = [
            make_ispe_box(2, 3).unwrap(),
            common::boxed(b"colr", &nclx(9, 16)),
            common::boxed(b"colr", b"profopaque"),
            common::boxed(b"colr", &nclx(primaries, 2)),
        ];
        let rows = |id, indices: &[u16]| IpmaEntry {
            item_id: id,
            associations: indices
                .iter()
                .map(|i| IpmaAssociation {
                    property_index: *i,
                    essential: false,
                })
                .collect(),
        };
        let iprp = common::boxed(
            b"iprp",
            &[
                common::boxed(b"ipco", &properties.concat()),
                make_ipma_box(0, 0, &[rows(1, &[1, 2]), rows(2, &[1, 3])]).unwrap(),
                make_ipma_box(0, 0, &[rows(2, &[4])]).unwrap(),
            ]
            .concat(),
        );
        let still = replace_meta_child(&common::heif(b"avif"), b"iprp", &iprp);
        let iinf = make_iinf_box(
            1,
            &[
                make_infe_box(1, FourCC::new(*b"av01"), 0).unwrap(),
                make_infe_box(2, FourCC::new(*b"av01"), 0).unwrap(),
            ],
        )
        .unwrap();
        let still = replace_meta_child(&still, b"iinf", &iinf);
        let entry = |id, offset, length| IlocEntry {
            item_id: id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length,
            }],
        };
        let iloc = make_iloc_box(1, 4, 4, 0, 0, &[entry(1, 0, 22), entry(2, 22, 22)]).unwrap();
        let still = replace_meta_child(&still, b"iloc", &iloc);
        let still = replace_meta_child(
            &still,
            b"idat",
            &common::boxed(b"idat", b"synthetic-encoded-itemauxiliary-encoded-item"),
        );
        let report = validate_heif_color_information(&still).unwrap();
        assert_eq!(report.checked_rules_satisfied(), valid);
        assert!(report.issues.iter().all(|i| i.item_id == Some(2)));
        if !valid {
            assert_eq!(report.issues[0].rule, "H05.paired-nclx-unspecified");
        }
        let movie = common::movie(1000, 1);
        let photo = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &movie,
                presentation: MediaTime::new(1, 2),
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let output = photo
            .plan(TargetProfile::AndroidAvif, WritePolicy::default())
            .execute()
            .unwrap();
        let after = MotionPhoto::parse(
            Input::SingleFile(&output.files[0].bytes),
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(after.heif_color_conformance().unwrap(), report);
        for (id, expected) in [
            (1, b"synthetic-encoded-item".as_slice()),
            (2, b"auxiliary-encoded-item".as_slice()),
        ] {
            let resource = after
                .resources()
                .find(|r| r.container_item_id == Some(id))
                .unwrap();
            let mut bytes = Vec::new();
            after.extract(resource, &mut bytes).unwrap();
            assert_eq!(bytes, expected);
        }
    }
}

#[test]
fn color_validation_preflights_rows_and_counts_repeated_payloads_across_items() {
    let properties = [
        make_ispe_box(2, 3).unwrap(),
        common::boxed(b"colr", &[b"prof".to_vec(), vec![0x5a; 8192]].concat()),
    ];
    for (rows, kind) in [(4097, 0), (4096, 1)] {
        let row = IpmaEntry {
            item_id: 1,
            associations: if kind == 0 {
                vec![]
            } else {
                vec![IpmaAssociation {
                    property_index: 2,
                    essential: false,
                }]
            },
        };
        let iprp = common::boxed(
            b"iprp",
            &[
                common::boxed(b"ipco", &properties.concat()),
                make_ipma_box(0, 0, &vec![row; rows]).unwrap(),
            ]
            .concat(),
        );
        let still = replace_meta_child(&common::heif(b"avif"), b"iprp", &iprp);
        assert!(matches!(
            validate_heif_color_information(&still),
            Err(MotionPhotoReadError::InputLimit)
        ));
    }
}
