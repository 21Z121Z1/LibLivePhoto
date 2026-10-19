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
        .join("../../web/public/demo")
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
    let photo = out.join("asset-0.jpg");
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
