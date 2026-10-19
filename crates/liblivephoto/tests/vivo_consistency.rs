mod common;
use liblivephoto::*;

const A: &[u8; 28] = b"synthetic-camera-id-00000000";
const B: &[u8; 28] = b"synthetic-camera-id-00000001";
fn footer(envelope: &[u8; 28], declared: &[u8; 28]) -> Vec<u8> {
    let json = format!(
        r#"{{"version":2202,"com.android.camera.imageTime":31,"com.android.camera.livephoto":"{}","unknown":1.234567890123456789e100}}"#,
        std::str::from_utf8(declared).unwrap()
    );
    common::camera_album_envelope(b"vivo", json.as_bytes(), envelope, false)
}
fn movie(envelope: &[u8; 28], declared: &[u8; 28]) -> Vec<u8> {
    [
        common::movie(1000, 1),
        common::vivo_stream_box(&[(8, 255, b"opaque\0unknown"), (8, 6, b"second")]),
        common::boxed(
            b"uuid",
            &[b"vivoMediaExtInfo".to_vec(), footer(envelope, declared)].concat(),
        ),
    ]
    .concat()
}
fn input(video: &[u8], xml: &str) -> Vec<u8> {
    [common::jpeg(xml), video.to_vec()].concat()
}
fn parse(bytes: &[u8]) -> MotionPhoto<'_> {
    MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::compatible()).unwrap()
}
fn slice(data: &[u8], ranges: &[ByteRange]) -> Vec<u8> {
    ranges
        .iter()
        .flat_map(|r| &data[r.lower_bound as usize..r.upper_bound as usize])
        .copied()
        .collect()
}

#[test]
fn physical_footer_aliases_are_deduplicated_and_all_ranges_point_to_original_source_bytes() {
    let video = movie(A, A);
    let bytes = input(&video, &common::xmp(video.len(), 1_000_000, ""));
    let p = parse(&bytes);
    let report = p.vivo_declaration_report().unwrap().unwrap();
    assert_eq!(report.footers.len(), 1);
    let f = &report.footers[0];
    assert_eq!(f.source_index, 0);
    assert_eq!(
        f.origins,
        [
            VivoFooterOrigin::TerminalSource,
            VivoFooterOrigin::MotionVideoUuid
        ]
    );
    assert_eq!(report.identifier_comparisons, 1);
    assert!(report.issues.is_empty());
    assert_eq!(report.xmp_spans.len(), 1);
    assert_eq!(report.xmp_spans[0].status, VivoXmpSpanStatus::Matched);
    assert_eq!(slice(&bytes, &f.json_extents), f.raw_json);
    assert_eq!(slice(&bytes, &f.identifier_extents), A);
    assert!(f
        .extents
        .iter()
        .all(|r| r.lower_bound >= p.motion_video().extents[0].lower_bound
            && r.upper_bound <= bytes.len() as u64));
    assert_eq!(f.image_time, Some(31));
    assert!(std::str::from_utf8(f.raw_json)
        .unwrap()
        .contains("1.234567890123456789e100"));
    let json = report.json();
    assert_eq!(json["pairingVerified"], false);
    assert_eq!(json["imageTimeUnit"], "unverified");
    let metadata = MetadataEngine::new()
        .parse(&bytes, &HandlerOptions::default())
        .unwrap();
    let projection = metadata
        .decoded
        .iter()
        .find(|d| d.provider == "vivo_media")
        .unwrap();
    assert_eq!(projection.value["declarations"], json);
    assert!(metadata.verified_motion);
    assert_eq!(p.motion_video_bytes().unwrap(), video);
}

#[test]
fn internal_conflicts_are_reported_without_invalidating_or_rewriting_the_media() {
    let video = movie(A, B);
    let bytes = input(&video, &common::xmp(video.len(), 1_000_000, ""));
    let p = parse(&bytes);
    let report = p.vivo_declaration_report().unwrap().unwrap();
    assert_eq!(report.issues.len(), 1);
    assert_eq!(report.issues[0].code, "footer-internal-identifiers-differ");
    assert_eq!(report.issues[0].footer_indices, [0]);
    let metadata = MetadataEngine::new()
        .parse(&bytes, &HandlerOptions::default())
        .unwrap();
    assert!(metadata.verified_motion);
    assert!(metadata
        .diagnostics
        .iter()
        .any(|s| s.contains("footer-internal-identifiers-differ")));
    assert_eq!(
        p.plan(TargetProfile::PreserveSource, WritePolicy::default())
            .execute()
            .unwrap()
            .files[0]
            .bytes,
        bytes
    );
    assert_eq!(p.motion_video_bytes().unwrap(), video);
}

#[test]
fn distinct_parts_and_source_indices_are_retained_even_for_equal_ids_or_equal_offsets() {
    for declared in [A, B] {
        let video = movie(declared, declared);
        let still = [
            common::jpeg(&common::xmp(video.len(), 1_000_000, "")),
            footer(A, A),
        ]
        .concat();
        let p = MotionPhoto::parse(
            Input::Parts {
                still: &still,
                movie: &video,
                presentation: MediaTime::new(1, 1),
            },
            ParseOptions::compatible(),
        )
        .unwrap();
        let report = p.vivo_declaration_report().unwrap().unwrap();
        assert_eq!(report.footers.len(), 2);
        assert_eq!(report.footers[0].source_index, 0);
        assert_eq!(report.footers[1].source_index, 1);
        assert_eq!(report.footers[1].origins.len(), 2);
        assert_eq!(report.identifier_comparisons, 4);
        assert_eq!(report.xmp_spans[0].status, VivoXmpSpanStatus::NotComparable);
        assert!(report.xmp_spans[0].declared_extent.is_none());
        if declared == A {
            assert!(report.issues.is_empty());
        } else {
            assert_eq!(
                report.issues.iter().map(|i| i.code).collect::<Vec<_>>(),
                ["envelope-identifiers-differ", "json-identifiers-differ"]
            );
        }
        for f in &report.footers {
            let source = p.source_bytes(f.source_index).unwrap();
            assert_eq!(slice(source, &f.json_extents), f.raw_json);
            assert_eq!(
                slice(source, &f.identifier_extents),
                f.envelope_identifier.as_bytes()
            );
        }
    }
}

#[test]
fn legacy_and_directory_spans_are_checked_independently_with_explicit_invalid_and_bounds_results() {
    let video = movie(A, A);
    for (offset, status) in [
        (video.len() as i64, VivoXmpSpanStatus::Matched),
        (video.len() as i64 - 1, VivoXmpSpanStatus::Mismatch),
        (i64::MAX, VivoXmpSpanStatus::OutOfRange),
        (0, VivoXmpSpanStatus::InvalidDeclaration),
        (-1, VivoXmpSpanStatus::InvalidDeclaration),
    ] {
        let xml=common::xmp(video.len(),1_000_000,"").replace("Camera:MotionPhoto=\"1\"",&format!("Camera:MotionPhoto=\"1\" Camera:MicroVideo=\"1\" Camera:MicroVideoOffset=\"{offset}\""));
        let bytes = input(&video, &xml);
        let p = parse(&bytes);
        let report = p.vivo_declaration_report().unwrap().unwrap();
        assert_eq!(report.xmp_spans.len(), 2);
        assert_eq!(report.xmp_spans[0].status, VivoXmpSpanStatus::Matched);
        assert_eq!(report.xmp_spans[1].status, status);
        assert_eq!(
            report.issues.is_empty(),
            status == VivoXmpSpanStatus::Matched
        );
        assert_eq!(p.motion_video_bytes().unwrap(), video);
    }
}

#[test]
fn recovered_video_exposes_a_disagreeing_directory_instead_of_replacing_it() {
    let video = movie(A, A);
    let bytes = input(&video, &common::xmp(video.len() - 1, 1_000_000, ""));
    assert!(MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::compatible()).is_err());
    let p = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::forensic()).unwrap();
    let report = p.vivo_declaration_report().unwrap().unwrap();
    assert_eq!(report.xmp_spans[0].status, VivoXmpSpanStatus::Mismatch);
    assert_eq!(report.issues[0].code, "xmp-media-span");
    assert_eq!(p.motion_video_bytes().unwrap(), video);
    assert_eq!(
        p.xmp_bytes().unwrap().unwrap().as_ref(),
        common::xmp(video.len() - 1, 1_000_000, "").as_bytes()
    );
}

#[test]
fn nested_magic_has_no_uuid_association_and_known_malformed_uuid_audit_fails() {
    let video = [
        common::movie(1000, 1),
        common::boxed(b"free", &common::vivo_footer_box()),
    ]
    .concat();
    let bytes = input(&video, &common::xmp(video.len(), 1_000_000, ""));
    let p = parse(&bytes);
    let report = p.vivo_declaration_report().unwrap().unwrap();
    assert_eq!(
        report.footers[0].origins,
        [VivoFooterOrigin::TerminalSource]
    );
    assert!(p.vivo_media_extensions().unwrap().is_none());
    let mut video = movie(A, A);
    let n = video.windows(11).position(|w| w == b"streamcount").unwrap();
    video[n] ^= 1;
    let bytes = input(&video, &common::xmp(video.len(), 1_000_000, ""));
    assert!(parse(&bytes).vivo_declaration_report().is_err());
}

#[test]
fn provider_filters_do_not_enable_terminal_camera_album_through_the_new_projection() {
    let video = movie(A, A);
    let bytes = input(&video, &common::xmp(video.len(), 1_000_000, ""));
    let metadata = MetadataEngine::new()
        .parse(
            &bytes,
            &HandlerOptions {
                disable_third_party: false,
                enabled_third_party: vec!["vivo_media".into()],
            },
        )
        .unwrap();
    let projection = metadata
        .decoded
        .iter()
        .find(|d| d.provider == "vivo_media")
        .unwrap();
    assert_eq!(
        projection.value["declarations"]["footers"][0]["origins"],
        serde_json::json!(["MotionVideoUuid"])
    );
    assert!(!metadata
        .decoded
        .iter()
        .any(|d| d.provider == "camera_album"));
    let metadata = MetadataEngine::new()
        .parse(
            &bytes,
            &HandlerOptions {
                disable_third_party: true,
                enabled_third_party: Vec::new(),
            },
        )
        .unwrap();
    assert!(!metadata.decoded.iter().any(|d| d.provider == "vivo_media"));
}
