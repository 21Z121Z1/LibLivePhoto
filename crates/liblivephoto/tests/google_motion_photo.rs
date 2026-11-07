//! Byte-level Motion Photo 1.0 writer contracts. These do not certify Photos playback.
mod common;
use liblivephoto::*;
use liblivephoto_format::exif::read_item_payload;
use liblivephoto_format::isobmff::{
    make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box, make_iref_box,
    make_ispe_box, make_pitm_box, parse_boxes, parse_meta_box, IlocEntry, IlocExtent,
    IpmaAssociation, IpmaEntry, IrefEntry, ItemInfo, ParsedMeta,
};
use liblivephoto_format::FourCC;

fn build(still: &[u8], movie: &[u8], target: TargetProfile, relabel: bool) -> ComposedAsset {
    let photo = MotionPhoto::parse(
        Input::Parts {
            still,
            movie,
            presentation: MediaTime::new(13, 16),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let plan = photo.plan(target, WritePolicy::default());
    let plan = if relabel {
        plan.relabel_video_major_brand(*b"mp42").unwrap()
    } else {
        plan
    };
    assert!(plan.can_execute());
    plan.execute().unwrap()
}

fn xmp_of(bytes: &[u8]) -> String {
    let parsed = MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::strict()).unwrap();
    String::from_utf8(parsed.xmp_bytes().unwrap().unwrap().into_owned()).unwrap()
}

fn top_kinds(bytes: &[u8]) -> Vec<[u8; 4]> {
    parse_boxes(bytes, 0..bytes.len())
        .unwrap()
        .into_iter()
        .map(|b| *b.kind.as_bytes())
        .collect()
}

fn mime_infe_version(bytes: &[u8]) -> u8 {
    let mut found = None;
    let mut pos = 0;
    while let Some(rel) = bytes[pos..].windows(4).position(|w| w == b"infe") {
        let at = pos + rel;
        let size = u32::from_be_bytes(bytes[at - 4..at].try_into().unwrap()) as usize;
        let end = at - 4 + size;
        let body = &bytes[at..end];
        if body
            .windows(b"application/rdf+xml\0".len())
            .any(|w| w == b"application/rdf+xml\0")
        {
            found = Some(bytes[at + 4]);
        }
        pos = at + 4;
    }
    found.expect("mime XMP item")
}

#[test]
fn jpeg_app1_directory_matches_motion_photo_1_and_drops_legacy_microvideo() {
    let movie = common::movie(1000, 1);
    let mut still = include_bytes!("data/web/public/demo/cover.jpg").to_vec();
    let packet = [
        b"http://ns.adobe.com/xap/1.0/\0".as_slice(),
        br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/" xmlns:Vendor="urn:llp:test" GCamera:MicroVideo="1" GCamera:MicroVideoVersion="1" GCamera:MicroVideoOffset="99" GCamera:MicroVideoPresentationTimestampUs="5" Vendor:keep="yes"/></rdf:RDF></x:xmpmeta>"#.as_slice(),
    ]
    .concat();
    let mut app1 = vec![0xff, 0xe1];
    app1.extend_from_slice(&((packet.len() + 2) as u16).to_be_bytes());
    app1.extend_from_slice(&packet);
    still.splice(2..2, app1);
    let out = build(&still, &movie, TargetProfile::AndroidJpeg, false);
    let bytes = &out.files[0].bytes;
    assert!(bytes.starts_with(&[0xff, 0xd8, 0xff, 0xe1]));
    assert!(bytes[..80]
        .windows(b"http://ns.adobe.com/xap/1.0/\0".len())
        .any(|w| w == b"http://ns.adobe.com/xap/1.0/\0"));
    let xmp = xmp_of(bytes);
    assert!(xmp.contains("Camera:MotionPhoto=\"1\""));
    assert!(xmp.contains("Camera:MotionPhotoVersion=\"1\""));
    assert!(xmp.contains(&format!("Item:Length=\"{}\"", movie.len())));
    assert!(xmp.contains("Item:Padding=\"0\""));
    assert!(!xmp.contains("MicroVideo"));
    assert!(xmp.contains("Vendor:keep=\"yes\""));
    assert!(
        xmp.find("Camera:MotionPhoto=\"1\"").unwrap() < xmp.find("Vendor:keep=\"yes\"").unwrap()
    );
    let parsed = MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::strict()).unwrap();
    let start = parsed.motion_video().extents[0].lower_bound as usize;
    assert_eq!(bytes.len() - start, movie.len());
    assert_eq!(&bytes[start + 4..start + 8], b"ftyp");
    assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
    assert!(out
        .report
        .evidence
        .contains(&PreservationEvidence::VideoContainerBytesEqual));
}

#[test]
fn heic_and_avif_use_a_mime_xmp_item_and_a_terminal_mpvd() {
    let movie = common::movie(1000, 1);
    for (brand, target, mime) in [
        (*b"heic", TargetProfile::AndroidHeif, "image/heic"),
        (*b"avif", TargetProfile::AndroidAvif, "image/avif"),
        (*b"mif1", TargetProfile::AndroidHeif, "image/heic"),
    ] {
        let out = build(&common::heif(&brand), &movie, target, false);
        let bytes = &out.files[0].bytes;
        let kinds = top_kinds(bytes);
        assert_eq!(
            kinds.iter().filter(|k| **k == *b"meta").count(),
            1,
            "{mime}"
        );
        assert_eq!(kinds.last().copied(), Some(*b"mpvd"), "{mime}");
        assert_eq!(&bytes[8..12], &brand, "{mime}");
        assert_eq!(mime_infe_version(bytes), 2, "{mime}");
        let xmp = xmp_of(bytes);
        assert!(xmp.contains(&format!("Item:Mime=\"{mime}\"")), "{xmp}");
        assert!(xmp.contains("Item:Semantic=\"Primary\""));
        assert!(xmp.contains("Item:Padding=\"8\""));
        assert!(xmp.contains(&format!(
            "Item:Semantic=\"MotionPhoto\" Item:Length=\"{}\"",
            movie.len()
        )));
        assert!(xmp.contains("Camera:MotionPhoto=\"1\""));
        assert!(xmp.contains("Camera:MotionPhotoVersion=\"1\""));
        assert!(!xmp.contains("MicroVideo"));
        let parsed = MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::strict()).unwrap();
        let video = parsed.motion_video().extents[0];
        let mpvd = parse_boxes(bytes, 0..bytes.len())
            .unwrap()
            .into_iter()
            .find(|b| b.kind.as_bytes() == b"mpvd")
            .unwrap();
        assert_eq!(mpvd.data_start as u64, video.lower_bound);
        assert_eq!(mpvd.data_end as u64, video.upper_bound);
        assert_eq!(video.upper_bound, bytes.len() as u64);
        assert_eq!(mpvd.data_start - mpvd.box_start, 8);
        assert_eq!(&bytes[mpvd.data_start + 4..mpvd.data_start + 8], b"ftyp");
        assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
    }
}

#[test]
fn real_heic_still_keeps_heic_major_brand_and_isom_video_until_relabel() {
    let still = include_bytes!("data/web/public/acceptance/still.heic");
    let movie = include_bytes!("data/web/public/demo/motion.mp4");
    let plain = build(still, movie, TargetProfile::AndroidHeif, false);
    let bytes = &plain.files[0].bytes;
    assert_eq!(&bytes[8..12], b"heic");
    let parsed = MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::strict()).unwrap();
    assert_eq!(&parsed.motion_video_bytes().unwrap()[8..12], b"isom");
    assert_eq!(mime_infe_version(bytes), 2);
    let labeled = build(still, movie, TargetProfile::AndroidHeif, true);
    let video = MotionPhoto::parse(
        Input::SingleFile(&labeled.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap()
    .motion_video_bytes()
    .unwrap()
    .to_vec();
    assert_eq!(&video[8..12], b"mp42");
    let mut expected = movie.to_vec();
    expected[8..12].copy_from_slice(b"mp42");
    assert_eq!(video, expected);
    assert!(labeled
        .report
        .resources
        .iter()
        .any(|r| { r.disposition == Disposition::Rewritten && r.reason.contains("mp42") }));
    assert!(!labeled
        .report
        .evidence
        .contains(&PreservationEvidence::VideoContainerBytesEqual));
    let xmp = xmp_of(&labeled.files[0].bytes);
    assert!(xmp.contains("Item:Mime=\"video/mp4\""));
}

#[test]
fn qt_major_brand_is_not_relabeled() {
    let mut movie = common::movie(1000, 1);
    movie[8..12].copy_from_slice(b"qt  ");
    let out = build(
        &common::heif(b"heic"),
        &movie,
        TargetProfile::AndroidHeif,
        true,
    );
    let parsed = MotionPhoto::parse(
        Input::SingleFile(&out.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
    assert!(xmp_of(&out.files[0].bytes).contains("video/quicktime"));
    assert!(out
        .report
        .evidence
        .contains(&PreservationEvidence::VideoContainerBytesEqual));
}

fn mime_infe(item_id: u16) -> Vec<u8> {
    let mut payload = item_id.to_be_bytes().to_vec();
    payload.extend_from_slice(&[0, 0]);
    payload.extend_from_slice(b"mime\0application/rdf+xml\0\0");
    make_full_box(FourCC::new(*b"infe"), 2, 0, &payload).unwrap()
}

fn parsed_meta(bytes: &[u8]) -> ParsedMeta {
    let top = parse_boxes(bytes, 0..bytes.len()).unwrap();
    let meta = top.iter().find(|b| b.kind.as_bytes() == b"meta").unwrap();
    parse_meta_box(bytes, meta).unwrap()
}

fn is_rdf_xml(bytes: &[u8], item: &ItemInfo) -> bool {
    item.item_type == Some(FourCC::new(*b"mime"))
        && bytes[item.box_range.clone()]
            .windows(b"application/rdf+xml\0".len())
            .any(|w| w == b"application/rdf+xml\0")
}

fn payload(bytes: &[u8], parsed: &ParsedMeta, item_id: u32) -> Vec<u8> {
    let entry = parsed
        .iloc
        .entries
        .iter()
        .find(|e| e.item_id == item_id)
        .unwrap();
    read_item_payload(bytes, entry, parsed.idat.as_ref()).unwrap()
}

/// Gain-map XMP is declared before the primary item's XMP, with another item
/// between them. Photos 8.0 keeps whichever rdf+xml infe comes first.
fn heif_with_leading_auxiliary_xmp(aux_xmp: &[u8], primary_xmp: &[u8]) -> Vec<u8> {
    let image = b"synthetic-encoded-item";
    let gain = b"synthetic-gain-map-item";
    let mut offset = 0u64;
    let mut extent = |len: usize| {
        let value = IlocExtent {
            index: None,
            offset,
            length: len as u64,
        };
        offset += len as u64;
        value
    };
    let image_extent = extent(image.len());
    let gain_extent = extent(gain.len());
    let aux_extent = extent(aux_xmp.len());
    let primary_extent = extent(primary_xmp.len());
    let entry = |item_id, extent| IlocEntry {
        item_id,
        construction_method: 1,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![extent],
    };
    let iinf = make_iinf_box(
        1,
        &[
            make_infe_box(1, FourCC::new(*b"hvc1"), 0).unwrap(),
            mime_infe(3),
            make_infe_box(2, FourCC::new(*b"hvc1"), 0).unwrap(),
            mime_infe(4),
        ],
    )
    .unwrap();
    let iloc = make_iloc_box(
        1,
        4,
        4,
        0,
        0,
        &[
            entry(1, image_extent),
            entry(3, aux_extent),
            entry(2, gain_extent),
            entry(4, primary_extent),
        ],
    )
    .unwrap();
    let iref = make_iref_box(
        1,
        &[
            IrefEntry {
                kind: FourCC::new(*b"cdsc"),
                from_item_id: 3,
                to_item_ids: vec![2],
            },
            IrefEntry {
                kind: FourCC::new(*b"cdsc"),
                from_item_id: 4,
                to_item_ids: vec![1],
            },
        ],
    )
    .unwrap();
    let iprp = common::boxed(
        b"iprp",
        &[
            common::boxed(b"ipco", &make_ispe_box(2, 3).unwrap()),
            make_ipma_box(
                0,
                0,
                &[IpmaEntry {
                    item_id: 1,
                    associations: vec![IpmaAssociation {
                        property_index: 1,
                        essential: true,
                    }],
                }],
            )
            .unwrap(),
        ]
        .concat(),
    );
    let idat = common::boxed(
        b"idat",
        &[image.as_slice(), gain, aux_xmp, primary_xmp].concat(),
    );
    let meta = common::full(
        b"meta",
        &[make_pitm_box(0, 1).unwrap(), iinf, iloc, iref, iprp, idat].concat(),
    );
    let ftyp = common::boxed(b"ftyp", &[b"heic".as_slice(), &[0; 4], b"mif1"].concat());
    [ftyp, meta].concat()
}

#[test]
fn primary_motion_xmp_becomes_the_first_rdf_xml_infe() {
    let aux_xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/" hdrgm:Version="1.0"/></rdf:RDF></x:xmpmeta>"#;
    let primary_xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Vendor="urn:llp:test" Vendor:keep="yes"/></rdf:RDF></x:xmpmeta>"#;
    let still = heif_with_leading_auxiliary_xmp(aux_xmp, primary_xmp);
    let before = parsed_meta(&still);
    assert_eq!(
        before
            .iinf
            .entries
            .iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>(),
        vec![1, 3, 2, 4]
    );
    assert_eq!(
        before
            .iinf
            .entries
            .iter()
            .filter(|item| is_rdf_xml(&still, item))
            .map(|item| item.item_id)
            .collect::<Vec<_>>(),
        vec![3, 4]
    );
    let movie = common::movie(1000, 1);
    let out = build(&still, &movie, TargetProfile::AndroidHeif, false);
    let bytes = &out.files[0].bytes;
    let after = parsed_meta(bytes);
    let ids: Vec<_> = after.iinf.entries.iter().map(|item| item.item_id).collect();
    assert_eq!(ids, vec![1, 4, 2, 3]);
    let rdf: Vec<_> = after
        .iinf
        .entries
        .iter()
        .filter(|item| is_rdf_xml(bytes, item))
        .map(|item| item.item_id)
        .collect();
    assert_eq!(rdf, vec![4, 3]);
    assert_eq!(after.primary_item_id, 1);
    let cdsc = |from: u32| {
        after
            .iref
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.kind.as_bytes() == b"cdsc" && e.from_item_id == from)
            .unwrap()
            .to_item_ids
            .clone()
    };
    assert_eq!(cdsc(4), vec![1]);
    assert_eq!(cdsc(3), vec![2]);
    let motion_text = String::from_utf8(payload(bytes, &after, 4)).unwrap();
    assert!(motion_text.contains("Camera:MotionPhoto=\"1\""));
    assert!(motion_text.contains("Vendor:keep=\"yes\""));
    assert!(!motion_text.contains("hdrgm:Version"));
    assert_eq!(payload(bytes, &after, 3), aux_xmp);
    assert_eq!(payload(bytes, &after, 1), b"synthetic-encoded-item");
    assert_eq!(payload(bytes, &after, 2), b"synthetic-gain-map-item");
    let parsed = MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::strict()).unwrap();
    assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
    assert!(xmp_of(bytes).contains("Camera:MotionPhoto=\"1\""));
}
