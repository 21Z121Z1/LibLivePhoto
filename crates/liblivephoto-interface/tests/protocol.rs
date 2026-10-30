use liblivephoto_interface::*;
use serde_json::json;
#[path = "../../liblivephoto/tests/common/mod.rs"]
mod common;

#[test]
fn explicit_samsung_selection_does_not_parse_or_rewrite_an_ambiguous_asset() {
    let source = include_bytes!("../../../fixtures/vendor-selection/distinct-ambiguous.jpg");
    let a = include_bytes!("../../../fixtures/vendor-selection/movie-a.mp4");
    let b = include_bytes!("../../../fixtures/vendor-selection/movie-b.mp4");
    assert!(!process(r#"{"action":"extract"}"#, source, &[]).success);
    for (index, expected) in [
        ("0", a.as_slice()),
        ("2", b.as_slice()),
        ("3", a.as_slice()),
    ] {
        let result = process(
            &json!({"action":"extract","samsungRecordIndex":index}).to_string(),
            source,
            &[],
        );
        assert!(result.success, "{:?}", result.metadata);
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].bytes, expected);
        assert_eq!(result.metadata["role"], "uninterpreted");
        assert_eq!(result.files[0].resource_id, None);
    }
    for request in [
        json!({"action":"extract","samsungRecordIndex":0}),
        json!({"action":"metadata","samsungRecordIndex":"0"}),
        json!({"action":"extract","samsungRecordIndex":"0","resourceIds":[0]}),
        json!({"action":"extract","samsungRecordIndex":"0","time":{"value":"1","timescale":1}}),
    ] {
        let result = process(&request.to_string(), source, &[]);
        assert!(!result.success);
        assert!(result.files.is_empty());
        assert_eq!(result.metadata["error"]["code"], "invalidRequest");
    }
    assert!(
        !process(
            r#"{"action":"extract","samsungRecordIndex":"0"}"#,
            source,
            a
        )
        .success
    );
}

#[test]
fn conformance_profiles_report_rule_kinds_and_reject_ambiguous_associations() {
    let image = common::heif_with_properties(&[
        common::boxed(b"colr", b"profraw-icc"),
        common::boxed(b"colr", b"nclx\0\x02\0\x02\0\x09\x80"),
    ]);
    let result = process(
        r#"{"action":"conformance","profile":"heif-color-2025"}"#,
        &image,
        &[],
    );
    assert!(result.success);
    assert_eq!(result.metadata["checkedRequiredRulesSatisfied"], true);
    assert!(result.files.is_empty());
    let bad = common::heif_with_properties(&[
        common::boxed(b"colr", b"profraw-icc"),
        common::boxed(b"colr", b"nclx\0\x09\0\x12\0\x09\x80"),
    ]);
    let result = process(
        r#"{"action":"conformance","profile":"heif-color-2025"}"#,
        &bad,
        &[],
    );
    assert!(!result.success);
    assert_eq!(result.metadata["issues"][0]["kind"], "Violation");
    let movie = common::many_frame_movie(3, 2);
    let result = process(
        r#"{"action":"conformance","profile":"google-video-container-1.0","trackIds":[1,2]}"#,
        &movie,
        &[],
    );
    assert!(result.success, "{:?}", result.metadata);
    let result = process(
        r#"{"action":"conformance","profile":"google-video-container-1.0","trackIds":[1]}"#,
        &movie,
        &[],
    );
    assert!(!result.success);
    assert_eq!(result.metadata["issues"][0]["kind"], "Unsupported");
    for request in [
        r#"{"action":"conformance","profile":[]}"#,
        r#"{"action":"conformance","profile":"invented"}"#,
        r#"{"action":"conformance","profile":"google-video-container-1.0","trackIds":[1,1]}"#,
        r#"{"action":"conformance","profile":"google-video-container-1.0","trackIds":[1,2,3]}"#,
    ] {
        let result = process(request, &movie, &[]);
        assert!(!result.success);
        assert_eq!(result.metadata["error"]["code"], "invalidRequest");
    }
    assert!(
        !process(
            r#"{"action":"conformance","profile":"heif-color-2025"}"#,
            &image,
            b"unused"
        )
        .success
    );
}
#[test]
fn protocol_returns_exact_bytes_and_explicit_plan_failures() {
    let still = include_bytes!("../../../web/public/demo/cover.jpg");
    let movie = include_bytes!("../../../web/public/demo/motion.mp4");
    let built=process(&json!({"action":"build","inputKind":"parts","time":{"value":"13","timescale":16},"target":"google-jpeg"}).to_string(),still,movie);
    assert!(built.success);
    let bytes = &built.files[0].bytes;
    let extract = process(
        r#"{"action":"extract","still":false,"video":true,"xmp":false}"#,
        bytes,
        &[],
    );
    assert!(extract.success);
    assert_eq!(extract.files[0].bytes, movie);
    let envelope = extract.into_envelope().unwrap();
    assert_eq!(&envelope[..4], b"LLP1");
    assert!(envelope.ends_with(movie));
    let blocked = process(
        r#"{"action":"build","inputKind":"parts","time":{"value":"1","timescale":3},"target":"google-jpeg"}"#,
        still,
        movie,
    );
    assert!(!blocked.success);
    assert!(blocked.files.is_empty());
    assert!(blocked.metadata["blockers"].is_array());
}
#[test]
fn request_type_errors_and_metadata_only_inputs_are_clear() {
    let still = include_bytes!("../../../web/public/demo/cover.jpg");
    let bad = process(
        r#"{"action":"parse","disableThirdParty":"true"}"#,
        still,
        &[],
    );
    assert!(!bad.success);
    assert_eq!(bad.metadata["error"]["code"], "invalidRequest");
    let metadata = process(r#"{"action":"metadata"}"#, still, &[]);
    assert!(metadata.success);
    assert_eq!(metadata.metadata["verifiedMotion"], false);
    assert!(!process(r#"{"action":"metadata"}"#, still, b"unused movie").success);
}
#[test]
fn physical_movie_brand_and_absent_optional_xmp_are_reported() {
    let movie = include_bytes!("../../../web/public/demo/apple-paired.mov");
    let still = include_bytes!("../../../web/public/demo/cover.jpg");
    let optimized = process(r#"{"action":"optimize"}"#, movie, &[]);
    assert!(optimized.success);
    assert_eq!(optimized.files[0].mime, "video/quicktime");
    assert_eq!(optimized.files[0].name, "movie.mov");
    let extracted = process(
        r#"{"action":"extract","inputKind":"parts","still":false,"video":false,"xmp":true}"#,
        still,
        movie,
    );
    assert!(extracted.success);
    assert!(extracted.files.is_empty());
    assert_eq!(extracted.metadata["unavailable"], json!(["xmp"]));
}
#[test]
fn agtm_binding_returns_complete_raw_metadata_and_source_classification_stays_unverified() {
    let movie = include_bytes!("../../../fixtures/bindings/agtm-video.mp4");
    let image = include_bytes!("../../../fixtures/bindings/agtm-motion.jpg");
    let raw = include_bytes!("../../../fixtures/bindings/agtm-raw.bin");
    let video_result = process(
        r#"{"action":"agtm","videoInput":true,"time":{"value":"1","timescale":1}}"#,
        movie,
        &[],
    );
    let image_result = process(r#"{"action":"agtm"}"#, image, &[]);
    for result in [video_result, image_result] {
        assert!(result.success);
        assert_eq!(result.files[0].bytes, raw);
        assert_eq!(
            result.metadata["samples"][0]["metadata"]["rules"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(result.metadata["renderedPixels"], false);
    }
    let classification = process(
        r#"{"action":"classify-metadata"}"#,
        include_bytes!("../../../fixtures/bindings/collection.json"),
        &[],
    );
    assert!(classification.success);
    assert_eq!(classification.metadata["declaresMotion"], true);
    assert_eq!(classification.metadata["verifiedMotion"], false);
}
#[test]
fn raw_metadata_blocks_are_exact_source_bytes_including_complete_large_trailers() {
    let mut large = include_bytes!("../../../web/public/demo/cover.jpg").to_vec();
    large.extend(vec![0x55; 1024 * 1024]);
    let complete = process(r#"{"action":"extract-metadata"}"#, &large, &[]);
    assert!(complete.success);
    assert_eq!(
        complete.files.last().unwrap().bytes,
        vec![0x55; 1024 * 1024]
    );
    assert_eq!(
        complete.metadata["collection"]["blocks"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["rawComplete"],
        false
    );
    let image = include_bytes!("../../../fixtures/bindings/agtm-motion.jpg");
    let result = process(r#"{"action":"extract-metadata"}"#, image, &[]);
    assert!(result.success);
    let collection = liblivephoto::MetadataEngine::new()
        .parse(image, &liblivephoto::HandlerOptions::default())
        .unwrap();
    assert_eq!(result.files.len(), collection.blocks.len());
    for (file, block) in result.files.iter().zip(&collection.blocks) {
        let expected = block
            .extents
            .iter()
            .flat_map(|e| &image[e.lower_bound as usize..e.upper_bound as usize])
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(file.bytes, expected);
        assert_eq!(file.resource_id, None);
    }
    assert!(
        !process(
            r#"{"action":"extract-metadata","blockIds":[0,0]}"#,
            image,
            &[]
        )
        .success
    );
}
