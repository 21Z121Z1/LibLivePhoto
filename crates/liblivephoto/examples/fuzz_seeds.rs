#[path = "../tests/common/mod.rs"]
mod common;
fn main() -> std::io::Result<()> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fuzz/corpus".into());
    for target in ["asset_input", "container_headers", "remux"] {
        let dir = std::path::Path::new(&root).join(target);
        std::fs::create_dir_all(&dir)?;
        for (name, bytes) in [
            ("single-track", common::input(123, 1)),
            ("secondary-track", common::input(1_234_567, 2)),
            ("ambiguous-tracks", common::input(123, 3)),
            ("movie", common::movie(1000, 2)),
            ("icc-structure44", include_bytes!("../../../fixtures/syntax/icc-structure-v4.4.icc").to_vec()),
            ("aac-pce-wave", include_bytes!("../../../fixtures/syntax/aac-pce-wave.mov").to_vec()),
            ("aac-pce-stz2", include_bytes!("../../../fixtures/syntax/aac-pce-stz2.mov").to_vec()),
            ("score-empty", vec![0xc0, 0]),
            ("camera-album",[vec![255,216,255,217],common::camera_album_envelope(b"vivo",br#"{"version":2202,"com.android.camera.imageTime":31,"com.android.camera.livephoto":"synthetic-camera-id-00000000"}"#,b"synthetic-camera-id-00000000",false)].concat()),
            ("camera-album55",[vec![255,216,255,217],common::camera_album_envelope(b"vivo",br#"{"version":2202,"opaque":["unchanged",1.23456789e100]}"#,b"synthetic-camera-id-00000000",true)].concat()),
            ("vivo-media",[common::movie(1000,1),common::vivo_stream_box(&[(7,255,b"opaque"),(7,10,b"second")]),common::vivo_footer_box()].concat()),
            ("heif-idat", common::heif(b"avif")),
            (
                "heif-colr-pair",
                common::heif_with_properties(&[
                    liblivephoto_format::isobmff::make_ispe_box(2, 3).unwrap(),
                    common::boxed(b"colr", b"profopaque-profile"),
                    common::boxed(b"colr", b"nclx\0\x02\0\x02\0\x09\x80"),
                ]),
            ),
            (
                "agtm-edits",
                common::metadata_edits(
                    common::metadata_movie(
                        b"it35",
                        &[5, 0xb5, 0, 0x90, 0, 1],
                        &[(&[0, 0], 1000)],
                        500,
                    ),
                    1000,
                    &[(1000, 500, 1, 0), (500, -1, 1, 0), (1000, 500, 1, 0)],
                ),
            ),
            (
                "agtm-overlap",
                common::overlapping_metadata_movie(b"it35", &[5, 0xb5, 0, 0x90, 0, 1], &[0, 0], 3),
            ),
            ("score-unknown", vec![0xc0, 4, 0xd0, 0, 0xd1, 0]),
            (
                "score-video-edits",
                common::movie_track_edits(
                    common::metadata_movie(
                        b"mett",
                        b"\0application/motionphoto-image-meta\0",
                        &[(
                            &[
                                0xc0, 20, 0xc1, 18, 0, 0, 0, 7, 0xc3, 12, 0x3f, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                            ],
                            2000,
                        )],
                        0,
                    ),
                    0,
                    1000,
                    &[(1000, 0, 1, 0), (500, -1, 1, 0), (1000, 0, 1, 0)],
                ),
            ),
            (
                "score-movie",
                common::metadata_movie(
                    b"mett",
                    b"\0application/motionphoto-image-meta\0",
                    &[(&[0xc0, 4, 0xd0, 0, 0xd1, 0], 1000)],
                    0,
                ),
            ),
            (
                "metadata-collection",
                liblivephoto::MetadataEngine::new()
                    .parse(
                        &common::input(123, 1),
                        &liblivephoto::HandlerOptions::default(),
                    )
                    .unwrap()
                    .json()
                    .to_string()
                    .into_bytes(),
            ),
            ("overflow-box", vec![255; 32]),
            ("truncated-xml", common::jpeg("<x:xmpmeta><rdf:RDF>")),
            ("empty", vec![]),
            (
                "nested-xml",
                common::jpeg(&format!("{}{}", "<a>".repeat(140), "</a>".repeat(140))),
            ),
            (
                "vendor-trailers",
                [
                    common::input(123, 1),
                    b"JXRS\0\0\0\xffLPEX lpexLivePhotoExtension {\"coverFramePts\":123.9}".to_vec(),
                ]
                .concat(),
            ),
        ] {
            std::fs::write(dir.join(name), bytes)?;
        }
        std::fs::write(
            dir.join("sef"),
            common::sef(&[
                (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
                (0x1234, "Unknown", vec![0; 4]),
            ]),
        )?;
    }
    Ok(())
}
