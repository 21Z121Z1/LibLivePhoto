# LibLivePhoto

[简体中文](README.zh-CN.md)

A Rust library and `llp` CLI to inspect, extract, build, and check Apple Live Photos, Android Motion Photos, and Samsung SEF files.

## Why

A Live Photo or Motion Photo combines a still image with a short video, but each ecosystem stores them differently. LibLivePhoto lets you see what's inside, extract original resources, and convert to supported formats without silently transcoding the media. C/JNI, Android AAR, Swift, and WASM bindings are available experimentally.

Rust 1.88+ (tested; not yet declared)

## Quick start

From the repository root:

```sh
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

To use the Rust library from an application alongside your `LibLivePhoto` checkout, add this to the application's `Cargo.toml`:

```toml
[dependencies]
liblivephoto = { path = "../LibLivePhoto/crates/liblivephoto" }
```

## Examples

These commands use the generated, CC0 [demo files](web/public/demo/provenance.json) and run from the repository root. Each `--out` path must be a directory that does not yet exist.

### Inspect a Motion Photo

```sh
./target/debug/llp parse web/public/demo/motion-photo.jpg
```

### Extract its video

```sh
./target/debug/llp extract web/public/demo/motion-photo.jpg --video --out llp-demo-extracted
```

The video is saved as `llp-demo-extracted/resource-1.mp4`.

### Build and check a Motion Photo JPEG

```sh
./target/debug/llp build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out llp-demo-built
./target/debug/llp check llp-demo-built/asset-0.jpg
```

`13/16` is the presentation time in the demo. See [CLI](CLI.md) for the other commands and options.

## Format support

This matrix summarizes implemented, fixture-scoped operations—not a guarantee for every device or file variant. See [Format support](FORMAT_SUPPORT.md) for the exact contracts.

| Format | Read / inspect | Extract | Build / convert | Photos / Gallery acceptance |
| --- | --- | --- | --- | --- |
| Android Motion Photo (JPEG) | Yes | Yes | `google-jpeg` | **Unverified** |
| Android Motion Photo (HEIF `mpvd`) | Yes | Yes | `google-heif` | **Unverified** |
| Android Motion Photo (AVIF `mpvd`) | Limited fixtures | Limited fixtures | `google-avif` | **Unverified** |
| Apple Live Photo (JPEG/HEIF + MOV) | Yes | Yes | `apple` pair | **Unverified** |
| Samsung SEF (JPEG, 106/107) | Yes | Yes; ambiguous records need selection | `samsung-jpeg-sef` if photo time is unknown | **Unverified** |
| Legacy Google MicroVideo | Synthetic fixtures | Synthetic fixtures | Convert to Android or preserve source | **Unverified** |
| Selected OPlus, Xiaomi, vivo, Honor extensions | Partial | Partial | Standard target or sidecars; no native writer | **Unverified** |

## Limitations

Preserved bytes, structural validity, and acceptance by a real device are different claims. Extraction can preserve the original resource bytes; `llp check` validates only supported structure. Neither proves that Apple Photos or a vendor Gallery will accept the result.

The CLI and shared interface cap combined input at **128 MiB**; Android services cap it at **32 MiB**. Parsing is not streaming, and fragmented movie layouts or unsupported top-level boxes such as `sidx` cannot be remuxed.

Writers cover source preservation, Android JPEG/HEIF/AVIF, Apple still+MOV pairs, and Samsung JPEG SEF (without a known photo time). There is no native OPlus, Xiaomi, vivo, or Honor writer, and no pixel transcoding. See [Compatibility](COMPATIBILITY.md) for conversion boundaries.

Bindings: [Java/JNI](java/README.md) · [Android](android/README.md) · [Swift](apple/README.md) · [WASM](web/README.md). Build and test provenance: [Verification](docs/VERIFICATION.md). Licensed under [Apache-2.0](LICENSE).
