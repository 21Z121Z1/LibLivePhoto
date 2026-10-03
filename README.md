# LibLivePhoto

LibLivePhoto is a portable Rust library for related still, video and auxiliary
resources. It parses bytes, exposes resource and track facts, extracts resources,
validates supported structures, and plans explicit format conversions.

```rust
use liblivephoto::{Input, MotionPhoto, ParseOptions};

fn extract_motion(input: &[u8]) -> Result<Vec<u8>, liblivephoto::MotionPhotoReadError> {
    let photo = MotionPhoto::parse(Input::SingleFile(input), ParseOptions::compatible())?;
    let mut motion = Vec::new();
    photo.extract(photo.motion_video(), &mut motion)?;
    Ok(motion)
}
```

Use `Input::ApplePair` for an original Apple pair. Use `Input::Parts` for an
association supplied by the caller. Filenames do not establish pairing.

`photo.plan(target, policy)` reports resources that the target cannot express.
The default policy rejects unmapped resources. `PreserveSidecars` retains their
exact bytes outside the target graph. `DropExplicitly` reports each permitted
drop. `execute()` returns bytes and a report; it does not publish files.

The default library has no XDRemux, Apple framework, codec or image processing
dependency. `liblivephoto-format` provides the small shared binary-format boundary.
The optional `xdremux-compat` module is an unstable migration interface.

Read [format support](FORMAT_SUPPORT.md), [compatibility](COMPATIBILITY.md),
[asset model](ASSET_MODEL.md), [architecture](ARCHITECTURE.md) and
[vendor evidence](evidence/VENDORS.md) before making a compatibility claim.

Validation:

```sh
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/external_consumer.py
python3 scripts/fixtures.py --fetch
cargo test -p liblivephoto --test fixtures -- --ignored
bash scripts/fuzz.sh 60
```

The last command requires nightly Rust and `cargo-fuzz`. It uses AddressSanitizer
and libFuzzer coverage. A bounded campaign does not prove absence of parser defects.
External analysis samples and proprietary APKs remain outside Git.
