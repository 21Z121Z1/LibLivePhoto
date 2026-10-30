# LibLivePhoto

[中文](README.zh-CN.md)

LibLivePhoto reads, checks, and builds Motion Photo and Live Photo files. The Rust library and the `llp` command are the supported way to use this tree. The C ABI, Java JNI client, Android AAR, Swift package, and WASM demo are experimental bindings over the same operations.

`Cargo.toml` sets the package version to 0.1.0. `llp version` reports that same version. `git tag` lists no tags. A request for the crate name `liblivephoto` on the crates.io API returned HTTP 404, so crates.io has no installable package under that name.

## Install the library and the CLI

`Cargo.toml` does not set `rust-version`. On this machine the locked build of `liblivephoto`, `liblivephoto-cli`, and `liblivephoto-native` failed with rustc 1.87.0 because `slice::as_chunks` was still unstable in `crates/liblivephoto-format/src/exif_raw.rs`. The same locked build succeeded with rustc 1.88.0 and with rustc 1.99.1. Those results are build logs, not a declared minimum supported Rust version.

From a clone, with a Rust toolchain that can build this lockfile, run the following commands.

```sh
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

`llp version` prints `"version": "0.1.0"` and `"success": true`.

A Rust crate in this workspace depends on the library by path.

```toml
liblivephoto = { path = "crates/liblivephoto", version = "0.1.0" }
```

## Other bindings

The Java command in this section was run here. The Android, Swift, and WASM commands were not. Their CI results are for commit `042a188efc2f9c10638a571c3fec57b5032d5870` on `main`.

`python3 scripts/test_java.py` builds `liblivephoto-native`, compiles the Java sources with `javac --release 8`, and runs `org.liblivephoto.NativeSmoke` on `web/public/demo`. With OpenJDK 21.0.12.1 and rustc 1.99.1, that script printed `"smoke": "passed"`.

`scripts/build_android.py` writes `android/build/liblivephoto-android.aar`. `MotionPhotoService.MAX_INPUT_BYTES` is 32 MiB, and `MetadataParserService` is an isolated process. This machine has no Android SDK, so the script was not run here. The [Android native services](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330060) job `sdk-and-services` succeeded on that `main` commit. The job builds the AAR and runs an owned API 35 x86_64 emulator. An emulator result leaves vendor Gallery acceptance unverified.

The Swift package is the `apple` directory. `Package.swift` declares Swift tools 5.9, macOS 13, iOS 16, and the library name `LibLivePhotoApple`. Swift is not installed here. The [Apple adapter and local web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330012) job `apple` succeeded on that same commit. The job runs `swift test --package-path apple` and an iOS Simulator acceptance. A simulator result leaves Photos acceptance unverified.

The WASM demo lives in `web`. The `web` job in that same workflow succeeded. It uses the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.104, and Node 24, then runs the Node tests and the Chrome browser tests. Node on this machine is v22.23.4, and those web commands were not run here.

## Read a Motion Photo

The files in `web/public/demo` are generated geometry. `provenance.json` marks them CC0 and records that they contain no private camera media. They are the inputs for the commands below.

```sh
cargo run --locked -p liblivephoto --example inspect -- web/public/demo/motion-photo.jpg
```

With rustc 1.99.1 the example prints `format=AndroidMotionPhoto`, `dialect=AndroidStandard`, four resources, one track, and `MediaTime { value: 812500, timescale: 1000000 }`. The primary video resource is 19670 bytes.

## Extract the video bytes

`llp` creates the output directory with `create_dir` and each output file with `create_new`. An existing output path fails. Remove a previous directory before you run the command again.

```sh
cargo run --locked -p liblivephoto-cli -- extract web/public/demo/motion-photo.jpg --video --out /tmp/llp-extract
cmp web/public/demo/motion.mp4 /tmp/llp-extract/resource-1.mp4 && echo match
```

The extract command exits 0. `resource-1.mp4` is 19670 bytes and matches `web/public/demo/motion.mp4` byte for byte. For this sample, the extracted video bytes are the original video bytes.

## Build a Motion Photo JPEG

```sh
cargo run --locked -p liblivephoto-cli -- build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out /tmp/llp-built
cargo run --locked -p liblivephoto-cli -- check /tmp/llp-built/asset-0.jpg
```

The build report has `"canExecute": true`, `"target": "AndroidJpeg"`, and `"consumerAcceptance": "Unverified"`. The video disposition is `Preserved`. The still disposition is `Rewritten`, because the writer replaces container metadata. `llp check` then reports `"valid": true` for `ResourceRanges`, `StillContainer`, `MotionContainer`, and `Tracks`.

`13/16` is the presentation time stored for these demo files in `web/public/demo/provenance.json`.

## Support matrix

A vendor name does not mean a device accepts the file. The longer contract table is [Format support](FORMAT_SUPPORT.md). The cells below follow that table.

| Contract | Bytes | Structure | Photos or Gallery |
| --- | --- | --- | --- |
| Android JPEG directory | Round-trip comparisons on real fixtures | Tests, including malformed files | Unverified |
| Android HEIF `mpvd` | Append-only metadata, with item and time comparisons | Tests, including malformed files | Unverified |
| Android AVIF `mpvd` | Append-only metadata on synthetic contracts and one generated file | Reference reparse and an independent still decode | Unverified |
| Apple JPEG or HEIF, plus a MOV | Pairing metadata, an exact timeline, and media comparisons. The fixture set includes one original iPhone 15 pair | Pairing and supported timed metadata | Unverified |
| Legacy Google Micro Video | Android conversion and a source copy, on synthetic tests | Bounds and directory checks | Unverified |
| ColorOS 15 and 16 extensions | Source copy and explicit sidecars. Original fixtures include two streams | LPEX, topology, and boundary tests | Unverified |
| OpCamera and JXRS layouts | Source copy, or a standard target with an explicit policy for unmapped resources. The cases are synthetic | Framing and typed fields. Checksum bytes stay in the file and are not treated as a verified checksum | Unverified |
| Samsung SEF 106 and 107 | A direct JPEG SEF writer, and only when presentation time is unknown | Directory tests, including malformed files | Unverified |
| Xiaomi Android-style Live Photo | One original fixture, written onward as an Android target or as explicit sidecars | Standard contract checks | Unverified |
| vivo single file and CameraAlbum 43 | Two original fixtures. Android target or explicit sidecars. No native footer writer | Footer ranges and declared-ID checks | Unverified |
| CameraAlbum 55 | No native writer. Synthetic contract only | Framing checks and malformed declarations | Unverified |
| vivo media UUID streams | Original movie bytes kept. No native UUID writer | Extent, size, and budget checks | Unverified |
| Xiaomi portrait or depth, and vivo dual or portrait | No original corpus and no dedicated writer | Unknown bytes can be kept | Unverified |
| HarmonyOS Moving Photo | No private on-disk contract in this library | No on-disk contract is claimed | Unverified |
| Honor `LIVE_` footer and backward TLV | Source copy or explicit sidecars, on synthetic contracts. No Honor writer | Bounded records and malformed cases | Unverified |

`TargetProfile` in `crates/liblivephoto/src/conversion.rs` lists `PreserveSource`, `AndroidJpeg`, `AndroidHeif`, `AndroidAvif`, `SamsungJpegSef`, and `ApplePair`. The CLI target names mapped in `liblivephoto-interface` are `preserve`, `google-jpeg`, `google-heif`, `google-avif`, `apple`, and `samsung-jpeg-sef`.

## Limits

Three outcomes are different facts.

Preserved bytes means an extract, or a `Preserved` disposition, keeps the original resource bytes. The extract example above does that for the demo video.

A structural pass means `llp check` reports `"valid": true`, or a build plan has no blockers and the output parses. The build example above does that. The same build report sets `"consumerAcceptance": "Unverified"`.

Photos and Gallery acceptance is unverified for every row in [Format support](FORMAT_SUPPORT.md). The CI jobs linked above run an emulator and a simulator.

Shared operations reject combined input above 128 MiB. `MAX_INPUT` in `crates/liblivephoto-interface/src/lib.rs` is `128 * 1024 * 1024`. Android services reject aggregate input above 32 MiB.

Remux accepts only the top-level boxes `ftyp`, `moov`, `mdat`, `free`, `skip`, and `wide`. Any other top-level box, including `sidx`, returns `UnsupportedLayout` from `crates/liblivephoto/src/remux/compose.rs`. Fragmented metadata samples and fragmented frame timing are unsupported layouts in `crates/liblivephoto/src/tracks.rs`. This pass did not submit a fragmented file to `llp`.

`TargetProfile` has no OPlus, Xiaomi, vivo, or Honor writer. The Samsung JPEG SEF writer blocks a known presentation time. This command exits 1.

```sh
cargo run --locked -p liblivephoto --example convert -- samsung-jpeg-sef /tmp/llp-samsung web/public/demo/motion-photo.jpg
```

The error text is `SEF target has no supported exact presentation-time mapping; use Android or PreserveSource`.

The library has no Moments construction API. Writers do not transcode pixels.

## Further reading

Command details are in [CLI](CLI.md). Conversion limits are in [Compatibility](COMPATIBILITY.md). Packaging is in [Native SDK](NATIVE_SDK.md). Binding notes are in [Java](java/README.md), [Android](android/README.md), [Apple](apple/README.md), and the [WASM demo](web/README.md).

Vendor ledgers, handoffs, and open questions are indexed in [Research notes](docs/research.md).
