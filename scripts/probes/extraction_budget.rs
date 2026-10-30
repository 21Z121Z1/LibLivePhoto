use liblivephoto::{Input, MotionPhoto, ParseOptions, RelationshipKind};
use liblivephoto_interface::{process, OperationResult, REPLAY_COPIED_BYTES, REPLAY_LIMIT};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
#[path = "../../liblivephoto/tests/common/mod.rs"]
mod common;

fn aliased_source(aliases: usize, filler: usize) -> Vec<u8> {
    let movie = common::movie(1000, 1);
    let extra = (0..aliases).map(|n| format!(r#"<rdf:li><Container:Item Item:Mime="application/octet-stream" Item:Semantic="Unknown{n}" Item:Length="0"/></rdf:li>"#)).collect::<String>();
    [
        common::jpeg(&common::xmp(movie.len(), 0, &extra)),
        vec![0x55; filler],
        movie,
    ]
    .concat()
}

fn boundary_source() -> Vec<u8> {
    let movie = include_bytes!("../../../web/public/demo/motion.mp4");
    let extra = (0..5).map(|n| format!(r#"<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="application/octet-stream" Item:Semantic="Unknown{n}" Item:Length="0"/></rdf:li>"#)).collect::<String>();
    let xmp = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1" Camera:MotionPhotoPresentationTimestampUs="0"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0"/></rdf:li>{extra}<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{}"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#,
        movie.len()
    );
    [common::jpeg(&xmp), movie.to_vec()].concat()
}

fn extract(source: &[u8], ids: &[usize], limit: usize) -> OperationResult {
    REPLAY_LIMIT.store(limit, Ordering::SeqCst);
    REPLAY_COPIED_BYTES.store(0, Ordering::SeqCst);
    process(
        &json!({"action":"extract","resourceIds":ids}).to_string(),
        source,
        &[],
    )
}

fn reject(source: &[u8], ids: &[usize], limit: usize, message: &str) {
    let result = extract(source, ids, limit);
    let copied = REPLAY_COPIED_BYTES.load(Ordering::SeqCst);
    println!(
        "{}",
        json!({"selected":ids,"limit":limit,"success":result.success,"copiedBytes":copied,"metadata":result.metadata})
    );
    assert!(!result.success);
    assert!(result.files.is_empty());
    assert_eq!(result.metadata["error"]["code"], "invalidRequest");
    assert_eq!(result.metadata["error"]["message"], message);
    assert_eq!(
        copied, 0,
        "the entire selection must pass before payload copies"
    );
}

fn snapshot(result: OperationResult) -> Value {
    assert!(result.success, "{:?}", result.metadata);
    let value = json!({"success":result.success,"metadata":result.metadata,"files":result.files.iter().map(|file| json!({"name":file.name,"mime":file.mime,"resourceId":file.resource_id,"bytes":file.bytes})).collect::<Vec<_>>()});
    assert!(result.into_envelope().is_ok());
    value
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args[1] == "compatibility" {
        let source = boundary_source();
        let selected = extract(&source, &[6, 2, 4], 100_000);
        let defaults = process(r#"{"action":"extract"}"#, &source, &[]);
        let absent = process(
            r#"{"action":"extract","inputKind":"parts","still":false,"video":false,"xmp":true}"#,
            include_bytes!("../../../web/public/demo/cover.jpg"),
            include_bytes!("../../../web/public/demo/motion.mp4"),
        );
        std::fs::write(
            &args[2],
            serde_json::to_vec(&json!([
                snapshot(selected),
                snapshot(defaults),
                snapshot(absent)
            ]))
            .unwrap(),
        )
        .unwrap();
        return;
    }
    let amplified = aliased_source(8, 4096);
    let photo =
        MotionPhoto::parse(Input::SingleFile(&amplified), ParseOptions::compatible()).unwrap();
    for resource in photo.resources().filter(|r| (2..10).contains(&r.id.0)) {
        assert_eq!(
            resource.relationship.as_ref().unwrap().kind,
            RelationshipKind::SharedData
        );
        assert_eq!(resource.extents, photo.still().extents);
    }
    reject(
        &amplified,
        &(2..10).collect::<Vec<_>>(),
        8192,
        "response budget exceeded",
    );
    let tiny = aliased_source(2, 0);
    let photo = MotionPhoto::parse(Input::SingleFile(&tiny), ParseOptions::compatible()).unwrap();
    let wire_only = aliased_source(2, 4000 - photo.still_bytes().unwrap().len());
    reject(&wire_only, &[2, 3], 8192, "response budget exceeded");
    let source = boundary_source();
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let accepted = extract(&source, &[6, 2, 4], 4919);
    assert!(accepted.success, "{:?}", accepted.metadata);
    assert_eq!(REPLAY_COPIED_BYTES.load(Ordering::SeqCst), 4407);
    assert_eq!(
        accepted
            .files
            .iter()
            .map(|f| f.resource_id.unwrap())
            .collect::<Vec<_>>(),
        [6, 2, 4]
    );
    assert!(accepted
        .files
        .iter()
        .all(|f| f.bytes == photo.still_bytes().unwrap()));
    assert_eq!(accepted.into_envelope().unwrap().len(), 4919);
    for limit in [4918, 4407, 3450] {
        reject(&source, &[6, 2, 4], limit, "response budget exceeded");
    }
    for (ids, message) in [
        (&[2, 2][..], "invalid/duplicate resource ID"),
        (&[2, 999][..], "resource ID missing"),
    ] {
        reject(&source, ids, 100_000, message);
    }
    println!("PASS: exact boundary, wire overhead, aliases, and complete selection before copy");
}
