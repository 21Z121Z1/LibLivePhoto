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

A runnable public-API converter is available without XDRemux:

```sh
cargo run -p liblivephoto --example convert -- apple new-output source.jpg --pair-id CALLER-UNIQUE-ID --sidecars
```

Use an identifier unique to the output asset. See the cross-format execution
contracts in [compatibility](COMPATIBILITY.md) for all profiles, preservation
policies and exact-time limits.

Read [format support](FORMAT_SUPPORT.md), [compatibility](COMPATIBILITY.md),
[asset model](ASSET_MODEL.md), [architecture](ARCHITECTURE.md) and
[vendor evidence](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/VENDORS.md) before making a compatibility claim.
The [current five-vendor research matrix](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/VENDOR_RESEARCH_20261019.md)
separates original-byte tests, static inference and unverified native behavior.
The first Google core comparison is recorded in the [coverage matrix](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/GOOGLE_CORE_COVERAGE.md).
The [run receipt](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/GOOGLE_CORE_RUN.md) records the tested SHA, commands and results.
The [complete interface inventory](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/GOOGLE_COMPLETE_INVENTORY.md) and
[acceptance checklist](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/ACCEPTANCE_CHECKLIST.md) track subsequent work
and remaining native/device gates. See [native SDK](NATIVE_SDK.md), [CLI](CLI.md),
[Java/JNI](java/README.md), [Android](android/README.md), [Apple](apple/README.md)
and the [local browser demo](web/README.md).
The [pinned source/test map](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/SOURCE_TEST_MAP.md) and
[stability gates](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/STABILITY_GATES.md) make operation-specific coverage,
cumulative resource limits and native acceptance boundaries explicit.
The [review handoff](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_20261019.md) links exact code/CI evidence
and concrete remaining boundaries.
See [core integration](CORE_INTEGRATION.md) for file ranges, AVIF, timed AGTM,
display/color facts, explicit codec contracts and metadata-track removal.

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

Round4 verified source **bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7** passed
all three workflows/eight jobs, including 42 new syntax/byte/time/error vectors
at every applicable native, mobile and actual browser entry. See the
[handoff](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_ROUND4_20261019.md),
[terminal receipt](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/platform-bd6f85b.json) and
[remaining boundaries](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/REMAINING_WORK.md). Native Gallery/Photos and
full ICC/AAC/rendering/device compatibility are still unverified or partial.
