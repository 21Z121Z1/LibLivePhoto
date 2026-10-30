mod common;
use liblivephoto::*;

fn movie() -> Vec<u8> {
    [
        common::movie(1000, 1),
        common::vivo_stream_box(&[
            (0xdec0de01, 255, b"\0\xffopaque"),
            (0xdec0de01, 10, b"second"),
            (3, 0, b""),
        ]),
        common::vivo_footer_box(),
    ]
    .concat()
}
fn motion(video: &[u8]) -> Vec<u8> {
    MotionPhoto::parse(
        Input::Parts {
            still: &[255, 216, 255, 217],
            movie: video,
            presentation: MediaTime::new(1, 1),
        },
        ParseOptions::strict(),
    )
    .unwrap()
    .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
    .execute()
    .unwrap()
    .files[0]
        .bytes
        .clone()
}
fn slice(data: &[u8], r: ByteRange) -> &[u8] {
    &data[r.lower_bound as usize..r.upper_bound as usize]
}

#[test]
fn observed_stream_framing_borrows_bytes_and_retains_duplicate_keys_and_unknown_tags() {
    let bytes = movie();
    let before = bytes.clone();
    let ext = inspect_vivo_media_extensions(&bytes).unwrap().unwrap();
    assert_eq!(ext.streams.len(), 3);
    assert!(ext.duplicate_keys);
    for (stream, payload) in
        ext.streams
            .iter()
            .zip([&b"\0\xffopaque"[..], &b"second"[..], &b""[..]])
    {
        assert_eq!(stream.raw_payload, payload);
        assert_eq!(slice(&bytes, stream.payload_extent), payload);
        assert_eq!(slice(&bytes, stream.framing_extent), b"streamdata");
        assert_eq!(&slice(&bytes, stream.directory_extent)[..10], b"streaminfo");
    }
    assert_eq!(ext.streams[0].encoding, 255);
    assert_eq!(ext.streams[0].key, ext.streams[1].key);
    assert_eq!(ext.footer.as_ref().unwrap().image_time, Some(31));
    assert!(std::str::from_utf8(ext.footer.as_ref().unwrap().raw_json)
        .unwrap()
        .contains("1.234567890123456789e100"));
    assert_eq!(bytes, before);
}

#[test]
fn top_level_association_ignores_nested_magic_and_refuses_multiple_known_boxes() {
    let mut bytes = common::movie(1000, 1);
    bytes.extend(common::boxed(
        b"free",
        &common::vivo_stream_box(&[(1, 10, b"opaque")]),
    ));
    assert!(inspect_vivo_media_extensions(&bytes).unwrap().is_none());
    assert!(
        inspect_vivo_media_extensions(b"vivoMediaEStream streamcount")
            .unwrap()
            .is_none()
    );
    let extension = common::vivo_stream_box(&[(1, 10, b"opaque")]);
    bytes.extend(&extension);
    bytes.extend(&extension);
    assert!(matches!(
        inspect_vivo_media_extensions(&bytes),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    let bytes = [
        common::movie(1000, 1),
        common::vivo_footer_box(),
        common::vivo_footer_box(),
    ]
    .concat();
    assert!(matches!(
        inspect_vivo_media_extensions(&bytes),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
}

#[test]
fn extended_box_headers_and_footer_ranges_stay_inside_the_enclosing_box() {
    let original = common::vivo_stream_box(&[(7, 10, b"opaque")]);
    let large = [
        1u32.to_be_bytes().to_vec(),
        b"uuid".to_vec(),
        ((original.len() + 8) as u64).to_be_bytes().to_vec(),
        original[8..].to_vec(),
    ]
    .concat();
    let bytes = [common::movie(1000, 1), large, common::vivo_footer_box()].concat();
    let ext = inspect_vivo_media_extensions(&bytes).unwrap().unwrap();
    assert_eq!(ext.streams[0].raw_payload, b"opaque");
    let footer = ext.footer.unwrap();
    let wrapper = ext.footer_box_extent.unwrap();
    assert!(wrapper.lower_bound < footer.extent.lower_bound);
    assert_eq!(footer.extent.upper_bound, wrapper.upper_bound);
    let mut bad = common::vivo_footer_box();
    // A known UUID without its observed vivo prefix cannot import a JSON prefix
    // from outside the wrapper or silently become an arbitrary CameraAlbum tail.
    bad.drain(24..28);
    let len = bad.len() as u32;
    bad[..4].copy_from_slice(&len.to_be_bytes());
    let bytes = [common::movie(1000, 1), bad].concat();
    assert!(inspect_vivo_media_extensions(&bytes).is_err());
}

#[test]
fn malformed_markers_lengths_counts_and_aggregate_budgets_fail_before_partial_results() {
    let base = movie();
    let ext = inspect_vivo_media_extensions(&base).unwrap().unwrap();
    let marker = ext.streams[0].framing_extent.lower_bound as usize;
    let directory = ext.streams[0].directory_extent.lower_bound as usize;
    let count = ext.stream_directory_extent.unwrap().upper_bound as usize - 2;
    for offset in [marker, directory, count - 11] {
        let mut bad = base.clone();
        bad[offset] ^= 1;
        assert!(matches!(
            inspect_vivo_media_extensions(&bad),
            Err(MotionPhotoReadError::MetadataMismatch(_))
        ));
    }
    let mut bad = base.clone();
    bad[directory + 15..directory + 19].copy_from_slice(&99u32.to_be_bytes());
    assert!(matches!(
        inspect_vivo_media_extensions(&bad),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
    let mut bad = base.clone();
    bad[directory + 15..directory + 19].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(matches!(
        inspect_vivo_media_extensions(&bad),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let mut bad = base.clone();
    bad[count..count + 2].copy_from_slice(&4097u16.to_be_bytes());
    assert!(matches!(
        inspect_vivo_media_extensions(&bad),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let bytes = [common::movie(1000, 1), common::vivo_stream_box(&[])].concat();
    assert!(inspect_vivo_media_extensions(&bytes)
        .unwrap()
        .unwrap()
        .streams
        .is_empty());
    // Recognized BMFF length overflow is a bounded unsupported layout.
    let mut bad = base;
    bad[..4].copy_from_slice(&1u32.to_be_bytes());
    bad[8..16].copy_from_slice(&u64::MAX.to_be_bytes());
    assert!(matches!(
        inspect_vivo_media_extensions(&bad),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
}

#[test]
fn parsed_asset_and_lossless_writer_retain_the_original_movie_and_uninterpreted_streams() {
    let video = movie();
    let bytes = motion(&video);
    let p = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict()).unwrap();
    assert_eq!(p.vivo_media_extensions().unwrap().unwrap().streams.len(), 3);
    let out = p
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .execute()
        .unwrap();
    let reopened = MotionPhoto::parse(
        Input::SingleFile(&out.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(reopened.motion_video_bytes().unwrap(), video);
    assert_eq!(reopened.presentation_time(), p.presentation_time());
    assert!(!reopened
        .resources()
        .any(|r| r.role == ResourceRole::GeometryDepthVideo));
}

#[test]
fn metadata_blocks_extract_exact_payloads_with_provider_filters_and_range_ownership() {
    let bytes = motion(&movie());
    let engine = MetadataEngine::new();
    let collection = engine.parse(&bytes, &HandlerOptions::default()).unwrap();
    let d = collection
        .decoded
        .iter()
        .find(|d| d.provider == "vivo_media")
        .unwrap();
    assert!(collection.verified_motion);
    assert!(!d.motion_declaration);
    assert!(d.video_extents.is_empty());
    assert_eq!(d.value["duplicateKeys"], true);
    assert_eq!(d.value["pairingVerified"], false);
    assert_eq!(d.value["footer"]["imageTimeUnit"], "unverified");
    for (record, payload) in d.value["streams"].as_array().unwrap().iter().zip([
        &b"\0\xffopaque"[..],
        &b"second"[..],
        &b""[..],
    ]) {
        let mut actual = Vec::new();
        collection
            .extract_block(
                record["blockIndex"].as_u64().unwrap() as usize,
                &bytes,
                &mut actual,
                64,
            )
            .unwrap();
        assert_eq!(actual, payload);
        assert_eq!(record["semantic"], "opaque");
    }
    let imported = MetadataCollection::from_json(collection.json().to_string().as_bytes()).unwrap();
    assert!(!imported.verified_motion);
    assert_eq!(imported.blocks, collection.blocks);
    let filtered = engine
        .parse(
            &bytes,
            &HandlerOptions {
                disable_third_party: false,
                enabled_third_party: vec!["google".into()],
            },
        )
        .unwrap();
    assert!(!filtered.decoded.iter().any(|d| d.provider == "vivo_media"));
    assert!(!filtered
        .blocks
        .iter()
        .any(|b| b.format_identifier.starts_with("vivo.")));
    let mut bad = bytes.clone();
    let marker = bad.windows(11).position(|w| w == b"streamcount").unwrap();
    bad[marker] ^= 1;
    let bad_collection = engine.parse(&bad, &HandlerOptions::default()).unwrap();
    assert!(bad_collection
        .diagnostics
        .iter()
        .any(|d| d.starts_with("vivo_media:")));
    assert!(!bad_collection
        .decoded
        .iter()
        .any(|d| d.provider == "vivo_media"));
    let trailer = bad_collection
        .blocks
        .iter()
        .position(|b| b.format_identifier == "container.trailer")
        .unwrap();
    let mut raw = Vec::new();
    bad_collection
        .extract_block(trailer, &bad, &mut raw, bad.len() as u64)
        .unwrap();
    assert_eq!(raw, slice(&bad, bad_collection.blocks[trailer].extents[0]));

    use std::io::{Seek, SeekFrom, Write};
    let path = std::env::temp_dir().join(format!(
        "llp-vivo-{}-{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(&[0; 17]).unwrap();
    file.write_all(&bytes).unwrap();
    file.write_all(&[0; 7]).unwrap();
    file.seek(SeekFrom::Start(7)).unwrap();
    let ranged = engine
        .parse_range(
            FileRange {
                file: &file,
                offset: 17,
                length: bytes.len() as u64,
            },
            &HandlerOptions::default(),
        )
        .unwrap();
    assert_eq!(ranged.blocks, collection.blocks);
    assert_eq!(file.stream_position().unwrap(), 7);
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        let fd = engine
            .parse_fd(
                file.as_fd(),
                17,
                bytes.len() as u64,
                &HandlerOptions::default(),
            )
            .unwrap();
        assert_eq!(fd.blocks, collection.blocks);
        assert_eq!(file.stream_position().unwrap(), 7);
    }
    drop(file);
    std::fs::remove_file(path).unwrap();
}
