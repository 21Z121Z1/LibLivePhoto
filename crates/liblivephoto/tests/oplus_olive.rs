mod common;
use liblivephoto::*;
fn still(props: &str) -> Vec<u8> {
    common::jpeg(&format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:o="http://ns.oplus.com/photo/1.0/camera/" {props}/></rdf:RDF></x:xmpmeta>"#
    ))
}
fn footer(mut data: Vec<u8>, n: u32) -> Vec<u8> {
    data.extend(b"jxrs");
    data.extend(n.to_le_bytes());
    data
}
#[test]
fn attribute_and_element_xmp_use_namespaces_and_small_exact_movie_ranges() {
    let movie = common::movie(1000, 1);
    let attrs = still(&format!(
        r#"o:OLivePhotoVersion="2" o:VideoLength="{}" o:PrimaryPresentationTimestampUs="500000""#,
        movie.len()
    ));
    let input = [attrs, movie.clone()].concat();
    let m = inspect_oplus_metadata(&input).unwrap().unwrap();
    assert_eq!(m.version, Some(2));
    assert_eq!(m.photo_time, MediaTime::new(500000, 1_000_000));
    assert_eq!(m.video.unwrap().length(), movie.len() as u64);
    let photo = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
    assert_eq!(photo.motion_video_bytes().unwrap(), movie);
    assert_eq!(photo.asset().dialect, VendorDialect::Oplus);
    let xml = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r xmlns:o="http://ns.oppo.com/photo/1.0/camera/"><o:OLivePhotoVersion> 2 </o:OLivePhotoVersion><o:PrimaryPresentationTimestampUs> 500000 </o:PrimaryPresentationTimestampUs></r></x:xmpmeta>"#;
    let input = [common::jpeg(xml), movie.clone()].concat();
    assert_eq!(
        inspect_oplus_metadata(&input).unwrap().unwrap().photo_time,
        m.photo_time
    );
}
#[test]
fn checksum_and_short_footers_extract_movie_without_claiming_checksum_validation() {
    let movie = common::movie(1000, 1);
    for checksum in [false, true] {
        let mut trailer = movie.clone();
        if checksum {
            trailer.extend([b'a'; 60]);
        }
        let input = [vec![255, 216, 255, 217], footer(trailer, 2)].concat();
        let m = inspect_oplus_metadata(&input).unwrap().unwrap();
        assert_eq!(m.video.unwrap().length(), movie.len() as u64);
        assert_eq!(m.checksum_bytes.is_some(), checksum);
        let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
        assert_eq!(p.motion_video_bytes().unwrap(), movie);
        assert!(p
            .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
            .execute()
            .is_err());
    }
}
#[test]
fn backward_directory_supports_both_footer_length_conventions_and_preserves_unknown_records() {
    let movie = common::movie(1000, 1);
    let dir=br#"[{"name":"live.subVideo","offset":6,"length":4,"extra":"opaque"},{"name":"capture.mode","offset":2,"length":2}]"#;
    for includes_footer in [false, true] {
        let mut trailer = movie.clone();
        trailer.extend(b"SUB!CM");
        trailer.extend(dir);
        trailer.push(0);
        let n = dir.len() + 1 + if includes_footer { 8 } else { 0 };
        let input = [vec![255, 216, 255, 217], footer(trailer, n as u32)].concat();
        let m = inspect_oplus_metadata(&input).unwrap().unwrap();
        assert_eq!(m.records.len(), 2);
        assert_eq!(m.records[0].raw["extra"], "opaque");
        assert_eq!(m.video.unwrap().length(), movie.len() as u64);
        let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
        assert_eq!(p.motion_video_bytes().unwrap(), movie);
        assert!(p
            .resources()
            .any(|r| r.vendor_role.as_deref() == Some("live.subVideo")
                && r.role == ResourceRole::Unknown));
        let policy = WritePolicy {
            unmapped: UnmappedPolicy::PreserveSidecars,
            ..WritePolicy::default()
        };
        let built = p
            .plan(TargetProfile::AndroidJpeg, policy)
            .execute()
            .unwrap();
        assert!(built
            .files
            .iter()
            .any(|f| f.sidecar_for.is_some() && f.bytes == b"SUB!"));
    }
}
#[test]
fn malformed_metadata_and_conflicting_lengths_fail_instead_of_selecting_auxiliary_movie() {
    let movie = common::movie(1000, 1);
    let input=[still(r#"o:OLivePhotoVersion="2" o:PrimaryPresentationTimestampUs="2" o:MotionPhotoPresentationTimestampUs="3""#),movie.clone()].concat();
    assert!(inspect_oplus_metadata(&input).is_err());
    let dir = br#"[{"name":"unknown","offset":999999,"length":2}]"#;
    let input = [
        vec![255, 216, 255, 217],
        footer(
            [movie.clone(), dir.to_vec(), vec![0]].concat(),
            dir.len() as u32 + 1,
        ),
    ]
    .concat();
    assert!(inspect_oplus_metadata(&input).is_err());
    let input = [still("o:OLivePhotoVersion=\"2\""), vec![]].concat();
    assert!(inspect_oplus_metadata(&input)
        .unwrap()
        .unwrap()
        .video
        .is_none());
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).is_err());
}
