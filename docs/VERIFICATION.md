# Verification

[中文](VERIFICATION.zh-CN.md)

These notes used to live on the English and Chinese landing pages. They record one documentation pass. They are not a support promise, and this readability rewrite did not run the commands again.

The landing page that held them is commit `af75e81` (`docs(readme): add a bilingual landing page for the preview`), merged to `main` as `8842459`. CI results below were cited for `main` commit `042a188efc2f9c10638a571c3fec57b5032d5870`.

## Package identity

`Cargo.toml` sets the package version to 0.1.0. After `cargo build --locked -p liblivephoto-cli`, `./target/debug/llp version` printed `"version": "0.1.0"` and `"success": true`. `git tag` listed no tags. A request for the crate name `liblivephoto` on the crates.io API returned HTTP 404, so that pass found no installable package under that name.

## Toolchain

`Cargo.toml` does not set `rust-version`. On that machine the locked build of `liblivephoto`, `liblivephoto-cli`, and `liblivephoto-native` failed with rustc 1.87.0 because `slice::as_chunks` was still unstable. The call site is `crates/liblivephoto-format/src/exif_raw.rs`. The same locked build succeeded with rustc 1.88.0 and with rustc 1.99.1. The command notes below used rustc 1.99.1 unless a line says otherwise. That is a build log, not a declared minimum.

## Commands run on the demo files

Inputs were `web/public/demo`. `provenance.json` marks those files CC0 and records that they contain no private camera media. `13/16` is the photo time stored there.

`cargo run --locked -p liblivephoto --example inspect -- web/public/demo/motion-photo.jpg` printed `format=AndroidMotionPhoto`, `dialect=AndroidStandard`, four resources, one track, and `MediaTime { value: 812500, timescale: 1000000 }`. The primary video resource was 19670 bytes.

```sh
cargo run --locked -p liblivephoto-cli -- extract web/public/demo/motion-photo.jpg --video --out /tmp/llp-extract
cmp web/public/demo/motion.mp4 /tmp/llp-extract/resource-1.mp4 && echo match
```

Extract exited 0. `resource-1.mp4` was 19670 bytes and matched `web/public/demo/motion.mp4`. For that sample, the extracted video bytes were the original video bytes.

```sh
cargo run --locked -p liblivephoto-cli -- build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out /tmp/llp-built
cargo run --locked -p liblivephoto-cli -- check /tmp/llp-built/asset-0.jpg
```

The build report had `"canExecute": true`, `"target": "AndroidJpeg"`, and `"consumerAcceptance": "Unverified"`. The video disposition was `Preserved`. The still disposition was `Rewritten`, because the writer replaces container metadata. `llp check` then reported `"valid": true` for `ResourceRanges`, `StillContainer`, `MotionContainer`, and `Tracks`.

The Samsung SEF convert example exited 1:

```sh
cargo run --locked -p liblivephoto --example convert -- samsung-jpeg-sef /tmp/llp-samsung web/public/demo/motion-photo.jpg
```

The error text was `SEF target has no supported exact presentation-time mapping; use Android or PreserveSource`.

That pass did not submit a fragmented file to `llp`.

## Java

`python3 scripts/test_java.py` builds `liblivephoto-native`, compiles the Java sources with `javac --release 8`, and runs `org.liblivephoto.NativeSmoke` on `web/public/demo`. With OpenJDK 21.0.12.1 and rustc 1.99.1, the script printed `"smoke": "passed"`.

## Android, Swift, and WASM

Those commands were not run on that machine. The CI results are for commit `042a188efc2f9c10638a571c3fec57b5032d5870` on `main`.

`scripts/build_android.py` writes `android/build/liblivephoto-android.aar`. `MotionPhotoService.MAX_INPUT_BYTES` is 32 MiB, and `MetadataParserService` is an isolated process. The machine had no Android SDK, so the script was not run there. The [Android native services](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330060) job `sdk-and-services` succeeded on that commit. The job builds the AAR and runs an owned API 35 x86_64 emulator. An emulator result leaves vendor Gallery acceptance unverified.

The Swift package is the `apple` directory. `Package.swift` declares Swift tools 5.9, macOS 13, iOS 16, and the library name `LibLivePhotoApple`. Swift was not installed on that machine. The [Apple adapter and local web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330012) job `apple` succeeded on the same commit. The job runs `swift test --package-path apple` and an iOS Simulator acceptance. A simulator result leaves Photos acceptance unverified.

The WASM demo lives in `web`. The `web` job in that same workflow succeeded. It uses the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.104, and Node 24, then runs the Node tests and the Chrome browser tests. Node on that machine was v22.23.4, and those web commands were not run there.

## Regenerate a receipt

Run this from the repository root and pass a new directory. `evidence/` is gitignored, so the receipt stays out of the tree. CI uploads that directory as an artifact.

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
```

The script builds `llp`, checks the checked-out head, runs the library, interface, and CLI tests, then `replay_extraction_budget.py` and `replay_historical_budgets.py`. Differential scripts, `scripts/test_syntax_contracts.py`, and `scripts/fuzz.sh` also write under `evidence/local`. They do not update tracked files.

Older research notes and SHA-named receipts are on branch [`archive/evidence-20261030`](https://github.com/21Z121Z1/LibLivePhoto/tree/archive/evidence-20261030/evidence) at `2fb54887aba91c0fbb607120038b93afc7dca8f0`. The index is [Research notes](research.md).
