# Thin Apple adapter

Swift Package for macOS 13+ / iOS 16+, using public PhotoKit/PhotosUI/AVFoundation.
Run `swift test --package-path apple` on macOS. Compile iOS in Xcode or run the
simulator-SDK build from `.github/workflows/apple-web.yml`. Linux cannot validate
Apple SDK branches; the development container has no Swift/Xcode installation.

The host validates files with the Rust core, retains the original manifest bytes
and files, and may decode manifest v1 through `InspectionManifest.decode`.
The fixture is actual output from `liblivephoto-bridge` on the included synthetic
Apple pair, not hand-authored metadata. This package does not yet provide a
native Rust C/Swift binary bridge; host integration uses the versioned manifest
and local resource URLs. WASM is the implemented direct Rust binding for the web.

Use `PhotoKitAdapter.load` for local paired resources, `resources(for:)` to retain
the complete native resource inventory, `export` for an explicitly selected
resource, and `importPair` to submit `.photo` + `.pairedVideo` with move disabled.
The caller obtains the appropriate Photo Library permission. No API is invoked
automatically by package import. Compilation/invalid-input tests do not exercise
user libraries or prove valid-pair import acceptance.

NativePlayback wraps the iOS system view, state, mute and system playback gesture.
NativeEditingSession forwards the system frame processor/audio volume and owns
preview/output cancellation. Serialize operations on each editing context; retain
sessions/wrappers for their operation lifetime. NativeOperation callbacks may run
on native queues: hop to the UI actor as needed. Cancellation is terminal before
a commit, and returns false for non-cancellable commits. Export is byte-budgeted
and refuses to overwrite an existing destination; I/O failure may leave a partial
new output. Import copies original files and does not move them.

CaptureContract is only host-session/settings validation, not a camera system.
Capture and private Photos effects are explicitly unavailable in portable APIs.
See the [public API mapping](../APPLE_API_ARCHITECTURE.md) for limits and evidence.
