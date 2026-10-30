# LibLivePhoto

[简体中文](README.zh-CN.md)

**LibLivePhoto** is a Rust library and `llp` CLI for working with Apple Live Photos, Android Motion Photos, and Samsung SEF files.

These formats all combine a still image with a short video, but store them differently. LibLivePhoto can inspect their contents, extract the original media, and build supported output formats. The C/JNI, Android AAR, Swift, and WASM bindings are experimental.

Rust 1.88+ (tested; not yet declared)

## Quick start

Clone the repository and build the CLI:

```sh
git clone https://github.com/21Z121Z1/LibLivePhoto.git
cd LibLivePhoto
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

To use the library in a Rust project next to the `LibLivePhoto` directory, add this to your project's `Cargo.toml`:

```toml
[dependencies]
liblivephoto = { path = "../LibLivePhoto/crates/liblivephoto" }
```

## Examples

Run these commands from the repository root. The [demo files](web/public/demo/provenance.json) contain generated media released under CC0.

### Inspect a file

```sh
./target/debug/llp parse web/public/demo/motion-photo.jpg
```

The report shows the format, resources, and photo time.

### Extract the video

```sh
./target/debug/llp extract web/public/demo/motion-photo.jpg --video --out llp-demo-extracted
```

This creates `llp-demo-extracted/resource-1.mp4`. The output directory must not already exist.

### Build and check a Motion Photo

```sh
./target/debug/llp build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out llp-demo-built
./target/debug/llp check llp-demo-built/asset-0.jpg
```

`13/16` is the photo time in the demo. The check reads the generated `asset-0.jpg`.

## Format support

The table covers implemented operations and tested file layouts. See [Format support](FORMAT_SUPPORT.md) for the details.

| Format | Read | Extract | Build / convert |
| --- | --- | --- | --- |
| Android Motion Photo JPEG | Yes | Yes | `google-jpeg` |
| Android HEIF (`mpvd`) | Yes | Yes | `google-heif` |
| Android AVIF (`mpvd`) | Limited samples | Limited samples | `google-avif` |
| Apple Live Photo (JPEG/HEIF + MOV) | Yes | Yes | `apple` pair |
| Samsung SEF JPEG (106/107) | Yes | Yes¹ | `samsung-jpeg-sef`² |
| Legacy Google MicroVideo | Synthetic samples | Synthetic samples | Android format or source copy |
| Selected OPlus, Xiaomi, vivo, and Honor extensions | Partial | Partial | Standard format or sidecars; no native writer |

¹ Ambiguous Samsung records need explicit selection. ² Samsung writing requires an unknown photo time.

**Apple Photos and vendor Gallery acceptance: unverified for every format above.**

## Limitations

Extraction can preserve the original media bytes. `llp check` verifies supported file structures. Neither result proves that a device will accept the file.

The CLI and shared interface accept up to **128 MiB** of combined input; Android services accept up to **32 MiB**. Parsing is not streaming. Remuxing does not support fragmented movies or top-level boxes such as `sidx`.

Writers support source preservation, Android JPEG/HEIF/AVIF, Apple still+MOV pairs, and Samsung JPEG SEF. There is no native OPlus, Xiaomi, vivo, or Honor writer. Writers do not transcode media.

More details: [CLI](CLI.md) · [Compatibility](COMPATIBILITY.md) · [Native SDK](NATIVE_SDK.md) · [Java/JNI](java/README.md) · [Android](android/README.md) · [Swift](apple/README.md) · [WASM](web/README.md).

Build and test records are in [Verification](docs/VERIFICATION.md). License: [Apache-2.0](LICENSE).
