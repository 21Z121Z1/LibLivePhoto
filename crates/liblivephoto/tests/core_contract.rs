mod common;
use liblivephoto::*;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "liblivephoto-core-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn memory_path_file_and_descriptor_slices_have_identical_results_and_preserve_cursors() {
    let input = common::input(123_000, 2);
    let temp = Temp::new();
    let path = temp.0.join("source.jpg");
    fs::write(&path, &input).unwrap();
    let memory = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    let opened = MotionPhoto::open(PathInput::SingleFile(&path), ParseOptions::strict()).unwrap();
    let enclosing = temp.0.join("enclosing.bin");
    let mut file = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(enclosing)
        .unwrap();
    file.write_all(&[b"unrelated-prefix".as_slice(), &input, b"unrelated-suffix"].concat())
        .unwrap();
    file.seek(SeekFrom::Start(3)).unwrap();
    let ranged = MotionPhoto::open_file(
        FileInput::SingleFile(FileRange {
            file: &file,
            offset: 16,
            length: input.len() as u64,
        }),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(file.stream_position().unwrap(), 3);
    for photo in [&opened, &ranged] {
        assert_eq!(photo.asset(), memory.asset());
        assert_eq!(photo.xmp_bytes().unwrap(), memory.xmp_bytes().unwrap());
        assert_eq!(
            photo.motion_video_bytes().unwrap(),
            memory.motion_video_bytes().unwrap()
        );
        photo.validate().unwrap();
        let mut bytes = Vec::new();
        photo.extract(photo.still(), &mut bytes).unwrap();
        assert_eq!(bytes, memory.still_bytes().unwrap());
        assert_eq!(
            photo
                .plan(TargetProfile::PreserveSource, WritePolicy::default())
                .execute()
                .unwrap()
                .files[0]
                .bytes,
            input
        );
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        let fd = MotionPhoto::open_fd(file.as_fd(), 16, input.len() as u64, ParseOptions::strict())
            .unwrap();
        assert_eq!(fd.asset(), memory.asset());
        assert_eq!(file.stream_position().unwrap(), 3);
    }
    for (offset, length) in [
        (0, 0),
        (u64::MAX, 2),
        (16, u64::MAX),
        (16, input.len() as u64 + 1000),
    ] {
        assert!(matches!(
            MotionPhoto::open_file(
                FileInput::SingleFile(FileRange {
                    file: &file,
                    offset,
                    length
                }),
                ParseOptions::strict()
            ),
            Err(MotionPhotoReadError::InvalidInputRange)
        ));
    }
    assert!(matches!(
        MotionPhoto::open_file(
            FileInput::SingleFile(FileRange {
                file: &file,
                offset: 16,
                length: input.len() as u64
            }),
            ParseOptions {
                max_input_bytes: input.len() - 1,
                ..ParseOptions::strict()
            }
        ),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn parts_from_paths_and_ranges_build_identical_motion_photos_with_a_combined_budget() {
    let temp = Temp::new();
    let still = vec![255, 216, 255, 217];
    let movie = common::movie(1000, 1);
    let a = temp.0.join("still.jpg");
    let b = temp.0.join("movie.mp4");
    fs::write(&a, &still).unwrap();
    fs::write(&b, &movie).unwrap();
    let presentation = MediaTime::new(123, 1000);
    let paths = MotionPhoto::open(
        PathInput::Parts {
            still: &a,
            movie: &b,
            presentation,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    let af = File::open(&a).unwrap();
    let bf = File::open(&b).unwrap();
    let input = FileInput::Parts {
        still: FileRange {
            file: &af,
            offset: 0,
            length: still.len() as u64,
        },
        movie: FileRange {
            file: &bf,
            offset: 0,
            length: movie.len() as u64,
        },
        presentation,
    };
    let ranges = MotionPhoto::open_file(input, ParseOptions::strict()).unwrap();
    assert_eq!(ranges.asset(), paths.asset());
    let out = paths
        .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
        .execute()
        .unwrap();
    assert_eq!(
        out,
        ranges
            .plan(TargetProfile::AndroidJpeg, WritePolicy::default())
            .execute()
            .unwrap()
    );
    assert!(matches!(
        MotionPhoto::open_file(
            input,
            ParseOptions {
                max_input_bytes: still.len() + movie.len() - 1,
                ..ParseOptions::strict()
            }
        ),
        Err(MotionPhotoReadError::InputLimit)
    ));
}

#[test]
fn avif_build_preserves_items_video_xmp_and_time_without_pixel_encoding() {
    let still = common::heif(b"avif");
    let movie = common::movie(1000, 1);
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: MediaTime::new(1, 4),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(photo.still().mime, "image/avif");
    assert_eq!(photo.asset().containers[0].kind, ContainerKind::Avif);
    let out = photo
        .plan(TargetProfile::AndroidAvif, WritePolicy::default())
        .execute()
        .unwrap();
    let parsed = MotionPhoto::parse(
        Input::SingleFile(&out.files[0].bytes),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(parsed.asset().layout, PhysicalLayout::BmffVideoBox);
    assert_eq!(parsed.still().mime, "image/avif");
    assert_eq!(parsed.motion_video_bytes().unwrap(), movie);
    assert!(photo
        .presentation_time()
        .unwrap()
        .equivalent(parsed.presentation_time().unwrap()));
    assert!(
        String::from_utf8_lossy(parsed.xmp_bytes().unwrap().as_ref().unwrap())
            .contains("image/avif")
    );
    for item in photo.resources().filter(|r| r.container_item_id.is_some()) {
        let target = parsed
            .resources()
            .find(|r| r.container_item_id == item.container_item_id)
            .unwrap();
        let mut a = Vec::new();
        let mut b = Vec::new();
        photo.extract(item, &mut a).unwrap();
        parsed.extract(target, &mut b).unwrap();
        assert_eq!(a, b);
    }
    assert!(!photo
        .plan(TargetProfile::AndroidHeif, WritePolicy::default())
        .can_execute());
    assert!(!photo
        .plan(TargetProfile::ApplePair, WritePolicy::default())
        .can_execute());
    assert!(photo.xmp_bytes().unwrap().is_none());
}

#[test]
fn malformed_and_conflicting_declared_time_cannot_silently_become_unknown() {
    for timestamp in ["n/a", "9223372036854775808", "-2"] {
        let movie = common::movie(1000, 1);
        let xmp = common::xmp(movie.len(), 123, "").replace(
            "TimestampUs=\"123\"",
            &format!("TimestampUs=\"{timestamp}\""),
        );
        let input = [common::jpeg(&xmp), movie].concat();
        assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
    }
    let movie = common::movie(1000, 1);
    let xmp = common::xmp(movie.len(), 123, "").replace("</rdf:RDF>", "<rdf:Description xmlns:c=\"http://ns.google.com/photos/1.0/camera/\" c:MotionPhotoPresentationTimestampUs=\"456\"/></rdf:RDF>");
    assert!(MotionPhoto::parse(
        Input::SingleFile(&[common::jpeg(&xmp), movie].concat()),
        ParseOptions::strict()
    )
    .is_err());
}

#[test]
fn inconsistent_declared_video_mime_is_diagnosed_or_rejected_by_mode() {
    let mut movie = common::movie(1000, 1);
    movie[8..12].copy_from_slice(b"qt  ");
    let input = [common::jpeg(&common::xmp(movie.len(), 123, "")), movie].concat();
    assert!(matches!(
        MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()),
        Err(MotionPhotoReadError::MetadataMismatch("video MIME"))
    ));
    let compatible =
        MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
    assert_eq!(compatible.motion_video().mime, "video/quicktime");
    assert!(compatible
        .diagnostics()
        .iter()
        .any(|d| matches!(d, Diagnostic::MetadataMismatch { field, .. } if field == "video MIME")));
}

#[test]
fn element_camera_properties_are_supported() {
    let movie = common::movie(1000, 1);
    let xmp = common::xmp(movie.len(),123_000,"")
        .replace("Camera:MotionPhoto=\"1\"", "")
        .replace("Camera:MotionPhotoVersion=\"1\"", "")
        .replace("Camera:MotionPhotoPresentationTimestampUs=\"123000\"", "")
        .replace("<Container:Directory>","<Camera:MotionPhoto>1</Camera:MotionPhoto><Camera:MotionPhotoVersion>1</Camera:MotionPhotoVersion><Camera:MotionPhotoPresentationTimestampUs> 123000 </Camera:MotionPhotoPresentationTimestampUs><Container:Directory>");
    let bytes = [common::jpeg(&xmp), movie.clone()].concat();
    let photo = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict()).unwrap();
    assert_eq!(
        photo.presentation_time(),
        MediaTime::new(123_000, 1_000_000)
    );
}

#[test]
fn existing_apple_pair_validation_also_applies_to_two_ranges_in_one_file() {
    let input = common::input(123_000, 1);
    let source = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).unwrap();
    let pair = source
        .plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing: PairingPolicy::Set("RANGED-PAIR".into()),
                unmapped: UnmappedPolicy::PreserveSidecars,
            },
        )
        .execute()
        .unwrap();
    let still = &pair.files[0].bytes;
    let movie = &pair.files[1].bytes;
    let temp = Temp::new();
    let path = temp.0.join("association.bin");
    fs::write(
        &path,
        [b"prefix".as_slice(), still, b"gap", movie, b"suffix"].concat(),
    )
    .unwrap();
    let mut file = File::open(path).unwrap();
    file.seek(SeekFrom::Start(2)).unwrap();
    let input = FileInput::ApplePair {
        still: FileRange {
            file: &file,
            offset: 6,
            length: still.len() as u64,
        },
        movie: FileRange {
            file: &file,
            offset: (9 + still.len()) as u64,
            length: movie.len() as u64,
        },
    };
    let ranged = MotionPhoto::open_file(input, ParseOptions::strict()).unwrap();
    assert_eq!(ranged.still_bytes().unwrap(), still);
    assert_eq!(ranged.motion_video_bytes().unwrap(), movie);
    assert!(ranged
        .presentation_time()
        .unwrap()
        .equivalent(source.presentation_time().unwrap()));
    assert_eq!(
        ranged.asset().pairing.identifier.as_deref(),
        Some("RANGED-PAIR")
    );
    assert_eq!((&file).stream_position().unwrap(), 2);
    assert!(matches!(
        MotionPhoto::open_file(
            input,
            ParseOptions {
                max_input_bytes: still.len() + movie.len() - 1,
                ..ParseOptions::strict()
            }
        ),
        Err(MotionPhotoReadError::InputLimit)
    ));
}
