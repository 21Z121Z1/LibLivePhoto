# Apple public API architecture and local demonstration

The existing Rust **Core / Formats / Remux** remains the authority for byte
ranges, pairing, exact integer time, orientation/color facts, validation and
lossless composition. No Apple parser has been rewritten in Swift or JavaScript.
The new inspection bridge is a versioned **adapter API**, not a stability promise
for every manifest field. The original Rust API and consumer tests remain intact.

```mermaid
flowchart LR
  F[Original files / paired resources] --> C[Core · Formats · Remux]
  C --> B[Owned inspection bridge · manifest v1]
  B --> W[WASM in dedicated worker]
  W --> D[Local browser demo]
  B --> S[Swift manifest contract]
  S --> N[Thin PhotoKit / PhotosUI adapter]
  N --> P[Apple system frameworks]
```

## Public API mapping

Official source links name the relevant public contracts. These are distinct
capabilities: local file parsing, native framework acceptance, editing output,
and a Photos app experience are not interchangeable evidence.

| Apple public contract | Responsibility / implementation | Status and validation boundary |
| --- | --- | --- |
| [PHLivePhoto](https://developer.apple.com/documentation/photos/phlivephoto), `request(withResourceFileURLs:placeholderImage:targetSize:contentMode:resultHandler:)`, cancellation and degraded/error keys | `PhotoKitAdapter.load` takes the still and paired movie URLs, retains native request cancellation, ignores degraded intermediate results, completes once. Rust `Input::ApplePair` validates actual metadata identity; `Input::Parts` is an explicitly caller-declared association. | Implemented. At 83018c4/9fa6208, 10 actual Swift tests include valid local-pair PhotoKit load and byte-exact photo/pairedVideo export; see exact CI receipts. A decoded manifest does not authenticate URLs or make them accepted by PhotoKit. |
| [PHAsset](https://developer.apple.com/documentation/photos/phasset), [PHAssetResource](https://developer.apple.com/documentation/photos/phassetresource), [PHAssetResourceManager](https://developer.apple.com/documentation/photos/phassetresourcemanager) | Inventory resources for an asset/live photo; export a caller-selected photo, paired video, adjustment or other resource with native progress, cancellation and a byte budget. Network download is opt-in. No filename-based pairing or arbitrary first-video selection. | Implemented adapter. Export of PHLivePhoto resources loaded from local files passed without a Photos-library mutation. Export of a library PHAsset requires its appropriate access. No permission is requested automatically by tests. I/O errors can leave a partial new destination; existing files and original media are not overwritten. |
| [PHAssetCreationRequest](https://developer.apple.com/documentation/photos/phassetcreationrequest), `.photo` / [`.pairedVideo`](https://developer.apple.com/documentation/photos/phassetresourcetype/pairedvideo) | `importPair` adds both resources in one native changes transaction with `shouldMoveFile=false`. Returns the created local identifier. | Implemented adapter. Host must obtain add permission. Native commit is explicitly non-cancellable after submission. Real valid-pair import and Gallery/Photos acceptance are **unverified**; compilation and web playback are not substitutes. |
| [PHLivePhotoView](https://developer.apple.com/documentation/photosui/phlivephotoview), playback style/delegate and playback gesture | iOS/macOS `NativePlayback` binds a live photo to the real view, full/hint playback, mute, start/end state and the system gesture recognizer. | Implemented iOS adapter, simulator-SDK compilation gate. The caller keeps the wrapper alive. macOS has no public playbackGestureRecognizer accessor; that capability is explicitly unavailable there. Its actual view object/lifecycle test passed at d5729c7; that checkpoint has 11 Swift tests. No device playback/gesture UX acceptance yet. |
| [PHLivePhotoEditingContext](https://developer.apple.com/documentation/photos/phlivephotoeditingcontext), frame processor, audio volume, preview, saved editing output | `NativeEditingSession` owns a real context; forwards the native frame-processing block and audio volume; prepares native previews or writes `PHContentEditingOutput`; propagates cancellation and errors. The host explicitly requests authorized editing input and commits the resulting same-asset output through `PhotoKitAdapter.commitEdit`. | Implemented adapter; native SDK gate. Adapter preview/save operations are serialized; adjustment data and prior host permission are required. Actual rendering, output acceptance, color/HDR fidelity, and asset commit require Apple fixtures/device validation. No private Photos effects are promised. |
| [AVCapturePhotoOutput](https://developer.apple.com/documentation/avfoundation/avcapturephotooutput), Live Photo capture settings and delegate callbacks | `LiveCaptureRequest` and iOS `CaptureContract.settings` validate the local movie destination and output support/enabled state. A host owns its capture session, permission, device configuration and delegate lifetime; it joins photo and movie completion/errors for the same request. | **Contract only**, not an implemented camera system. Core/browser capture reports unavailable. No camera/device test is claimed. |
| [LivePhotosKit Player](https://developer.apple.com/documentation/livephotoskitjs/livephotoskit.player): `photoSrc`, [`videoSrc`](https://developer.apple.com/documentation/livephotoskitjs/livephotoskit.player/videosrc), [`photoTime`](https://developer.apple.com/documentation/livephotoskitjs/livephotoskit.player/phototime), [`frameTimes`](https://developer.apple.com/documentation/livephotoskitjs/livephotoskit.player/frametimes), [`metadataVideoSrc`](https://developer.apple.com/documentation/livephotoskitjs/livephotoskit.player/metadatavideosrc) | `configureLivePhotosKitPlayer` accepts an app-supplied Apple Player and HTMLVideoElement; explicitly supplies core-derived `photoTime` and `frameTimes`, optionally supplies the original MOV blob as `metadataVideoSrc`. Unknown timing fails rather than inventing a default. The demo itself uses its own HTML media controls, not Apple's SDK. | Binding contract unit-tested. Per the user's cited official contract, URL `videoSrc` can supply timing metadata; passing HTMLVideoElement does not auto-detect these values; `metadataVideoSrc` can supply original-MOV timing separately. **Apple CDN/runtime is not loaded or tested.** |

Documentation retrieval limit (2026-10-19): the official pages were opened, but
returned the JavaScript-only shell to the web reader. Their linked Markdown
failed with `Unsupported content-type: text/markdown`; direct JSON requests from
the container failed with `Tunnel connection failed: 403 Forbidden`. The
LivePhotosKit timing distinction above is the user-provided cited contract,
implemented and tested as our binding behavior, **not an independently retrieved
full-body documentation verification**. No access controls were changed or
bypassed. Native SDK compilation validates signatures, not all API semantics.

## Resource and operation contracts

`liblivephoto-bridge::inspect` accepts existing `Input` variants and returns an
owned manifest plus exact still/movie bytes. A native JSON example and a WASM
binding use the same code. Every 64-bit time, extent or duration crossing JSON is
a decimal **string**; timescales remain positive integers. JavaScript converts
to floating point only at the playback boundary, while the exact values remain
visible and downloadable. Swift decodes exact `Int64`/`UInt32`; conversion to
`CMTime` rejects timescales outside positive `Int32` rather than rounding.

`video_frame_times` / `MotionPhoto::frame_times` reuse core sample-table parsing.
They read `stts`, signed/unsigned `ctts`, preserve decode sample indices, sort by
presentation time and map unit-rate edit lists including empty edits. Different
movie/media scales are combined exactly or fail on overflow. Allocation and
edit/sample iteration are bounded. Fragmented movies, non-unit edits or missing
sample tables return an explicit error; a movie can still be preserved and
extracted when its preview frame timing is unavailable. No constant frame-rate
inference or fabricated time zero is used. The existing 1/3-second microsecond
conversion rejection remains tested.

The Rust bridge exposes actual Parse / Validate / Extract / Complete phase
boundaries with callback cancellation. Synchronous parsing is not interruptible
inside a phase. Web parsing runs in a dedicated worker that is terminated on
cancel/replacement; FileReader is aborted during input reading; stale generations
cannot publish results. Progress reports actual bytes during reading and an
indeterminate parser phase. Each resulting media URL is revoked on replacement
or page disposal.

Swift `NativeOperation` provides progress, terminal success/failure/cancellation,
and exactly-once completion. Identifier/cancellation races and stale completions
are tested. Native library changes and export publication cross an explicit
commit boundary; cancel returns false after that boundary. Errors retain native
errors or typed adapter failures. Capabilities for capture/private Photos effects
return documented unavailable reasons rather than empty success stubs.

## Local browser behavior

The runnable [web demo](web/README.md) accepts a supported motion file or an Apple
pair. Ordinary unpaired still/video inputs require an explicit caller association.
An Apple identifier mismatch always fails, even when a previous preview exists.
It shows resource ranges, exact photo time, actual sample frame times, original
still/video extraction, diagnostics, and observed display/color facts. Unknown
EXIF/color/HDR remains unknown. Preserved ICC, gain-map or AGTM metadata is not
claimed as browser pixel rendering.

The player supports cover/video switching, play/pause/mute, keyboard operation,
seeking to the photo time and sample-based frame stepping. A compatible cover can
be drawn from the decoded video at the parsed photo time, or explicitly supplied
as a JPEG/PNG. Both are labelled preview-only, do not replace the original still,
and are not declared HDR-correct or pixel-equivalent. HEIC/HEVC decoding failures
are displayed with extraction still available. No implicit transcode or remote
conversion exists. Decoding unsupported video requires a compatible browser/OS
or a separately prepared local preview; this stage has no video transcode service.

The same input is used for sample-table timing, browser seeks and source pixel
comparison. Synthetic frame 13 is the cover at **13/16 second**. Android XMP
represents it as 812500/1000000; the Apple metadata track represents it as
1625/2000. Tests compare their rational equivalence, frame positions, extracted
MP4 bytes and decoded cover/video pixels. They also test invalid pairing,
unknown time, unavailable video, cancellation, keyboard and mobile layout.

No files are uploaded, no Apple script/CDN/analytics is fetched, and the server
binds only to `127.0.0.1`. It serves static GET/HEAD and rejects upload methods.
No deployment or publication has been performed.

## Still open

The [Google inventory](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/GOOGLE_COMPLETE_INVENTORY.md) retains every open
Google interface/provider/remux/wrapper item. Moments construction is excluded.
Native valid-pair loading/export now has actual tests plus a real RustCoreBridge C ABI client; the next CI receipt must establish their outcome. Native valid-pair PhotoKit import/edit/playback and Gallery/Photos UX still need
permissioned Apple device/corpus runs. Apple SDK runtime behavior, full official
document bodies, LivePhotosKit runtime, Safari/Firefox behavior and hardware
capture remain separate verification work. This is an implemented architecture
and local demonstration, not full Google or Apple ecosystem coverage.

The `d5729c7` Apple CI checkpoint also builds the static XCFramework and actually
executes the Swift bridge linked with Rust in iOS 26.5 Simulator (owned iPhone
17e instance): parse/exact time, original movie extraction, complete AGTM and
errors pass. Receipt: [mobile-and-sdk-d5729c7.json](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/mobile-and-sdk-d5729c7.json). It does not access a
Photos library or prove physical display/gesture/HDR/camera acceptance. Actual
LivePhotosKit runtime remains blocked by the environment proxy's CDN 403; see
[livephotoskit-access.json](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/livephotoskit-access.json) rather than treating the binding unit test
as an Apple SDK run.
