# Native SDK integration

The shared operation implementation backs CLI, C ABI, Java/JNI, Swift and Android
services. JSON requests use `liblivephoto-interface`; LLP1 responses own exact
JSON and original output bytes. No wrapper silently publishes files, uploads,
transcodes or creates Moments metadata. Integer times remain authoritative.

Optional `conformance` requests inspect a single immutable HEIF image or movie.
Use `profile:"heif-color-2025"`, or `profile:"google-video-container-1.0"` with
ordered `trackIds:[primary,secondary]` (secondary optional). CLI `--tracks` and
Java `new Request("conformance").option("profile",...).tracks(...)` use the same
schema. Reports list checked rules and Violation/Unsupported/Recommendation;
success covers required listed rules only. See [exact scope and examples](CORE_INTEGRATION.md).
Host C/JNI calls are tested; the Android service allowlists do not expose this
additional action. Full bitstream, rendering and native-album acceptance remain
separate from either conformance profile.

## Host archives

```sh
python3 scripts/package_native.py
python3 scripts/test_packaged_sdk.py dist/liblivephoto-<os>-<arch>-sdk.zip
```

Requires full JDK 21 and Rust. Archives include the release CLI, shared/static
libraries, C header, Java 8 jar, docs, source identity, file hashes and each locked
dependency's notices. CI tests extracted packages on Linux x86_64, macOS arm64
and Windows x86_64. The external Java consumer compiles against only the packaged
jar and loads the extracted release library. No release or registry is published.

For C, include `native/include/liblivephoto.h`. Rust owns response handles: copy
data before freeing, use the matching free once. Valid pointers are the caller's
responsibility; null/length/budgets are checked. Core/format/interface continue to
forbid unsafe code; pointer operations are isolated in the native boundary.

For Java, include the jar and set `-Dliblivephoto.native.path=/absolute/library`.
See [Java](java/README.md). Input bytes, paths and borrowed `FileChannel` ranges
share the operation schema. Positional reads preserve ownership/cursor. Cancel
stops queued/reading work and discards late results; synchronous Rust phases are
not internally interruptible.

## Apple static core

On Xcode with Rust targets `aarch64-apple-ios`, `aarch64-apple-ios-sim` and
`x86_64-apple-ios`:

```sh
python3 scripts/package_apple.py
python3 scripts/test_apple_simulator.py
```

The XCFramework contains iOS arm64 and arm64/x86_64 Simulator static libraries
plus headers/notices. Link the applicable archive into the host, retaining and
exporting C entry points, then use `RustCoreBridge(linkedCore: true)`. The supplied
Simulator acceptance uses `-force_load` and `-export_dynamic`; hosts with dead
stripping must likewise retain/export the `llp_*` symbols the bridge resolves.
The local dylib initializer is also available on supported native hosts. Hosts
sign their final apps. No library is downloaded by the adapter.

Simulator acceptance compiles actual Swift against the iOS SDK, links Rust,
creates its own compatible simulator and executes parse/extract/AGTM/error calls.
Only that newly created simulator is shut down/deleted. SDK build, Simulator
runtime and physical-device/Photos-library acceptance have separate receipts.
See [Apple](apple/README.md).

## Android and metadata

See [Android](android/README.md) for the AAR, four services, isolated metadata
process, FD/ranges, pipe output, budgets and installed-APK tests. No SDK license
is accepted automatically; generated test keys remain ignored and unuploaded.

`metadata` returns extents, bounded raw views, declarations and independent source
verification. `extract-metadata` copies full selected blocks (`blockIds`, CLI
`--blocks`) under a cumulative 128 MiB budget. Blocks may contain coded HEIF items
or proprietary video trailers; the response explicitly says so. This extends
Google's first-matching motion-block convenience. No pixels/depth/HDR semantics
are invented. `classify-metadata` always clears persisted source verification.
Absent optional XMP appears in `unavailable`. Invalid checks have `success=false`.

Use result/diagnostics rather than a signature or produced file as proof of
Gallery/Photos acceptance. Unknown reference-bearing MP4 layouts remain rejected.
Lossless remux preserves packets and exact time; opt-in compaction/drop is reported.

Round4 verified source **bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7** passed
all three workflows/eight jobs, including 42 new syntax/byte/time/error vectors
at every applicable native, mobile and actual browser entry. See the
[handoff](evidence/HANDOFF_ROUND4_20261019.md),
[terminal receipt](evidence/platform-bd6f85b.json) and
[remaining boundaries](evidence/REMAINING_WORK.md). Native Gallery/Photos and
full ICC/AAC/rendering/device compatibility are still unverified or partial.

The shared operation protocol now has a minimal WASM export generated into
`web/dist/operation-pkg` (browser) and `operation-node` (Node):
`process_operation(requestJSON, primaryBytes, movieBytes)` returns owned LLP1
bytes. It calls the same Rust implementation and does not publish files. The
existing inspection/UI module remains separate. Its42-case tests validate raw
ICC profiles, AAC declarations, compact tables, byte preservation and exact time.
Android service allowlists stay unchanged: conformance/remux tests use actual
in-app JNI, while existing parse/build/extract/AGTM tests use Binder/FD ranges.
A built arm64 Android/iOS library is not physical-device runtime acceptance.
