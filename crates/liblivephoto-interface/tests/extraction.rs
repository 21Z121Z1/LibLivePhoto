use liblivephoto::{Input, MotionPhoto, ParseOptions, RelationshipKind, ResourceRole};
use liblivephoto_interface::{process, OperationFile, OperationResult};
use serde_json::json;
#[path = "../../liblivephoto/tests/common/mod.rs"]
mod common;

#[test]
fn selected_shared_resources_keep_unknown_bytes_order_and_manifest() {
    let extra = (0..5).map(|n| format!(r#"<rdf:li><Container:Item Item:Mime="application/octet-stream" Item:Semantic="Unknown{n}" Item:Length="0"/></rdf:li>"#)).collect::<String>();
    let movie = common::movie(1000, 1);
    let source = [common::jpeg(&common::xmp(movie.len(), 0, &extra)), movie].concat();
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let selected = [6, 2, 4];
    let files = selected
        .map(|id| {
            let resource = photo.resources().find(|r| r.id.0 == id).unwrap();
            assert_eq!(resource.role, ResourceRole::Unknown);
            assert_eq!(
                resource.relationship.as_ref().unwrap().kind,
                RelationshipKind::SharedData
            );
            OperationFile {
                name: format!("resource-{id}.bin"),
                mime: "application/octet-stream".into(),
                resource_id: Some(id),
                bytes: photo.still_bytes().unwrap().to_vec(),
            }
        })
        .to_vec();
    let metadata = json!({"files":files.iter().map(|f| json!({"name":f.name,"mime":f.mime,"resourceId":f.resource_id,"bytes":f.bytes.len().to_string()})).collect::<Vec<_>>(),"unavailable":[]});
    let expected = OperationResult {
        success: true,
        metadata,
        files,
    };
    let actual = process(
        &json!({"action":"extract","resourceIds":selected}).to_string(),
        &source,
        &[],
    );
    assert_eq!(actual, expected);
    assert_eq!(
        actual.into_envelope().unwrap(),
        expected.into_envelope().unwrap()
    );
    for (ids, message) in [
        (json!([2, 2]), "invalid/duplicate resource ID"),
        (json!([2, 999]), "resource ID missing"),
        (json!([2, "bad"]), "invalid/duplicate resource ID"),
    ] {
        let result = process(
            &json!({"action":"extract","resourceIds":ids}).to_string(),
            &source,
            &[],
        );
        assert!(!result.success);
        assert!(result.files.is_empty());
        assert_eq!(result.metadata["error"]["code"], "invalidRequest");
        assert_eq!(result.metadata["error"]["message"], message);
    }
}

#[test]
fn default_jpeg_and_heif_extraction_keep_still_movie_and_xmp_order() {
    let movie = common::movie(1000, 1);
    for (still, target) in [
        (
            include_bytes!("../../../web/public/demo/cover.jpg").to_vec(),
            "google-jpeg",
        ),
        (common::heif(b"heic"), "google-heif"),
    ] {
        let built = process(&json!({"action":"build","inputKind":"parts","target":target,"time":{"value":"1","timescale":4}}).to_string(), &still, &movie);
        assert!(built.success, "{target}: {:?}", built.metadata);
        let source = &built.files[0].bytes;
        let photo =
            MotionPhoto::parse(Input::SingleFile(source), ParseOptions::compatible()).unwrap();
        let result = process(r#"{"action":"extract"}"#, source, &[]);
        assert!(result.success, "{:?}", result.metadata);
        assert_eq!(result.files.len(), 3);
        assert_eq!(result.files[0].resource_id, Some(photo.still().id.0));
        assert_eq!(result.files[0].bytes, photo.still_bytes().unwrap());
        assert_eq!(result.files[1].resource_id, Some(photo.motion_video().id.0));
        assert_eq!(result.files[1].bytes, movie);
        assert_eq!(result.files[2].name, "metadata.xmp");
        assert_eq!(result.files[2].mime, "application/rdf+xml");
        assert_eq!(result.files[2].resource_id, None);
        assert_eq!(
            result.files[2].bytes,
            photo.xmp_bytes().unwrap().unwrap().as_ref()
        );
        assert_eq!(result.metadata["unavailable"], json!([]));
    }
}

#[test]
fn selected_heif_item_uses_ordered_extent_bytes_and_exact_length() {
    use liblivephoto_format::isobmff::{make_iloc_box, IlocEntry, IlocExtent};
    let iloc = make_iloc_box(
        1,
        4,
        4,
        0,
        0,
        &[IlocEntry {
            item_id: 1,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![
                IlocExtent {
                    index: None,
                    offset: 12,
                    length: 3,
                },
                IlocExtent {
                    index: None,
                    offset: 0,
                    length: 4,
                },
            ],
        }],
    )
    .unwrap();
    let still = common::replace_heif_meta_child(&common::heif(b"heic"), b"iloc", &iloc);
    let movie = common::movie(1000, 1);
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: None,
        },
        ParseOptions::compatible(),
    )
    .unwrap();
    let item = photo
        .resources()
        .find(|r| r.container_item_id == Some(1))
        .unwrap();
    assert_eq!(item.extents.len(), 2);
    let result = process(
        &json!({"action":"extract","inputKind":"parts","resourceIds":[item.id.0]}).to_string(),
        &still,
        &movie,
    );
    assert!(result.success, "{:?}", result.metadata);
    let encoded = b"synthetic-encoded-item";
    assert_eq!(
        result.files[0].bytes,
        [&encoded[12..15], &encoded[..4]].concat()
    );
    assert_eq!(result.metadata["files"][0]["bytes"], "7");
}
