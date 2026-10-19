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
