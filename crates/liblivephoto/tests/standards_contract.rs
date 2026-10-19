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

fn replace_meta_child(still: &[u8], kind: &[u8; 4], replacement: &[u8]) -> Vec<u8> {
    let top = parse_boxes(still, 0..still.len()).unwrap();
    let meta = top.iter().find(|b| b.kind.as_bytes() == b"meta").unwrap();
    let children = parse_boxes(still, meta.data_start + 4..meta.data_end).unwrap();
    let mut payload = vec![0; 4];
    for child in children {
        payload.extend_from_slice(if child.kind.as_bytes() == kind {
            replacement
        } else {
            &still[child.box_range()]
        });
    }
    [
        still[top[0].box_range()].to_vec(),
        common::boxed(b"meta", &payload),
    ]
    .concat()
}

fn with_properties(properties: &[Vec<u8>]) -> Vec<u8> {
    let iprp = common::boxed(
        b"iprp",
        &[
            common::boxed(b"ipco", &properties.concat()),
            make_ipma_box(
                0,
                0,
                &[IpmaEntry {
                    item_id: 1,
                    associations: (1..=properties.len())
                        .map(|i| IpmaAssociation {
                            property_index: i as u16,
                            essential: true,
                        })
                        .collect(),
                }],
            )
            .unwrap(),
        ]
        .concat(),
    );
    replace_meta_child(&common::heif(b"avif"), b"iprp", &iprp)
}

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
