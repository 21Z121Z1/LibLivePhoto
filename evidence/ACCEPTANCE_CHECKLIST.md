# Auditable completion checklist

User's latest goal is complete capability coverage across every board. The sole
explicit feature exclusion is **Moments construction**. No board is labelled
100% while required rows or native acceptance gates remain open. A supported,
validated layout does not imply every vendor, codec, device or MP4 layout works.

Reference: Google `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`. Detailed Google rows
remain in [GOOGLE_COMPLETE_INVENTORY.md](GOOGLE_COMPLETE_INVENTORY.md), including
source/header/implementation/test authority. This list adds delivery/acceptance
gates; it does not remove or replace those rows.

Stability is a hard gate for all boards. See [STABILITY_GATES.md](STABILITY_GATES.md)
for malformed ranges, cumulative allocation/copy bounds, FFI ownership,
cancellation, failure rollback and the limits of each runtime acceptance claim.
The [pinned source/test map](SOURCE_TEST_MAP.md) distinguishes public facade,
internal builder use and operation-specific input/output differences. For the
current branch SHA, Android is verified only if its CI artifact contains
`runtime-receipt.json` with all four completed scopes; a partial trace below is
historical evidence, not a substituted pass. Emulator success covers that x86_64
API35 runtime, not an arm64 device, every API26+ runtime, or vendor Gallery.

| Board / stable gate | Actual deliverable / acceptance | Current evidence or open requirement |
| --- | --- | --- |
| Core C-INPUT | Per-operation path/memory/open-file and Unix fd/range, budgets, cursor ownership, output sink/publication distinction | First-stage tests and CI receipts; new metadata range/fd parity tests. CLI/JVM FileChannel and Android FD wrappers now expose per-operation limits and preserve caller ownership; installed Android runtime is a separate gate. |
| Core C-TIME | Integer time, exact conversion/overflow rejection, sample composition/edit timing, no fabricated cover time | Rust regression incl. 1/3 rejection; frame 13 at 13/16 in browser and Swift manifest. |
| Core C-SEMANTICS | Pairing/provenance, orientation/transforms, explicit color/HDR facts, codec/AAC capability contracts, unknown-resource handling | Existing contract/media tests; native color/HDR rendered-fidelity acceptance remains open. |
| Core C-METADATA | Ordinary images, XMP document, collection raw/header extents, source validation, registration/filtering, existing AGTM and score/flag decode | Implemented at 3f5287e; 9-vector descriptor and 38-case provider/control/raw-block differential coverage added; original consumers retained. No Moments authoring. |
| Formats F-GOOGLE | JPEG/HEIC/AVIF directory, padding/mpvd/time; byte-preserving parse/extract/build | Existing fixtures/differentials. Pinned Google differences documented. |
| Formats F-VENDORS | Exact MicroVideo, Samsung, Oplus and Honor variants and limits, disabled providers, metadata-only distinction | Synthetic/reference tests plus current legal corpus; same-input canonical/alias/filter/raw-block and typed provider field audit added. Honor original-device redistributable corpus and vendor Gallery acceptance are absent. |
| Formats F-APPLE | Still/movie pairing and timed metadata, original resources/unknown facts, supported lossless conversion | Existing core regression; actual macOS PhotoKit valid local pair load and byte-exact resource export passed at 83018c4. Photos-library import/edit and physical-device acceptance remain separate and open. |
| Remux R-TRACKS | Explicit copy/import/delete, IDs/dependencies, metadata tracks, byte/timing/property preservation, detailed disposition | Bounded standalone implementation + source/packet tests. Fragmented/indexed/external/reference-bearing layouts are explicitly blocked, not called fully compatible. Pinned copy/delete behavior audited, including reference time-shift difference. |
| Remux R-OPTIMIZE | Support query, fast-start/compaction, correct offsets/large fields, hvc1 boundary, unknown bytes retained/reported | 31c9f93 tests/receipt; six pinned optimizer/copy/delete comparisons include real H.264 B frames, AAC, Apple metadata and hvc1; unsupported layouts remain explicit. |
| Google G-CLI | Complete equivalent build/check/extract/metadata/AGTM/version commands, errors/exit codes and packaging | Maintained llp commands implemented and integration-tested; release CLI/C ABI/Java archive and external consumer checks passed on three host platforms at 9fa6208; complete metadata and AGTM interfaces have shared-core tests. |
| Google G-JAVA | Maintained Java model and JNI build/check/extract/metadata/AGTM APIs, path/memory/range, callbacks/errors/ownership | Java/JNI and isolated C ABI implemented; actual host JVM memory/path/channel-range/build/extract/error/cancel/lifetime smoke and C calls passed. Three-platform packaged release/external Java consumer passed at 9fa6208; actual AGTM path/memory/range and raw metadata cases added. |
| Google G-ANDROID | Metadata parser and builder/checker/extractor services, fd ownership/bounds, isolated service and usable package/client | Implemented and verified at a10e5d2: actual SDK36/NDK27.3 dual-ABI AAR/APK, API35 x86_64 installed runtime, all four remote services and scopes passed (core20/metadata11/lifecycle16/concurrency18; 65 assertions). Raw AGTM, exact bytes/time, range/cursor, repeated calls, queued readers and terminal cancellation executed. [Runtime receipt](android-runtime-a10e5d2.json). Arm64 device and native Gallery remain separate; no device permission changes. |
| Apple A-CORE-BRIDGE | Native Swift actually invokes Rust core; exact resource/time/diagnostic contract and real tests | Actual C ABI binding and mandatory native library CI tests passed at 83018c4 (10 Swift tests total). iOS static XCFramework and actual statically linked Swift/Rust operations passed in iOS 26.5 Simulator at d5729c7; physical signed-app/display acceptance remains separate. |
| Apple A-RESOURCES | PHLivePhoto load, resource inventory and export; PHAsset creation/import with pairedVideo and correct permission/commit behavior | At 83018c4 actual macOS PhotoKit loads the generated valid pair and exports photo/pairedVideo bytes exactly; 10 native tests passed. Permitted Photos-library import/edit acceptance remains open. |
| Apple A-PLAYBACK | System live view, playback state, mute and gesture behavior | iOS SDK compile passed; macOS system-view object/lifecycle test passed at d5729c7 (11 Swift tests); macOS public gesture accessor is unavailable in the SDK; physical playback/gesture/display acceptance remains open. |
| Apple A-EDIT | Frame processor/audio/preview/output and asset edit commit, cancellation/progress/errors | Editing input, host adjustment/output and explicit commit APIs compile after the SDK callback fix; permissioned asset/render/commit/color runs remain open. |
| Apple A-CAPTURE | Explicit host-owned AVCapturePhotoOutput adapter contract and native availability | Settings/support contract exists; no portable camera/private Photos effects claim. Hardware acceptance requires a supported device and host capture session. |
| Web W-BINDING | Actual Rust WASM, local motion/pair inputs, exact photo/frame timing and extraction | Implemented; local and remote Chrome tests passed; no remote upload, deploy or implicit transcode. |
| Web W-INTERACTION | Responsive/keyboard load/switch/play/pause/mute/seek/step/cancel, clear invalid/unsupported states and compatible preview | Seven actual Chrome tests include AAC decode/mute, HEIC and unsupported-movie codec errors, original-byte downloads, URL disposal and replacement/cancel races; no physical speaker/HDR claim. |
| Web W-TIMING | Cover/video pixel and timeline test; HTMLVideoElement photoTime/frameTimes explicitly supplied; original metadataVideoSrc path | 48 sample times, frame 13, exact rational seek, independent compositor mediaTime=13/16 observation, pixel comparison and optional LPK-binding unit test. Apple Player runtime/full document-body verification remains open. |
| Release V-REGRESSION | Exact-SHA Linux/macOS/Windows, corpus, consumers, sanitizers, frontend/native builds with evidence | Existing jobs retained; read every checkpoint result. Compile success is not device Gallery/Photos acceptance. |
| Release V-DATA | Legal redistributable test data, dependency-specific licensing, no private images/credentials, original worktrees intact | Synthetic CC0 demo and locked fixture provenance; no private media committed. SDK/AAR/XCFramework now retain 46 locked dependency inventories/notices; missing jni-sys-macros workspace licenses were recovered from its exact recorded source commit. |

Known environment/source limits must be reported as concrete blockers with the
minimal required input. Current examples: a permissioned Apple device/Photos
library for import/edit/playback UX; a passing Android installed-service runtime;
legally redistributable original Honor variants for corpus/device acceptance;
full Apple document bodies unavailable through the current reader/container.
Independent implementation/testing continues while those gates are unavailable.
No merge, force push, release or public website deployment is authorized.
