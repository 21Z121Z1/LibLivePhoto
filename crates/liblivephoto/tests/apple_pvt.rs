use liblivephoto::{
    apple, pvt, Input, MotionPhoto, PairingPolicy, ParseOptions, TargetProfile, WritePolicy,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn scratch() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "llp-pvt-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}

fn demo(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/web/public/demo")
            .join(name),
    )
    .unwrap()
}

#[test]
fn motion_photo_package_injects_one_identifier_and_keeps_encoded_video() {
    let source = demo("motion-photo.jpg");
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let policy = WritePolicy {
        pairing: PairingPolicy::Set("motion-pvt-1".into()),
        ..WritePolicy::default()
    };
    let output = photo
        .plan(TargetProfile::ApplePair, policy)
        .execute()
        .unwrap();
    let still = &output.files[0];
    let movie = &output.files[1];
    let packaged = pvt::package_pair(
        "motion",
        &still.mime,
        still.bytes.clone(),
        movie.bytes.clone(),
    )
    .unwrap();
    assert_eq!(packaged.content_identifier, "MOTION-PVT-1");
    assert_eq!(packaged.still_name, "motion.jpg");
    assert_eq!(packaged.movie_name, "motion.mov");
    assert_eq!(packaged.metadata, pvt::metadata_plist());
    assert!(packaged
        .presentation
        .equivalent(photo.presentation_time().unwrap()));
    assert_eq!(
        apple::read_still_identifier(&packaged.still)
            .unwrap()
            .as_deref(),
        Some("MOTION-PVT-1")
    );
    assert_eq!(
        apple::read_movie_identifier(&packaged.movie)
            .unwrap()
            .as_deref(),
        Some("MOTION-PVT-1")
    );
    assert_eq!(
        apple::media_payloads(&packaged.movie).unwrap(),
        apple::media_payloads(photo.motion_video_bytes().unwrap()).unwrap()
    );
    let root = scratch();
    let package = root.join("motion.pvt");
    fs::create_dir(&package).unwrap();
    fs::write(package.join(&packaged.still_name), &packaged.still).unwrap();
    fs::write(package.join(&packaged.movie_name), &packaged.movie).unwrap();
    fs::write(package.join(packaged.metadata_name), packaged.metadata).unwrap();
    let read = pvt::read_directory(&package, 32 * 1024 * 1024).unwrap();
    assert_eq!(read.still, packaged.still);
    assert_eq!(read.movie, packaged.movie);
    fs::write(package.join("extra.jpg"), &packaged.still).unwrap();
    assert!(pvt::read_directory(&package, 32 * 1024 * 1024).is_err());
    let archive = root.join("archive.pvt");
    fs::write(&archive, b"PK\x03\x04not-a-directory").unwrap();
    assert!(pvt::read_directory(&archive, 32).is_err());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn existing_apple_pair_package_keeps_both_member_byte_strings() {
    let still = demo("apple-still.jpg");
    let movie = demo("apple-paired.mov");
    let photo = MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let identifier = photo.asset().pairing.identifier.clone().unwrap();
    let copied = photo
        .plan(TargetProfile::ApplePair, WritePolicy::default())
        .execute()
        .unwrap();
    assert_eq!(copied.files[0].bytes, still);
    assert_eq!(copied.files[1].bytes, movie);
    let packaged = pvt::package_pair(
        "IMG_0010",
        &copied.files[0].mime,
        copied.files[0].bytes.clone(),
        copied.files[1].bytes.clone(),
    )
    .unwrap();
    assert_eq!(packaged.still, still);
    assert_eq!(packaged.movie, movie);
    assert_eq!(packaged.content_identifier, identifier);
    assert_eq!(packaged.still_name, "IMG_0010.jpg");
    assert_eq!(packaged.movie_name, "IMG_0010.mov");
}
