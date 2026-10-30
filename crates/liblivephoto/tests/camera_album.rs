mod common;
use liblivephoto::*;

const ID: &[u8; 28] = b"synthetic-camera-id-00000000";
fn envelope(json: &[u8], tail55: bool) -> Vec<u8> {
    [
        vec![255, 216, 255, 217],
        common::camera_album_envelope(b"vivo", json, ID, tail55),
    ]
    .concat()
}
fn fields() -> Vec<u8> {
    format!(r#"{{"version":2202,"com.android.camera.livephoto":"{}","com.android.camera.imageTime":31,"unknown":{{"opaque":[1,null,"unaltered"]}}}}"#,std::str::from_utf8(ID).unwrap()).into_bytes()
}
#[test]
fn normal_and_static_only_extended_layouts_expose_exact_bytes_without_time_or_pair_guesses() {
    let raw = fields();
    for tail55 in [false, true] {
        let bytes = envelope(&raw, tail55);
        let before = bytes.clone();
        let metadata = inspect_camera_album_footer(&bytes).unwrap().unwrap();
        assert_eq!(metadata.raw_json, raw);
        assert_eq!(metadata.version, Some(2202));
        assert_eq!(metadata.image_time, Some(31));
        assert_eq!(
            metadata.identifier_agreement,
            DeclaredIdentifierAgreement::Equal
        );
        assert_eq!(metadata.prefix, CameraAlbumPrefix::Vivo);
        assert_eq!(
            metadata.envelope_identifier,
            std::str::from_utf8(ID).unwrap()
        );
        assert_eq!(
            metadata.layout,
            if tail55 {
                CameraAlbumLayout::Observed55ByteTail
            } else {
                CameraAlbumLayout::LengthDelimited43
            }
        );
        assert_eq!(
            &bytes[metadata.json_extent.lower_bound as usize
                ..metadata.json_extent.upper_bound as usize],
            raw
        );
        assert_eq!(metadata.extent.lower_bound, 4);
        assert!(metadata
            .raw_json
            .windows(b"unaltered".len())
            .any(|w| w == b"unaltered"));
        assert_eq!(bytes, before);
        assert!(MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict()).is_err());
    }
}

#[test]
fn full_footer_block_and_provider_controls_do_not_certify_a_movie() {
    let bytes = envelope(&fields(), false);
    let collection = MetadataEngine::new()
        .parse(&bytes, &HandlerOptions::default())
        .unwrap();
    assert!(!collection.verified_motion);
    assert!(collection.declares_motion(&HandlerOptions::default()));
    let decoded = collection
        .decoded
        .iter()
        .find(|d| d.provider == "camera_album")
        .unwrap();
    assert_eq!(decoded.value["pairingVerified"], false);
    assert_eq!(decoded.value["imageTimeUnit"], "unverified");
    assert_eq!(decoded.value["imageTime"], "31");
    assert!(decoded.video_extents.is_empty());
    let index = decoded.block_index.unwrap();
    let mut raw = Vec::new();
    collection
        .extract_block(index, &bytes, &mut raw, 65536)
        .unwrap();
    assert_eq!(raw, &bytes[4..]);
    assert!(collection.blocks[index].raw_complete);
    let imported = MetadataCollection::from_json(collection.json().to_string().as_bytes()).unwrap();
    assert!(!imported.verified_motion);
    assert_eq!(imported.blocks, collection.blocks);
    for options in [
        HandlerOptions {
            disable_third_party: true,
            ..Default::default()
        },
        HandlerOptions {
            enabled_third_party: vec!["samsung_sef".into()],
            ..Default::default()
        },
    ] {
        let disabled = MetadataEngine::new().parse(&bytes, &options).unwrap();
        assert!(!disabled
            .decoded
            .iter()
            .any(|d| d.provider == "camera_album"));
        assert!(!disabled
            .blocks
            .iter()
            .any(|b| b.format_identifier == "camera-album.footer"));
    }
}

#[test]
fn unknown_versions_and_identifier_conflicts_are_observed_without_repair() {
    let raw=br#"{"version":9999,"com.android.camera.livephoto":"different","com.android.camera.imageTime":-1}"#;
    let bytes = envelope(raw, false);
    let metadata = inspect_camera_album_footer(&bytes).unwrap().unwrap();
    assert_eq!(metadata.version, Some(9999));
    assert_eq!(metadata.image_time, Some(-1));
    assert_eq!(
        metadata.identifier_agreement,
        DeclaredIdentifierAgreement::Different
    );
    assert_eq!(metadata.raw_json, raw);
    let collection = MetadataEngine::new()
        .parse(&bytes, &HandlerOptions::default())
        .unwrap();
    assert!(collection
        .diagnostics
        .iter()
        .any(|d| d.contains("identifiers disagree")));
    let absent = envelope(br#"{"unknown":true}"#, false);
    let metadata = inspect_camera_album_footer(&absent).unwrap().unwrap();
    assert_eq!(
        metadata.identifier_agreement,
        DeclaredIdentifierAgreement::Absent
    );
    assert!(metadata.image_time.is_none());
    assert!(metadata.version.is_none());
    assert!(!MetadataEngine::new()
        .parse(&absent, &HandlerOptions::default())
        .unwrap()
        .declares_motion(&HandlerOptions::default()));
}

#[test]
fn unknown_json_numbers_and_escaping_remain_original_text_in_the_public_projection() {
    let raw = br#"{"version":2202,"unknown":1.234567890123456789e100,"text":"\u0061\\\""}"#;
    let bytes = envelope(raw, false);
    assert!(
        inspect_camera_album_footer(&bytes)
            .unwrap()
            .unwrap()
            .raw_json
            == raw
    );
    let collection = MetadataEngine::new()
        .parse(&bytes, &HandlerOptions::default())
        .unwrap();
    let decoded = collection
        .decoded
        .iter()
        .find(|d| d.provider == "camera_album")
        .unwrap();
    assert!(decoded.value["rawJson"].as_str().unwrap().as_bytes() == raw);
    assert!(decoded.value.get("rawFields").is_none());
}

#[test]
fn duplicate_keys_including_escaped_aliases_and_nested_conflicts_fail_before_last_wins() {
    for raw in [
        br#"{"version":1,"version":2}"#.as_slice(),
        br#"{"version":1,"\u0076ersion":1}"#,
        br#"{"unknown":{"x":1,"x":1}}"#,
        br#"{"com.android.camera.imageTime":1.5}"#,
        br#"{"com.android.camera.livephoto":17}"#,
        br#"{"version":"2202"}"#,
        b"{\"version\":1} {\"other\":1}",
        b"[]",
        b"{\"x\":NaN}",
    ] {
        let bytes = envelope(raw, false);
        assert!(matches!(
            inspect_camera_album_footer(&bytes),
            Err(MotionPhotoReadError::MetadataMismatch(_))
        ));
        let collection = MetadataEngine::new()
            .parse(&bytes, &HandlerOptions::default())
            .unwrap();
        assert!(collection
            .diagnostics
            .iter()
            .any(|d| d.starts_with("camera_album:")));
        assert!(!collection
            .decoded
            .iter()
            .any(|d| d.provider == "camera_album"));
    }
    let bytes = envelope(
        br#"{"version":2202,"x":"quote\" colon: bracket} slash\\","a":{"same":1},"b":{"same":2}}"#,
        false,
    );
    assert!(inspect_camera_album_footer(&bytes).unwrap().is_some());
}

#[test]
fn boundary_length_magic_separator_and_encoding_errors_never_recover_by_guessing() {
    let valid = envelope(&fields(), false);
    for prefix in 0..valid.len() {
        assert!(inspect_camera_album_footer(&valid[..prefix]).is_ok());
    }
    let mut bad = valid.clone();
    let marker = bad.len() - 58;
    bad[marker - 4..marker].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(matches!(
        inspect_camera_album_footer(&bad),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let mut bad = valid.clone();
    bad[marker + 11..marker + 15].copy_from_slice(&46u32.to_be_bytes());
    assert!(matches!(
        inspect_camera_album_footer(&bad),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
    let mut bad = valid.clone();
    let n = bad.len();
    bad[n - 15] = 0;
    assert!(matches!(
        inspect_camera_album_footer(&bad),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
    let mut bad = valid.clone();
    bad[n - 43] = 0;
    assert!(matches!(
        inspect_camera_album_footer(&bad),
        Err(MotionPhotoReadError::MetadataMismatch(_))
    ));
    let mut bad = valid.clone();
    bad.extend(b"unframed tail");
    assert!(inspect_camera_album_footer(&bad).unwrap().is_none());
    let mut extended = envelope(&fields(), true);
    let n = extended.len();
    extended[n - 55] = 1;
    assert!(matches!(
        inspect_camera_album_footer(&extended),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
    let mut unsupported = valid.clone();
    unsupported.splice(n - 55..n - 55, [0]);
    assert!(matches!(
        inspect_camera_album_footer(&unsupported),
        Err(MotionPhotoReadError::UnsupportedLayout(_))
    ));
}

#[test]
fn json_allocation_and_depth_budgets_are_explicit() {
    let huge = format!("{{\"opaque\":\"{}\"}}", "a".repeat(65536));
    assert!(matches!(
        inspect_camera_album_footer(&envelope(huge.as_bytes(), false)),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let deep = format!("{{\"x\":{}0{}}}", "[".repeat(40), "]".repeat(40));
    assert!(matches!(
        inspect_camera_album_footer(&envelope(deep.as_bytes(), false)),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let dense = format!("{{\"x\":[{}]}}", vec!["0"; 9000].join(","));
    assert!(matches!(
        inspect_camera_album_footer(&envelope(dense.as_bytes(), false)),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn file_and_borrowed_range_keep_relative_extents_cursor_and_snapshot_fences() {
    use std::{
        fs::OpenOptions,
        io::{Seek, SeekFrom, Write},
    };
    let path = std::env::temp_dir().join(format!("llp-camera-album-range-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    struct Remove(std::path::PathBuf);
    impl Drop for Remove {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _owned = Remove(path);
    let bytes = envelope(&fields(), false);
    file.write_all(&[0; 13]).unwrap();
    file.write_all(&bytes).unwrap();
    file.write_all(&[0; 4]).unwrap();
    file.seek(SeekFrom::Start(7)).unwrap();
    let engine = MetadataEngine::new();
    let options = HandlerOptions::default();
    let expected = engine.parse(&bytes, &options).unwrap();
    let range = FileRange {
        file: &file,
        offset: 13,
        length: bytes.len() as u64,
    };
    assert_eq!(engine.parse_range(range, &options).unwrap(), expected);
    assert_eq!(file.stream_position().unwrap(), 7);
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        assert_eq!(
            engine
                .parse_fd(file.as_fd(), 13, bytes.len() as u64, &options)
                .unwrap(),
            expected
        );
        assert_eq!(file.stream_position().unwrap(), 7);
    }
    let including_extra = engine
        .parse_range(
            FileRange {
                file: &file,
                offset: 13,
                length: bytes.len() as u64 + 1,
            },
            &options,
        )
        .unwrap();
    assert!(!including_extra
        .decoded
        .iter()
        .any(|d| d.provider == "camera_album"));
    assert_eq!(file.stream_position().unwrap(), 7);
    drop(file);
}
