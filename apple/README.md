# Thin Apple adapter

Swift Package for macOS 13+ / iOS 16+, using public PhotoKit/PhotosUI/AVFoundation.
Run `swift test --package-path apple` on macOS. Compile iOS in Xcode or run the
simulator-SDK build from `.github/workflows/apple-web.yml`. Linux cannot validate
Apple SDK branches; the development container has no Swift/Xcode installation.

The host validates files with the Rust core, retains the original manifest bytes
and files, and may decode manifest v1 through `InspectionManifest.decode`.
The fixture is actual output from `liblivephoto-bridge` on the included synthetic
Apple pair, not hand-authored metadata. The package also has the real native RustCoreBridge described below; host integration retains the versioned manifest and local resource URLs. WASM remains the direct Rust binding for the web.

Use `PhotoKitAdapter.load` for local paired resources, `resources(for:)` to retain
the complete native resource inventory, `export` for an explicitly selected
resource, and `importPair` to submit `.photo` + `.pairedVideo` with move disabled.
The caller obtains the appropriate Photo Library permission. No API is invoked
automatically by package import. Compilation/invalid-input tests do not exercise
user libraries or prove valid-pair import acceptance.

NativePlayback wraps iOS and macOS system views, state and mute. The public system playback gesture accessor exists on iOS; macOS reports that accessor unavailable. Its native object test does not establish physical playback/HDR acceptance.
NativeEditingSession forwards the system frame processor/audio volume and owns
preview/output cancellation. Operations on each editing context are serialized by the adapter; retain
sessions/wrappers for their operation lifetime. NativeOperation callbacks may run
on native queues: hop to the UI actor as needed. Cancellation is terminal before
a commit, and returns false for non-cancellable commits. Export is byte-budgeted
and refuses to overwrite an existing destination; I/O failure may leave a partial
new output. Import copies original files and does not move them.

CaptureContract is only host-session/settings validation, not a camera system.
Capture and private Photos effects are explicitly unavailable in portable APIs.
See the [public API mapping](../APPLE_API_ARCHITECTURE.md) for limits and evidence.

`RustCoreBridge` now invokes the real isolated C ABI from a caller-supplied local
native library. It copies the LLP1 response before freeing the Rust-owned handle,
retains authoritative JSON bytes and decodes the exact inspection contract. Its
async wrapper suppresses cancelled/stale completions without claiming synchronous
native work was interrupted. Build `cargo build -p liblivephoto-native` and set
`LIBLIVEPHOTO_NATIVE_LIBRARY` to the dylib for actual macOS bridge tests. iOS hosts
must embed a correctly built/signed native library in their app; simulator-SDK
compilation alone does not validate device linking or runtime policy. No library
code is downloaded or loaded from a remote URL.

The native test suite now attempts valid project-generated local-pair loading and
resource export byte comparison, independent of Photo Library import permission.
Only exact CI/test receipts establish whether these gates passed. Import/editing
and physical playback/HDR acceptance still require an appropriate permitted asset
and Apple device; these tests do not bypass privacy permission.

`PhotoKitAdapter.editingInput` requests an already authorized asset, with explicit
network/progress/cancellation policy and adjustment-data handling. Create output
with real host adjustment data, render via `NativeEditingSession.save`, then call
`PhotoKitAdapter.commitEdit` for the same asset. Import/commit require prior host
permission and do not request permission automatically. Audio volume must be
finite and in [0,1]. Actual permitted library/render acceptance is a separate gate.

For the static XCFramework use `RustCoreBridge(linkedCore: true)` with retained,
exported C symbols. See [native SDK integration](../NATIVE_SDK.md) for packaging,
link flags and the actual iOS Simulator Swift/Rust acceptance command.

The runtime harness creates and deletes only its own simulator UUID. It records
boot progress, owned-device state, scoped CoreSimulator logs and failed receipts.
After two iPhone17e/iOS26.5 monitor timeouts, it prefers a compatible SE device
from the installed SDK, or accepts `--device-type IDENTIFIER` only if compatible.
If `bootstatus` times out but that owned device is `Booted`, the harness may try
the unchanged actual Swift/Rust process with its 120-second deadline. All original
native assertions and runtime identity output remain required; failed assertions
are never retried as success. This establishes C ABI execution, not SpringBoard,
Photos-library or physical-device UI readiness. Cleanup failures also fail the
acceptance. `python3 scripts/test_apple_harness.py` exercises fault injection
only; its seven tests are not Simulator acceptance.

The harness selects an already installed iOS runtime at or above the iOS16
deployment target, preferring the lowest available version; `--runtime IDENTIFIER`
can request another available compatible runtime. It validates the queried
iPhoneSimulator SDK, isolates inherited SDKROOT in child processes, and records
all available runtimes/selected SDK. XCFramework builds select the correct SDK
per target with an explicit iOS16 deployment target. This does not alter saved
environment settings, install runtimes, disable migration or grant permissions.
