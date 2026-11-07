use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "llp-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_llp"))
        .args(args)
        .output()
        .unwrap()
}
fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn demo(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/web/public/demo")
        .join(name)
}
#[test]
fn conformance_cli_is_opt_in_and_uses_explicit_profile_and_track_associations() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.mp4");
    let original = std::fs::read(demo("motion.mp4")).unwrap();
    std::fs::write(&source, &original).unwrap();
    let result = run(&[
        "conformance",
        s(&source),
        "--tracks",
        "1",
        "--options-json",
        r#"{"profile":"google-video-container-1.0"}"#,
    ]);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(value["metadata"]["checkedRules"].is_array(), "{value}");
    assert!(value["metadata"]["scope"]
        .as_str()
        .unwrap()
        .contains("no codec decoding"));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let result = run(&[
        "conformance",
        s(&source),
        "--options-json",
        r#"{"profile":"unknown"}"#,
    ]);
    assert_eq!(result.status.code(), Some(1));
}
#[test]
fn real_cli_build_check_extract_round_trip_and_refuse_overwrite() {
    let scratch = Scratch::new();
    let out = scratch.0.join("built");
    let still = demo("cover.jpg");
    let movie = demo("motion.mp4");
    let built = run(&[
        "build",
        s(&still),
        "--movie",
        s(&movie),
        "--time",
        "13/16",
        "--target",
        "google-jpeg",
        "--out",
        s(&out),
    ]);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let photo = out.join("assetMP.jpg");
    assert!(photo.exists());
    let checked = run(&["check", s(&photo)]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );
    let extraction = scratch.0.join("extracted");
    assert!(
        run(&["extract", s(&photo), "--video", "--out", s(&extraction)])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(extraction.join("resource-1.mp4")).unwrap(),
        std::fs::read(&movie).unwrap()
    );
    let unchanged = std::fs::read(out.join("report.json")).unwrap();
    assert_eq!(
        run(&["extract", s(&photo), "--out", s(&out)]).status.code(),
        Some(2)
    );
    assert_eq!(std::fs::read(out.join("report.json")).unwrap(), unchanged);
    let error = run(&[
        "build",
        s(&still),
        "--movie",
        s(&movie),
        "--time",
        "1/3",
        "--target",
        "google-jpeg",
        "--out",
        s(&scratch.0.join("rounded")),
    ]);
    assert_eq!(error.status.code(), Some(1));
    assert!(!scratch.0.join("rounded").exists());
}
#[test]
fn apple_pvt_build_round_trip_preserves_report_outside_the_package() {
    let scratch = Scratch::new();
    let source = demo("motion-photo.jpg");
    let package = scratch.0.join("motion.pvt");
    let built = run(&[
        "build",
        s(&source),
        "--target",
        "apple-pvt",
        "--pair-id",
        "motion-pvt-1",
        "--out",
        s(&package),
    ]);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&built.stdout).unwrap();
    assert_eq!(report["metadata"]["container"], "apple-pvt");
    assert_eq!(
        report["metadata"]["pvt"]["contentIdentifier"],
        "MOTION-PVT-1"
    );
    let still_name = report["metadata"]["pvt"]["still"].as_str().unwrap();
    let movie_name = report["metadata"]["pvt"]["movie"].as_str().unwrap();
    assert_eq!(still_name, "motion.jpg");
    assert_eq!(movie_name, "motion.mov");
    assert_eq!(
        std::fs::read_dir(&package)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<std::collections::BTreeSet<_>>(),
        [
            "metadata.plist".to_string(),
            still_name.into(),
            movie_name.into()
        ]
        .into_iter()
        .collect()
    );
    assert!(!package.join("report.json").exists());
    let metadata = std::fs::read(package.join("metadata.plist")).unwrap();
    let metadata = String::from_utf8(metadata).unwrap();
    assert!(metadata.contains("<key>PFVideoComplementMetadataVersionKey</key><string>1</string>"));
    let parsed = run(&["parse", s(&package)]);
    assert!(
        parsed.status.success(),
        "{}",
        String::from_utf8_lossy(&parsed.stdout)
    );
    let parsed: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    assert_eq!(parsed["metadata"]["pairIdentifier"], "MOTION-PVT-1");
    assert_eq!(parsed["pvt"]["metadata"], "metadata.plist");
    assert!(run(&["check", s(&package)]).status.success());
    let again = run(&[
        "build",
        s(&source),
        "--target",
        "apple-pvt",
        "--pair-id",
        "motion-pvt-1",
        "--out",
        s(&package),
    ]);
    assert_eq!(again.status.code(), Some(2));
    assert!(package.join("metadata.plist").is_file());
    let unknown = run(&[
        "build",
        s(&demo("cover.jpg")),
        "--movie",
        s(&demo("motion.mp4")),
        "--time",
        "13/16",
        "--target",
        "not-a-target",
        "--out",
        s(&scratch.0.join("unused")),
    ]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&unknown.stdout).contains("unknown target"));
    assert!(!scratch.0.join("unused").exists());
    let sidecars = scratch.0.join("sidecars.pvt");
    let rejected = run(&[
        "build",
        s(&source),
        "--target",
        "apple-pvt",
        "--pair-id",
        "motion-pvt-1",
        "--sidecars",
        "--out",
        s(&sidecars),
    ]);
    assert_eq!(rejected.status.code(), Some(2));
    assert!(!sidecars.exists());
}

#[test]
fn metadata_version_ranges_and_invalid_input_have_explicit_exit_results() {
    let version = run(&["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("LLP1"));
    let input = demo("motion-photo.jpg");
    let report = run(&["metadata", s(&input), "--disable-3p"]);
    assert!(report.status.success());
    let v: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(v["metadata"]["verifiedMotion"], true);
    let scratch = Scratch::new();
    let wrapped = scratch.0.join("wrapped.bin");
    let original = std::fs::read(input).unwrap();
    std::fs::write(
        &wrapped,
        [b"prefix-prefix".as_slice(), original.as_slice(), b"suffix"].concat(),
    )
    .unwrap();
    let range = format!("13:{}", original.len());
    let parsed = run(&["parse", s(&wrapped), "--range", &range]);
    assert!(parsed.status.success());
    let v: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    assert_eq!(v["metadata"]["photoTime"]["value"], "812500");
    assert_eq!(
        run(&["parse", s(&wrapped), "--range", "18446744073709551615:2"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(run(&["check", s(&wrapped)]).status.code(), Some(1));
}
