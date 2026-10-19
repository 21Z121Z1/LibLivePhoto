# Complete Google interface inventory

Reference: `google/libmotionphoto@12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`.
This inventory extends the earlier core-only assessment. It is an open work list,
not a completion claim. The first implementation and receipts remain available
at `88e0fdd92fb9ff7bb963b64e6676e4ec1d73b0d4`.

Scope update: the user explicitly excluded **Moments construction** on
2026-10-19. Creation/import of Moments video plus its generated scoring metadata
is excluded from completion criteria. Existing metadata reading/preservation is
retained. General track import and explicitly reported deletion remain in scope.
Java/JNI, CLI and Android services are now in scope. Rust/Java APIs may use their
own data structures; missing user capabilities cannot be hidden by that choice.

Source locations below are relative to the pinned Google repository. Header
declarations are checked against implementations, tests and CMake/Gradle targets.
The [source/test map](SOURCE_TEST_MAP.md) links pinned declarations and
implementations to concrete regressions, including each operation's different
input/output behavior. [Stability gates](STABILITY_GATES.md) and
[cd2159d evidence](stability-cd2159d.json) record cumulative budgets and actual
local/host verification. Installed runtime status belongs to the exact-SHA CI
artifact, separate from compilation and the historical partial traces below.
At verified baseline `4848e5fb80b650fdefcedbe9aebedef9de7e627f`, all three
workflows passed, including all four Android scopes/65 assertions and an actual
SE3/iOS26.2 Swift/Rust process. [Exact baseline evidence](stable-baseline-4848e5f.json)
binds job IDs, source trees and artifacts. This closes bounded API35/x86_64 and
iOS26.2 process gates; it does not establish physical arm64/Gallery/Photos
acceptance or resolve earlier iOS26.5 startup failures. The
[current acceptance list](ACCEPTANCE_CHECKLIST.md) and [remaining work](REMAINING_WORK.md)
supersede historical checkpoint priorities below.
The [76c25e4 continuation](standards-edit-76c25e4.json) independently passed all
three workflows with171 Rust tests, public-fixture2 and native processes. It
adds exact AGTM unit-rate edit selection, fixes HEIF mirror/absolute-item
preservation and XMP padding output, and binds the
[normative clauses](NORMATIVE_CONFORMANCE.md) to positive/negative tests.

Code `91fc38065d90f09ad971a89097cc022d20bebae6` adds H05's selected color
rules, explicit scoring-media-time projection, optional G08/G09 declarations
and bounded Android startup diagnostics. Local default/all-feature runs each
passed197 tests, with public fixture2 executed separately,85 same-input
comparisons,1322483 ASan executions, actual C/JNI/CLI, unchanged XDRemux replay
and seven Chromium tests. [Exact source/run receipt](conformance-91fc380.json)
and [full differential receipts](differential-91fc380.json) keep native CI status
separate. Its Linux/macOS/Windows, corpus/ASan and Apple/web workflows passed
(197 tests per default/all-feature run on Linux/macOS,196 on Windows, which
excludes one Unix-only FD test);
Android packaging passed but actual acceptance failed during APK installation,
before any service scope. [Install failure and inaccessible saved diagnostics](android-install-91fc380.json)
remain explicit; host fault-injection markers are not native passes.

Code `8dbe8300c0915dc0c921fbbbabb4b219577f84da` adds bounded CameraAlbum
raw-footer inspection and metadata blocks, with eight independent regressions
and both existing vivo originals. Default/all-feature205, public-fixture2,
provider differential38, actual C/JNI, unchanged XDRemux replay and asset ASan95130
passed locally. [Exact source/command receipt](vendor-footer-8dbe830.json) and
[five-vendor evidence matrix](VENDOR_RESEARCH_20261019.md) distinguish original
bytes, static inference, listing-only versions and absent native acceptance.
The preceding Android diagnostics change makes install results visible in job
logs and suppresses simulated host success output; no guest-startup root-cause
fix is claimed. Atc15648c the same source objects passed all portable, corpus,
ASan and Apple/web jobs. Android installed core20 passed, then metadata returned
Process crashed/code0; later scopes did not execute. [Terminal evidence](platform-c15648c.json)
keeps that failed overall gate. The follow-up [crash-diagnostic receipt](android-crash-harness-b5ceff4.json)
records13 host fault tests and unchanged65 native assertions/deadlines; its new
platform run remains required.

| ID | Public capability / shipped interface | Google authority | Current implementation and status |
| --- | --- | --- | --- |
| N01 | Parse path, memory, fd/range; MIME, directory, padding, exact cover time | `libmotionphoto_api.h` ParseMotionPhotoFrom*; `libmotionphoto_api.cc` | implemented/verified: Rust path/memory/fd/range plus CLI/JVM FileChannel and Android FD wrappers; per-wrapper budgets and descriptor ownership are explicit; installed Android runtime remains a separate gate |
| N02 | Check path, memory, fd/range and diagnostics | facade CheckMotionPhotoFrom*; checker/validator headers and implementations | structural/resource and existing score/stabilization checks implemented; invalid check results return success=false with diagnostics; bounds/overflow/time and metadata regressions run; native Gallery acceptance is separate |
| N03 | Extract still, video, raw XMP individually or together | facade ExtractPrimaryImageFrom*, ExtractVideoTrackFrom*, ExtractMotionPhotoFrom* | exact still/video/raw XMP and selected-resource extraction exposed through Rust, CLI, C/Java/Swift; optional XMP absence is explicit; track removal is selected/reported, never implicit |
| N04 | Extract timed AGTM from image/video path, memory, fd/range | facade ExtractAgtmFrom*; `MetadataEngine::ExtractAgtmAtTimestamp*` | complete version-0 fields, all curves/rules/slopes/custom chromaticities and raw mix coefficients implemented; standalone-video entry point verified; 12 same-input vectors match the reference projection; unknown scaling is not fabricated |
| N05 | Build from still/movie path, bytes, fd/ranges; output file or stream sink | facade BuildMotionPhotoFrom*; `motion_photo_builder/function.h` BuildMotionPhotoDirect | existing lossless plan/execute plus CLI safe-new-directory and caller-owned C/JVM/Swift outputs; path/memory/ranges enter through snapshots; no imitation of reference HEIF pixel re-encoding; wrapper budgets documented |
| N06 | Version query | facade GetLibMotionPhotoVersion | core version(), llp version, C ABI/API version and Java native version verified, including extracted SDK archives on three host platforms |
| M01 | Ingest/export standalone XMP document, camera/container values and validation | MotionPhoto, MotionPhotoEngine, Parser, Serializer, Reader, Writer headers | standalone XmpDocument/raw serialization/projection/rewrite implemented; resource validation remains source-dependent |
| M02 | Metadata collection on ordinary stills and motion photos; EXIF/XMP/ICC/JFIF/proprietary blocks, extents and typed payloads | MetadataEngine, ContainerSplitter, metadata_collection.proto | ordinary JPEG/HEIC/AVIF collections, header/resource extents, raw views/complete-block extraction, typed projections and untrusted JSON classification implemented; 38 pinned same-input provider/control cases and raw-byte tests; HEIF no-mpvd and namespace-alias differences documented |
| M03 | Provider registration, disable-all and enable-list controls; serialized collection classification | MetadataProviderRegistry, IMetadataProvider, HandlerOptions, IsMotionPhoto | owned provider registration, disable-all/enable-list options, core parse filtering and collection declaration classification implemented; custom native facade composition is not inferred from semantic declarations |
| M04 | Existing scoring/stabilization metadata decode and semantic validation | video_metadata*, video_metadata_decoder*, MotionPhotoChecker | existing scoring/stabilization descriptors and semantic checks implemented; nine pinned decoder vectors match; raw/unknown bits retained; no Moments authoring |
| P01 | Google JPEG, HEIC, AVIF, directory items, mpvd and padding | Google provider, JPEG/HEIF splitters/builders, mpvd headers/tests | implemented; exact property/media paths verified; avoid reference HEIF pixel re-encode |
| P02 | Legacy MicroVideo | plugins/legacy_micro_video provider/proto/tests | bounded JPEG legacy attributes/elements, provider filtering and raw metadata extraction implemented; canonical GCamera plus namespace-equivalent alias behavior audited against pinned source; source verification separate from declarations |
| P03 | Samsung SEF trailer and HEIC sefd/mpvd, raw MP4/mpv2/mpv3 offsets, keys/version | plugins/samsung_sef provider/proto/tests | raw/inline mpv2/mpv3/pointer and HEIF sefd/mpvd/stub layouts implemented and regression-tested; same-input raw/version/key/magic/length checks; writer remains direct JPEG only; native Gallery and wider original-device corpus are unverified |
| P04 | Oplus OpCamera XMP attributes/elements, plain BMFF trailer, JXRS short/checksum/JSON-directory variants | plugins/oplus_olive provider/proto/tests | JPEG OpCamera/JXRS plain/short/checksum/JSON-directory reads implemented; container-independent OpCamera XMP covers HEIC metadata declarations; same-input canonical/alias and HEIF mpvd/no-mpvd cases; this does not promise all vendor Gallery behavior or native writers |
| P05 | Honor/Huawei 20/40-byte LIVE footer, HiHonor_/LivePhoto_ TLV, sceneMode, metadata-only trailers | plugins/honor_live_photo provider/proto/tests | 20/40-byte LIVE and HiHonor_/LivePhoto_ TLV, sceneMode, metadata-only distinction and unknown raw records implemented; synthetic same-input decode/raw fields verified; legally redistributable original Honor device corpus and native acceptance remain absent |
| P06 | Additional CameraAlbum raw trailer inspection (outside pinned Google providers) | Not a pinned Google capability; independent original-byte and static evidence in VENDOR_RESEARCH_20261019.md | Bounded43-byte terminal layout verified on two existing vivo originals;55-byte layout static/synthetic only. Raw JSON/block/declared integer and ID agreement are exposed. No inferred time unit, unique capture pairing, depth, native writer or Gallery compatibility claim. |
| R01 | Inspect and preserve all movie tracks, sample descriptions, timing, metadata samples | MP4 reader/box reader, TrackIsAgtmMetadata, sample APIs | Raw packet CTS, exact frames and AGTM unit-rate empty/trim/repeat mapping implemented. Raw scoring fields retain declared-global-time validation; optional explicitly associated video-media-CTS projection reports repeats/trims/unspecified without rewriting payloads. Optional G08/G09 profile checks declarations/order/exact recommended correspondence. Non-unit/fragmented mapping and fractional destination conversions remain explicit unsupported/error cases. |
| R02 | Copy/import selected tracks across files with reports | MotionPhotoMp4FileBuilder track-copy helpers | bounded general explicit import with unique IDs, exact movie-time mapping, byte verification and movie-metadata sidecars; pinned MP4CopyTrack comparison exposes a 1/8-second B-frame shift in reference copy while Rust retains source time; reference-bearing layouts rejected |
| R03 | Delete metadata/selected tracks with reports | StripMetadataTrack and builder internals | explicit selected-track deletion with disposition reports and byte/time checks; pinned StripMetadataTrack and primary-builder deletion audited on existing metadata; referenced/unknown layouts rejected |
| R04 | MP4 optimization, support query, track summary | OptimizeMp4File, IsOptimizationSupported, GetFileTrackSummary | fast-start/co64 relocation and opt-in compaction implemented; six pinned optimizer/copy/delete checks include real hvc1 and AAC; Google support query rejects hvc1 while Rust lossless relocation preserves packets/time; unsupported layouts remain bounded and explicit |
| X01 | Moments video assembly and generated Moments metadata track | AddMomentsMp4FileTrack/AddMomentsMetadataTrack and builder -mv/-mx | **excluded by explicit user instruction**; not a gap or completion gate |
| C01 | Builder command, still/movie inputs, timestamp and output diagnostics | motion_photo_builder target/function/main | maintained llp build command with exact time/plans/targets/pairing/resource policy and safe new publication; CLI integration plus extracted release SDK on Linux/macOS/Windows passed at 9fa6208; Moments excluded |
| C02 | Checker command and exit codes/diagnostics | motion_photo_checker target/function/main | llp check structured diagnostics/exit codes implemented and tested; metadata-invalid checks also return success=false; original fixture and consumer gates retained |
| C03 | Extractor command, optional image/video/XMP outputs | motion_photo_extractor target/function/main | llp extract selected still/video/XMP/resources and extract-metadata full raw blocks, ranges and new outputs; exact byte and overwrite-refusal tests run; absent optional XMP reported |
| C04 | Metadata command, details/classification, -agtm and -ts | motion_photo_metadata_engine target/function/main | llp metadata/classify-metadata/extract-metadata/agtm/xmp implemented; exact explicit timestamps/provider controls and complete raw AGTM; reference -json parsed-but-unused difference remains documented |
| J01 | Java builder path/memory and callback/error result | lib/java MotionPhotoBuilder, MotionPhotoJni; JNI exports including builderToByteArray | maintained Java/JNI build and async callback/state/error contract; actual memory/path/range/cancel/lifetime checks plus three-platform packaged release jar consumer passed at 9fa6208; no Moments construction |
| J02 | Java checker and extractor with diagnostics | MotionPhotoChecker, MotionPhotoExtractor, MotionPhotoJni | check/extract/raw metadata and selected operation schema implemented; exact original-byte/invalid-time/bounded stream tests run; Android service runtime separate |
| J03 | Java metadata path/fd/range, collection classification and plugin controls | MetadataEngine, MetadataEngineJni | FileChannel descriptor/range ownership/cursor and metadata declaration classification/provider controls verified; complete raw source-block extraction available; no protobuf or C++ ABI compatibility promised |
| J04 | Java AGTM path/bytes/fd/range/explicit timestamp; raw metadata extraction | MetadataEngine and JNI exports | complete AGTM JSON/raw payload via image photo time and standalone-video exact timestamp; actual JVM memory/path/channel-range equivalence, unknown/unrepresentable-time failures tested; extraction is not pixel rendering |
| A01 | Isolated metadata parser service using ParcelFileDescriptor; bounded collection validation and ownership | IMetadataParserService.aidl, MetadataParserService, manifest | isolated parser/FD/raw metadata/AGTM/client/queue/pipe implemented; SDK36/NDK27.3 arm64+x86_64 built. Baseline4848e5f installed API35/x86_64 all four scopes/65 assertions passed, including metadata/lifecycle/concurrency; physical arm64 and vendor Gallery remain unverified |
| A02 | Builder service path/memory | IMotionPhotoBuilderService.aidl, MotionPhotoBuilderService | FD builder implemented; memory/path use host FDs or direct Java engine. Baseline4848e5f installed runtime executed build, exact bytes/time and 1/3 rejection within the complete four-scope/65-assertion receipt; Moments excluded |
| A03 | Checker service | IMotionPhotoCheckerService.aidl, MotionPhotoCheckerService | role-limited checker/owner UID/FD/callback bounds implemented. Baseline4848e5f installed runtime executed version/check and wrong-role/range rejection within all four completed scopes |
| A04 | Extractor service with optional output paths | IMotionPhotoExtractorService.aidl, MotionPhotoExtractorService | selected/raw metadata output pipe implemented; host publishes new files. Baseline4848e5f installed runtime executed byte-exact movie extraction, complete raw AGTM and terminal cancellation within all four completed scopes |
| V01 | Java/native and Android service integration tests/build packaging | lib/java/build.gradle, lib/android/build.gradle, javatests, core CMake JNI target | C/JNI/Java8 jar and extracted SDK acceptance passed on Linux/macOS/Windows at 9fa6208; Android AAR/real test APK and Apple static XCFramework built; mobile runtime/native permission gates remain separate |
| V02 | Linux/macOS/Windows existing consumers and fixture/fuzz gates | user repository verify.yml; Google build_and_test.yml | cross-platform original consumers/corpus/fuzz retained; verified baseline4848e5f passed all three workflows,154 all-feature tests plus explicit fixture job, three-host native/JVM SDK consumers,3142603 ASan runs, Android65 assertions and actual iOS26.2 Swift/Rust process. Later code needs separate execution evidence |

The `samples/android` activity/fragments are integration demonstrations. Their
library-backed parse/check/build/extract actions are represented above. Camera2,
MediaRecorder and playback UI code are platform demo clients, not new portable
library capture/playback APIs. No unrelated capture, player or editing SDK is added.

The old core-only run is a historical receipt. It does not satisfy newly added
interface requirements. Every subsequent stage must list the exact commit tested,
the executed command/results, platform limits, and the still-open IDs above.

## AGTM checkpoint and next priority (2026-10-19)

Code commit `1ea8c484dbb6fcf6189763aeb9cf1396851a9ed7` passed 107 workspace/all-feature Rust tests, clippy with warnings denied, 12 pinned-Google same-input AGTM cases, and 49,312 ASan fuzz runs. Commands: `cargo test --locked --workspace --all-targets --all-features`; `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`; `python3 scripts/agtm_differential.py --reference-source /workspace/libmotionphoto-reference`; `cargo +nightly fuzz run asset_input -- -max_total_time=30 -print_final_stats=1`. Exact receipt: [agtm-stage.json](agtm-stage.json). Three external-data tests are ignored in this default run; prior fixture results remain historical. No new native-gallery acceptance is claimed.

The user next prioritized Apple public-API architecture and a real local browser demo. Remaining Google IDs above stay open, including wrappers, Android services, generic remux/optimization, provider variants and metadata engine gaps. Moments construction remains excluded. Apple/frontend work does not imply these Google items are complete.

## Maintained wrapper and remux checkpoint

See [native SDK integration](../NATIVE_SDK.md), [auditable acceptance gates](ACCEPTANCE_CHECKLIST.md),
`metadata_differential.py`, `mp4_differential.py`, and exact stage receipts. Historical
priority notes above do not mean later implemented rows are still missing. Only
installed runtime/device/legal corpus/permission gates can close those specific
requirements. Every supported variant is bounded by its concrete tests; an
unknown layout is rejected or preserved with an explicit disposition.
