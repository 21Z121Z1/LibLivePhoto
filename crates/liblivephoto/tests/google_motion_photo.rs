//! Byte-level Motion Photo 1.0 writer contracts. These do not certify Photos playback.
mod common;
use liblivephoto::*;
use liblivephoto_format::isobmff::parse_boxes;

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
