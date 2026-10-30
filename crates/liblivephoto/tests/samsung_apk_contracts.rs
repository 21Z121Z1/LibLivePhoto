//! Owned synthetic controls for the signed Samsung component observations in
//! Static Samsung package observations live on branch archive/evidence-20261030 at evidence/SAMSUNG_APK_ROUND7.md. No vendor APK or implementation is linked.
mod common;
use liblivephoto::*;

#[test]
fn samsung_type_and_name_must_agree_before_default_video_selection() {
    let wrong_type = common::movie(1000, 1);
    let wrong_name = common::movie(2000, 1);
    let primary = common::movie(3000, 1);
    let mut records = vec![
        (0x0a33, "MotionPhoto_Data", wrong_type.clone()),
        (0x0a30, "MotionPhoto_AutoPlay", wrong_name.clone()),
    ];
    let source = common::sef(&records);
    let inventory = inspect_samsung_videos(&source).unwrap().unwrap();
    assert!(inventory.declared_primary_records.is_empty());
    assert!(matches!(
        inventory.select(None),
        Err(MotionPhotoReadError::Unrecognized)
    ));
    assert_eq!(inventory.select(Some(0)).unwrap().bytes, wrong_type);
    assert_eq!(inventory.select(Some(1)).unwrap().bytes, wrong_name);
    assert!(MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).is_err());

    records.push((0x0a30, "MotionPhoto_Data", primary.clone()));
    let source = common::sef(&records);
    let inventory = inspect_samsung_videos(&source).unwrap().unwrap();
    assert_eq!(inventory.declared_primary_records, [2]);
    assert_eq!(inventory.select(None).unwrap().bytes, primary);
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert_eq!(photo.motion_video_bytes().unwrap(), primary);
    for original in [&wrong_type, &wrong_name] {
        let resource = photo
            .resources()
            .find(|resource| {
                if resource.role != ResourceRole::UnknownVideo {
                    return false;
                }
                let mut bytes = Vec::new();
                photo.extract(resource, &mut bytes).unwrap();
                bytes == *original
            })
            .expect("crossed wire declarations remain separate unknown videos");
        assert_ne!(resource.id, photo.motion_video().id);
    }
}

#[test]
fn samsung_xmp_microseconds_keep_absence_zero_and_fractional_time_distinct() {
    let video = common::movie(1000, 1);
    let sef = common::sef(&[(0x0a30, "MotionPhoto_Data", video.clone())]);
    for value in [None, Some(0), Some(123_457)] {
        let property = value
            .map(|time| format!(" Camera:MotionPhotoPresentationTimestampUs=\"{time}\""))
            .unwrap_or_default();
        let xmp = format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"{property}/></rdf:RDF></x:xmpmeta>"#
        );
        // Direct SEF records and their directory move together. Backward
        // offsets stay valid; no vendor pointer or time field is invented.
        let source = [common::jpeg(&xmp), sef[4..].to_vec()].concat();
        let photo =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        assert_eq!(
            photo.presentation_time(),
            value.map(|value| MediaTime {
                value,
                timescale: 1_000_000,
            })
        );
        assert_eq!(photo.motion_video_bytes().unwrap(), video);
    }
}
