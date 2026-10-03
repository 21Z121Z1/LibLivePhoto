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
            ("movie", common::movie(1000, 2)),
            ("overflow-box", vec![255; 32]),
            ("truncated-xml", common::jpeg("<x:xmpmeta><rdf:RDF>")),
            ("empty", vec![]),
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
