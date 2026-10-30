# LibLivePhoto

[中文](README.zh-CN.md)

Read, check, and build Motion Photo and Live Photo files with one Rust library and the `llp` command.

Phone cameras store a still and a short video in vendor-specific containers. LibLivePhoto parses those files, returns the original bytes when the format allows it, and writes a few explicit targets. The C ABI, Java JNI client, Android AAR, Swift package, and WASM demo are experimental bindings over the same operations.

Rust 1.88+ (tested; not yet declared).

## Quick start

From a clone of this repository:

```sh
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

A crate in this workspace depends on the library by path:

```toml
liblivephoto = { path = "crates/liblivephoto", version = "0.1.0" }
```

## Examples

The files in `web/public/demo` are generated geometry. `provenance.json` marks them CC0 and records that they contain no private camera media.

`llp` creates the output directory and every file inside it. An existing directory makes the command fail, so remove the previous one before running it again.

### Inspect a Motion Photo

```sh
cargo run --locked -p liblivephoto --example inspect -- web/public/demo/motion-photo.jpg
```

### Extract the video

```sh
cargo run --locked -p liblivephoto-cli -- extract web/public/demo/motion-photo.jpg --video --out /tmp/llp-extract
```

### Build a Motion Photo JPEG

`13/16` is the photo time stored for these demo files.

```sh
cargo run --locked -p liblivephoto-cli -- build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out /tmp/llp-built
cargo run --locked -p liblivephoto-cli -- check /tmp/llp-built/asset-0.jpg
```

CLI target names are `preserve`, `google-jpeg`, `google-heif`, `google-avif`, `apple`, and `samsung-jpeg-sef`. Flag details are in [CLI](CLI.md).

## Support

Device acceptance is separate from the vendor name. The full contract table is [Format support](FORMAT_SUPPORT.md).

| Contract | In this tree | Photos or Gallery |
| --- | --- | --- |
| Android JPEG directory | Round-trip on real fixtures, including malformed files | Unverified |
| Android HEIF `mpvd` | Append-only metadata, with item and time checks | Unverified |
| Android AVIF `mpvd` | Append-only metadata on synthetic contracts and one generated file | Unverified |
| Apple JPEG or HEIF, plus a MOV | Pairing, timeline, and media checks, including one iPhone 15 pair | Unverified |
| Legacy Google Micro Video | Android conversion and a source copy, on synthetic tests | Unverified |
| ColorOS 15 and 16 | Source copy and explicit sidecars; originals include two streams | Unverified |
| OpCamera and JXRS | Source copy, or a standard target when unmapped resources have an explicit policy | Unverified |
| Samsung SEF 106 and 107 | Direct JPEG SEF writer, only when the photo time is unknown | Unverified |
| Xiaomi Android-style Live Photo | One original fixture, written as an Android target or as sidecars | Unverified |
| vivo single file and CameraAlbum 43 | Two original fixtures; Android target or sidecars; no native footer writer | Unverified |
| CameraAlbum 55 | Synthetic contract only; no native writer | Unverified |
| vivo media UUID streams | Original movie bytes kept; no native UUID writer | Unverified |
| Xiaomi portrait or depth; vivo dual or portrait | No original corpus and no dedicated writer | Unverified |
| HarmonyOS Moving Photo | No private on-disk contract in this library | Unverified |
| Honor `LIVE_` footer and backward TLV | Source copy or sidecars on synthetic contracts; no Honor writer | Unverified |

## Limitations

Original bytes, a structural check, and acceptance by Photos or Gallery are three different results. Extraction can return the original resource bytes. `llp check`, and a build plan with no blockers, speak only to structure. Photos and Gallery stay unverified on every row above.

Shared operations reject combined input above 128 MiB. Android services reject aggregate input above 32 MiB.

Remux accepts only the top-level boxes `ftyp`, `moov`, `mdat`, `free`, `skip`, and `wide`. Any other top-level box, including `sidx`, is rejected. Fragmented metadata samples and fragmented frame timing are unsupported.

`TargetProfile` writes `PreserveSource`, Android JPEG, HEIF, and AVIF, Samsung JPEG SEF, and an Apple pair. It has no OPlus, Xiaomi, vivo, or Honor writer. The Samsung JPEG SEF writer refuses a known photo time. There is no Moments construction API, and writers do not transcode pixels.

Binding notes are in [Java](java/README.md), [Android](android/README.md), [Apple](apple/README.md), and the [WASM demo](web/README.md). Command output and CI records are in [Verification](docs/VERIFICATION.md).

## Further reading

Conversion limits are in [Compatibility](COMPATIBILITY.md). Packaging is in [Native SDK](NATIVE_SDK.md). Vendor ledgers and open questions are indexed in [Research notes](docs/research.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
