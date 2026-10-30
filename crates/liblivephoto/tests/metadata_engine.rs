mod common;
use liblivephoto::*;
use serde_json::json;
#[test]
fn ordinary_still_motion_and_serialized_collections_are_distinct() {
    let engine = MetadataEngine::new();
    let still = include_bytes!("../../../web/public/demo/cover.jpg");
    let plain = engine.parse(still, &HandlerOptions::default()).unwrap();
    assert!(!plain.verified_motion);
    assert!(!plain.declares_motion(&HandlerOptions::default()));
    assert!(plain.blocks.iter().any(|b| b.kind == "JFIF"));
    let input = include_bytes!("../../../web/public/demo/motion-photo.jpg");
    let motion = engine.parse(input, &HandlerOptions::default()).unwrap();
    assert!(motion.verified_motion);
    assert!(motion.declares_motion(&HandlerOptions::default()));
    let xmp = motion.blocks.iter().find(|b| b.kind == "XMP").unwrap();
    assert!(XmpDocument::parse(&xmp.raw_bytes)
        .unwrap()
        .validate_motion()
        .is_ok());
    let doc = motion.json().to_string();
    let loaded = MetadataCollection::from_json(doc.as_bytes()).unwrap();
    assert!(!loaded.verified_motion);
    assert!(loaded.declares_motion(&HandlerOptions::default()));
    assert_eq!(loaded.blocks, motion.blocks);
    let mut invalid = motion.json();
    invalid["blocks"][0]["extents"][0]["end"] = json!("18446744073709551616");
    assert!(MetadataCollection::from_json(invalid.to_string().as_bytes()).is_err());
}
#[test]
fn heif_items_and_large_trailers_are_bounded_views_and_full_bytes_can_be_extracted() {
    let mut image = common::heif(b"avif");
    let collection = MetadataEngine::new()
        .parse(&image, &HandlerOptions::default())
        .unwrap();
    assert!(collection
        .blocks
        .iter()
        .any(|b| b.format_identifier == "heif.item"));
    image = vec![255, 216, 255, 217];
    image.extend(vec![0x55; 1024 * 1024]);
    let c = MetadataEngine::new()
        .parse(&image, &HandlerOptions::default())
        .unwrap();
    let b = c
        .blocks
        .iter()
        .find(|b| b.format_identifier == "container.trailer")
        .unwrap();
    assert!(!b.raw_complete);
    assert_eq!(b.raw_bytes.len(), 65536);
    let mut output = Vec::new();
    c.extract_block(0, &image, &mut output, 2 * 1024 * 1024)
        .unwrap();
    assert_eq!(output, &image[4..]);
    assert!(c.extract_block(0, &image, std::io::sink(), 64).is_err());
    let loaded = MetadataCollection::from_json(c.json().to_string().as_bytes()).unwrap();
    assert_eq!(loaded.blocks, c.blocks);
}
struct Custom;
impl MetadataProvider for Custom {
    fn name(&self) -> &str {
        "test.provider"
    }
    fn decode(
        &self,
        b: &MetadataBlock,
        _source: &[u8],
    ) -> std::result::Result<Option<ProviderPayload>, MotionPhotoReadError> {
        Ok(
            (b.format_identifier == "container.trailer").then(|| ProviderPayload {
                value: json!({"observed":true}),
                motion_declaration: true,
                video_in_block: Some(ByteRange::new(0, 4).unwrap()),
            }),
        )
    }
}
#[test]
fn manually_constructed_invalid_extents_fail_before_any_output_is_written() {
    let mut collection = MetadataCollection {
        source_length: 8,
        blocks: vec![MetadataBlock {
            kind: "custom".into(),
            format_identifier: "test".into(),
            extents: vec![ByteRange::new(0, 2).unwrap()],
            header_extents: vec![],
            raw_bytes: vec![],
            raw_complete: false,
        }],
        decoded: vec![],
        diagnostics: vec![],
        verified_motion: false,
    };
    for range in [
        ByteRange {
            lower_bound: 5,
            upper_bound: 3,
        },
        ByteRange {
            lower_bound: 0,
            upper_bound: 9,
        },
        ByteRange {
            lower_bound: u64::MAX,
            upper_bound: 0,
        },
    ] {
        collection.blocks[0].extents.truncate(1);
        collection.blocks[0].extents.push(range);
        let mut output = Vec::new();
        assert!(matches!(
            collection.extract_block(0, b"12345678", &mut output, 16),
            Err(MotionPhotoReadError::InvalidResource)
        ));
        assert!(
            output.is_empty(),
            "a later invalid extent must not publish a prefix"
        );
    }
}

struct ReversedRange;
impl MetadataProvider for ReversedRange {
    fn name(&self) -> &str {
        "test.reversed"
    }
    fn decode(
        &self,
        b: &MetadataBlock,
        _: &[u8],
    ) -> std::result::Result<Option<ProviderPayload>, MotionPhotoReadError> {
        Ok(
            (b.format_identifier == "container.trailer").then(|| ProviderPayload {
                value: json!({}),
                motion_declaration: true,
                video_in_block: Some(ByteRange {
                    lower_bound: 4,
                    upper_bound: 1,
                }),
            }),
        )
    }
}

#[test]
fn provider_relative_ranges_cannot_panic_the_engine() {
    let mut engine = MetadataEngine::new();
    engine.register(Box::new(ReversedRange)).unwrap();
    let result = engine.parse(
        &[255, 216, 255, 217, 1, 2, 3, 4],
        &HandlerOptions::default(),
    );
    assert!(matches!(result, Err(MotionPhotoReadError::InvalidResource)));
}

#[test]
fn collection_json_limits_tree_complexity_without_counting_string_contents() {
    let source = include_bytes!("../../../web/public/demo/motion-photo.jpg");
    let collection = MetadataEngine::new()
        .parse(source, &HandlerOptions::default())
        .unwrap();
    let mut document = collection.json();
    let opaque = "\"\\,:{[".repeat(100_000);
    document["decoded"][0]["value"]["opaque"] = json!(opaque);
    let restored = MetadataCollection::from_json(document.to_string().as_bytes()).unwrap();
    assert_eq!(restored.decoded[0].value["opaque"], opaque);
    let mut dense = collection.json().to_string();
    dense.pop();
    dense.push_str(",\"ignored\":[");
    dense.push_str(&"0,".repeat(300_000));
    dense.push_str("0]}");
    assert!(dense.len() < 1024 * 1024);
    assert!(matches!(
        MetadataCollection::from_json(dense.as_bytes()),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn custom_registration_and_filters_apply_to_decoding_and_core_parsing() {
    let mut engine = MetadataEngine::new();
    engine.register(Box::new(Custom)).unwrap();
    assert!(engine.register(Box::new(Custom)).is_err());
    let input = vec![255, 216, 255, 217, 1, 2, 3, 4];
    let all = engine.parse(&input, &HandlerOptions::default()).unwrap();
    assert!(all.declares_motion(&HandlerOptions::default()));
    assert!(!all.verified_motion);
    assert_eq!(
        all.decoded[0].video_extents,
        [ByteRange::new(4, 8).unwrap()]
    );
    let disabled = HandlerOptions {
        disable_third_party: true,
        ..HandlerOptions::default()
    };
    let none = engine.parse(&input, &disabled).unwrap();
    assert!(none.decoded.is_empty());
    assert!(!all.declares_motion(&disabled));
    let movie = common::movie(1000, 1);
    let mut op = common::jpeg(&format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><a xmlns:o="http://ns.oplus.com/photo/1.0/camera/" o:OLivePhotoVersion="2" o:VideoLength="{}"/></x:xmpmeta>"#,
        movie.len()
    ));
    op.extend(movie);
    assert!(MotionPhoto::parse_with_handlers(
        Input::SingleFile(&op),
        ParseOptions::compatible(),
        &HandlerOptions::default()
    )
    .is_ok());
    assert!(MotionPhoto::parse_with_handlers(
        Input::SingleFile(&op),
        ParseOptions::compatible(),
        &disabled
    )
    .is_err());
    let google = include_bytes!("../../../web/public/demo/motion-photo.jpg");
    assert!(MotionPhoto::parse_with_handlers(
        Input::SingleFile(google),
        ParseOptions::compatible(),
        &disabled
    )
    .is_ok());
}
#[test]
fn standalone_xmp_rewrite_retains_unknown_values_and_exact_time_conversion() {
    let input = common::xmp(100, 500000, "");
    let doc = XmpDocument::parse(input.as_bytes()).unwrap();
    assert_eq!(doc.as_bytes(), input.as_bytes());
    let mut fields = doc.fields().clone();
    fields
        .camera_values
        .insert("MotionPhotoPresentationTimestampUs".into(), 812500);
    let changed = doc.rewrite(&fields).unwrap();
    assert!(String::from_utf8_lossy(changed.as_bytes()).contains("retain &amp; verify"));
    assert!(changed
        .photo_time()
        .unwrap()
        .equivalent(MediaTime::new(13, 16).unwrap()));
    assert!(changed.validate_motion().is_ok());
    assert!(XmpDocument::motion("image/jpeg", "video/mp4", 100, MediaTime::new(1, 3)).is_err());
}
#[test]
fn metadata_in_progressive_scan_tail_is_not_lost() {
    let data = [
        vec![255, 216, 255, 218, 0, 2],
        vec![1, 2, 255, 0, 3],
        vec![255, 254, 0, 6],
        b"note".to_vec(),
        vec![255, 217],
    ]
    .concat();
    let c = MetadataEngine::new()
        .parse(&data, &HandlerOptions::default())
        .unwrap();
    assert_eq!(c.blocks.len(), 1);
    assert_eq!(c.blocks[0].raw_bytes, b"note");
}
#[cfg(unix)]
#[test]
fn range_and_borrowed_fd_use_the_same_relative_collection_without_moving_cursor() {
    use std::io::{Seek, SeekFrom, Write};
    use std::os::fd::AsFd;
    let path =
        std::env::temp_dir().join(format!("liblivephoto-metadata-{}.bin", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .read(true)
        .open(&path)
        .unwrap();
    let bytes = include_bytes!("../../../web/public/demo/motion-photo.jpg");
    file.write_all(b"prefix-prefix").unwrap();
    file.write_all(bytes).unwrap();
    file.write_all(b"suffix").unwrap();
    file.seek(SeekFrom::Start(7)).unwrap();
    let engine = MetadataEngine::new();
    let options = HandlerOptions::default();
    let memory = engine.parse(bytes, &options).unwrap();
    let range = engine
        .parse_range(
            FileRange {
                file: &file,
                offset: 13,
                length: bytes.len() as u64,
            },
            &options,
        )
        .unwrap();
    let fd = engine
        .parse_fd(file.as_fd(), 13, bytes.len() as u64, &options)
        .unwrap();
    assert_eq!(range, memory);
    assert_eq!(fd, memory);
    assert_eq!(file.stream_position().unwrap(), 7);
    drop(file);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn empty_and_multiple_root_xmp_documents_are_rejected() {
    assert!(XmpDocument::parse(b"").is_err());
    assert!(XmpDocument::parse(b"<a/><b/>").is_err());
}
