mod common;
use liblivephoto::*;

#[test]
fn layout_dialect_and_provenance_are_independent_and_tracks_are_nested() {
    let input = common::input(1_234_000, 2);
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    assert_eq!(p.asset().layout, PhysicalLayout::AppendedResources);
    assert_eq!(p.format(), AssetFormat::AndroidMotionPhoto);
    assert_eq!(p.asset().dialect, VendorDialect::AndroidStandard);
    assert_eq!(p.resources().filter(|r| r.parent.is_none()).count(), 2);
    let c = &p.asset().containers[1];
    assert_eq!(c.resource, p.motion_video().id);
    assert_eq!(c.tracks.len(), 2);
    assert_eq!(c.tracks[0].role, ResourceRole::PrimaryMotionVideo);
    assert_eq!(c.tracks[1].role, ResourceRole::SubstitutionVideo);
    assert_eq!(c.tracks[1].id, 2);
    assert_eq!(
        c.tracks[1].relationship,
        Some(TrackRelationship {
            kind: RelationshipKind::Substitutes,
            target_track_id: c.tracks[0].id,
        })
    );
    assert_eq!(p.presentation_time(), MediaTime::new(1_234_000, 1_000_000));
}

#[test]
fn unknown_video_directory_item_does_not_replace_primary_motion() {
    let primary = common::movie(1000, 1);
    let unknown = common::movie(1000, 1);
    let extra = format!(
        r#"<rdf:li><Container:Item Item:Mime="video/mp4" Item:Semantic="depthMotionPhoto" Item:Length="{}"/></rdf:li>"#,
        unknown.len()
    );
    let input = [
        common::jpeg(&common::xmp(primary.len(), 123_000, &extra)),
        unknown.clone(),
        primary.clone(),
    ]
    .concat();
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    assert_eq!(p.motion_video_bytes().unwrap(), primary);
    let r = p
        .asset()
        .auxiliary
        .iter()
        .find(|r| r.vendor_role.as_deref() == Some("depthMotionPhoto"))
        .unwrap();
    assert_eq!(r.role, ResourceRole::UnknownVideo);
    assert_eq!(r.vendor_role.as_deref(), Some("depthMotionPhoto"));
    let mut extracted = Vec::new();
    p.extract(r, &mut extracted).unwrap();
    assert_eq!(extracted, unknown);
    assert_eq!(r.parent, Some(p.still().id));
}

#[test]
fn namespace_uri_determines_motion_fields_and_prefixes_can_change() {
    let v = common::movie(1000, 1);
    let xml = common::xmp(v.len(), 0, "");
    let aliased = xml
        .replace("Camera:", "c:")
        .replace("xmlns:Camera=", "xmlns:c=")
        .replace("Container:", "d:")
        .replace("xmlns:Container=", "xmlns:d=")
        .replace("Item:", "i:")
        .replace("xmlns:Item=", "xmlns:i=");
    let input = [common::jpeg(&aliased), v.clone()].concat();
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_ok());
    let spoof = xml.replace("http://ns.google.com/photos/1.0/camera/", "urn:spoof");
    let input = [common::jpeg(&spoof), v].concat();
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
}

#[test]
fn zero_length_unknown_item_is_a_shared_resource_and_remains_extractable() {
    let v = common::movie(1000, 1);
    let extra = r#"<rdf:li><Container:Item Item:Mime="application/octet-stream" Item:Semantic="PrivateUnknown" Item:Length="0"/></rdf:li>"#;
    let input = [common::jpeg(&common::xmp(v.len(), 0, extra)), v].concat();
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    let r = &p.asset().auxiliary[0];
    assert_eq!(r.role, ResourceRole::Unknown);
    assert_eq!(
        r.relationship.as_ref().unwrap().kind,
        RelationshipKind::SharedData
    );
    let mut extracted = Vec::new();
    p.extract(r, &mut extracted).unwrap();
    assert_eq!(extracted, p.still_bytes().unwrap());
}

#[test]
fn malformed_nested_xml_and_input_budgets_fail_closed() {
    let v = common::movie(1000, 1);
    let xml = common::xmp(v.len(), 0, "")
        .replace("<rdf:Seq>", &format!("<rdf:Seq>{}", "<n>".repeat(129)))
        .replace("</rdf:Seq>", &format!("{}</rdf:Seq>", "</n>".repeat(129)));
    let input = [common::jpeg(&xml), v].concat();
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
    assert!(matches!(
        MotionPhoto::parse(
            Input::SingleFile(&input),
            ParseOptions {
                max_input_bytes: 8,
                ..ParseOptions::forensic()
            }
        ),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn forensic_ambiguity_is_not_resolved_by_candidate_order() {
    let source = [
        vec![255, 216, 255, 217],
        common::movie(1000, 1),
        common::movie(1000, 1),
    ]
    .concat();
    assert!(matches!(
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::forensic()),
        Err(MotionPhotoReadError::AmbiguousRecovery)
    ));
}

#[test]
fn sample_tables_cannot_reference_headers_or_declare_unbounded_counts() {
    for (box_name, relative, value) in [
        (b"stco", 16, 0u32),
        (b"stsz", 16, u32::MAX),
        (b"stts", 16, u32::MAX),
    ] {
        let mut movie = common::movie(1000, 1);
        let start = movie.windows(4).position(|w| w == box_name).unwrap() - 4;
        movie[start + relative..start + relative + 4].copy_from_slice(&value.to_be_bytes());
        let input = [common::jpeg(&common::xmp(movie.len(), 123_000, "")), movie].concat();
        assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
    }
}
