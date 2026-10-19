use liblivephoto_interface::*;
use serde_json::json;
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
