# Auditable completion checklist

User's latest goal is complete capability coverage across every board. The sole
explicit feature exclusion is **Moments construction**. No board is labelled
100% while required rows or native acceptance gates remain open. A supported,
validated layout does not imply every vendor, codec, device or MP4 layout works.

Reference: Google `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`. Detailed Google rows
remain in [GOOGLE_COMPLETE_INVENTORY.md](GOOGLE_COMPLETE_INVENTORY.md), including
source/header/implementation/test authority. This list adds delivery/acceptance
gates; it does not remove or replace those rows.

| Board / stable gate | Actual deliverable / acceptance | Current evidence or open requirement |
| --- | --- | --- |
| Core C-INPUT | Per-operation path/memory/open-file and Unix fd/range, budgets, cursor ownership, output sink/publication distinction | First-stage tests and CI receipts; new metadata range/fd parity tests. Wrapper exposure remains open. |
| Core C-TIME | Integer time, exact conversion/overflow rejection, sample composition/edit timing, no fabricated cover time | Rust regression incl. 1/3 rejection; frame 13 at 13/16 in browser and Swift manifest. |
| Core C-SEMANTICS | Pairing/provenance, orientation/transforms, explicit color/HDR facts, codec/AAC capability contracts, unknown-resource handling | Existing contract/media tests; native color/HDR rendered-fidelity acceptance remains open. |
| Core C-METADATA | Ordinary images, XMP document, collection raw/header extents, source validation, registration/filtering, existing AGTM and score/flag decode | Implemented at 3f5287e; exact metadata receipt and continued differential/consumer checks required. No Moments authoring. |
| Formats F-GOOGLE | JPEG/HEIC/AVIF directory, padding/mpvd/time; byte-preserving parse/extract/build | Existing fixtures/differentials. Pinned Google differences documented. |
| Formats F-VENDORS | Exact MicroVideo, Samsung, Oplus and Honor variants and limits, disabled providers, metadata-only distinction | Synthetic/reference tests plus current legal corpus; complete per-provider differential audit remains open. Honor original-device redistributable corpus and vendor Gallery acceptance are absent. |
| Formats F-APPLE | Still/movie pairing and timed metadata, original resources/unknown facts, supported lossless conversion | Existing core regression; actual macOS PhotoKit valid local pair load and byte-exact resource export passed at 83018c4. Photos-library import/edit and physical-device acceptance remain separate and open. |
| Remux R-TRACKS | Explicit copy/import/delete, IDs/dependencies, metadata tracks, byte/timing/property preservation, detailed disposition | Bounded standalone implementation + source/packet tests. Fragmented/indexed/external/reference-bearing layouts are explicitly blocked, not called fully compatible. Reference behavior audit remains open. |
| Remux R-OPTIMIZE | Support query, fast-start/compaction, correct offsets/large fields, hvc1 boundary, unknown bytes retained/reported | 31c9f93 tests/receipt; actual pinned optimizer differential and broader codec/layout tests remain open. |
| Google G-CLI | Complete equivalent build/check/extract/metadata/AGTM/version commands, errors/exit codes and packaging | Maintained llp commands implemented and integration-tested; cross-platform command/artifact CI and broader reference command comparisons remain open. |
| Google G-JAVA | Maintained Java model and JNI build/check/extract/metadata/AGTM APIs, path/memory/range, callbacks/errors/ownership | Java/JNI and isolated C ABI implemented; actual host JVM memory/path/channel-range/build/extract/error/cancel/lifetime smoke and C calls passed. Cross-platform packaging/per-method cases remain open. |
| Google G-ANDROID | Metadata parser and builder/checker/extractor services, fd ownership/bounds, isolated service and usable package/client | Four services, FD/range client and AAR/installed-APK acceptance implemented; SDK/NDK and runtime execution pending. No local Android SDK/device. |
| Apple A-CORE-BRIDGE | Native Swift actually invokes Rust core; exact resource/time/diagnostic contract and real tests | Actual C ABI binding and mandatory native library CI tests passed at 83018c4 (10 Swift tests total). Signed iOS native-library embedding/runtime is a separate open gate. |
| Apple A-RESOURCES | PHLivePhoto load, resource inventory and export; PHAsset creation/import with pairedVideo and correct permission/commit behavior | At 83018c4 actual macOS PhotoKit loads the generated valid pair and exports photo/pairedVideo bytes exactly; 10 native tests passed. Permitted Photos-library import/edit acceptance remains open. |
| Apple A-PLAYBACK | System live view, playback state, mute and gesture behavior | iOS simulator-SDK compile passed; real playback/gesture acceptance open. |
| Apple A-EDIT | Frame processor/audio/preview/output and asset edit commit, cancellation/progress/errors | Adapter compile passed; actual permitted PhotoKit asset/render/output/color runs open. |
| Apple A-CAPTURE | Explicit host-owned AVCapturePhotoOutput adapter contract and native availability | Settings/support contract exists; no portable camera/private Photos effects claim. Hardware acceptance requires a supported device and host capture session. |
| Web W-BINDING | Actual Rust WASM, local motion/pair inputs, exact photo/frame timing and extraction | Implemented; local and remote Chrome tests passed; no remote upload, deploy or implicit transcode. |
| Web W-INTERACTION | Responsive/keyboard load/switch/play/pause/mute/seek/step/cancel, clear invalid/unsupported states and compatible preview | Four actual browser tests and screenshots; add broader real-codec/error/audio/cleanup acceptance before closing. |
| Web W-TIMING | Cover/video pixel and timeline test; HTMLVideoElement photoTime/frameTimes explicitly supplied; original metadataVideoSrc path | 48 sample times, frame 13, exact rational seek, pixel comparison and optional LPK-binding unit test. Apple Player runtime/full document-body verification remains open. |
| Release V-REGRESSION | Exact-SHA Linux/macOS/Windows, corpus, consumers, sanitizers, frontend/native builds with evidence | Existing jobs retained; read every checkpoint result. Compile success is not device Gallery/Photos acceptance. |
| Release V-DATA | Legal redistributable test data, dependency-specific licensing, no private images/credentials, original worktrees intact | Synthetic CC0 demo and locked fixture provenance; no private media committed. New binding dependencies require their own license review. |

Known environment/source limits must be reported as concrete blockers with the
minimal required input. Current examples: a permissioned Apple device/Photos
library for import/edit/playback UX; an Android device/emulator/service runtime;
legally redistributable original Honor variants for corpus/device acceptance;
full Apple document bodies unavailable through the current reader/container.
Independent implementation/testing continues while those gates are unavailable.
No merge, force push, release or public website deployment is authorized.
