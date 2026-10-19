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

| ID | Public capability / shipped interface | Google authority | Current implementation and status |
| --- | --- | --- | --- |
| N01 | Parse path, memory, fd/range; MIME, directory, padding, exact cover time | `libmotionphoto_api.h` ParseMotionPhotoFrom*; `libmotionphoto_api.cc` | implemented/locally verified in the first stage |
| N02 | Check path, memory, fd/range and diagnostics | facade CheckMotionPhotoFrom*; checker/validator headers and implementations | structural validation implemented; complete specification/metadata diagnostic projection remains partial |
| N03 | Extract still, video, raw XMP individually or together | facade ExtractPrimaryImageFrom*, ExtractVideoTrackFrom*, ExtractMotionPhotoFrom* | exact extraction implemented; complete convenience/publishing interface and explicit optional track-removal reporting remain to be delivered |
| N04 | Extract timed AGTM from image/video path, memory, fd/range | facade ExtractAgtmFrom*; `MetadataEngine::ExtractAgtmAtTimestamp*` | complete version-0 fields, all curves/rules/slopes/custom chromaticities and raw mix coefficients implemented; standalone-video entry point verified; 12 same-input vectors match the reference projection; unknown scaling is not fabricated |
| N05 | Build from still/movie path, bytes, fd/ranges; output file or stream sink | facade BuildMotionPhotoFrom*; `motion_photo_builder/function.h` BuildMotionPhotoDirect | in-memory construction and caller publication implemented; direct sink/output convenience and interface layer pending |
| N06 | Version query | facade GetLibMotionPhotoVersion | crate version available; common public/CLI/Java query pending |
| M01 | Ingest/export standalone XMP document, camera/container values and validation | MotionPhoto, MotionPhotoEngine, Parser, Serializer, Reader, Writer headers | motion-file XMP read/write exists; standalone document API and field projection pending |
| M02 | Metadata collection on ordinary stills and motion photos; EXIF/XMP/ICC/JFIF/proprietary blocks, extents and typed payloads | MetadataEngine, ContainerSplitter, metadata_collection.proto | resource facade covers motion files; ordinary-still metadata engine, serialization and full block projection pending |
| M03 | Provider registration, disable-all and enable-list controls; serialized collection classification | MetadataProviderRegistry, IMetadataProvider, HandlerOptions, IsMotionPhoto | built-in readers exist; registry/filter/collection API pending |
| M04 | Existing scoring/stabilization metadata decode and semantic validation | video_metadata*, video_metadata_decoder*, MotionPhotoChecker | raw tracks preserved; full descriptor decode/validation pending; no Moments authoring required |
| P01 | Google JPEG, HEIC, AVIF, directory items, mpvd and padding | Google provider, JPEG/HEIF splitters/builders, mpvd headers/tests | implemented; exact property/media paths verified; avoid reference HEIF pixel re-encode |
| P02 | Legacy MicroVideo | plugins/legacy_micro_video provider/proto/tests | existing bounded JPEG contract; projection/filter/complete variants need audit |
| P03 | Samsung SEF trailer and HEIC sefd/mpvd, raw MP4/mpv2/mpv3 offsets, keys/version | plugins/samsung_sef provider/proto/tests | existing direct/pointer/BMFF reader and bounded direct writer; every reference variant and metadata field must be checked |
| P04 | Oplus OpCamera XMP attributes/elements, plain BMFF trailer, JXRS short/checksum/JSON-directory variants | plugins/oplus_olive provider/proto/tests | existing ColorOS LPEX reader verified; reference OpCamera/JXRS variants pending |
| P05 | Honor/Huawei 20/40-byte LIVE footer, HiHonor_/LivePhoto_ TLV, sceneMode, metadata-only trailers | plugins/honor_live_photo provider/proto/tests | implemented synthetic contract; distinguish metadata-only identification from a motion asset; wider projection pending |
| R01 | Inspect and preserve all movie tracks, sample descriptions, timing, metadata samples | MP4 reader/box reader, TrackIsAgtmMetadata, sample APIs | existing track/resource model; general metadata sample interface and unsupported layout audit pending |
| R02 | Copy/import selected tracks across files with reports | MotionPhotoMp4FileBuilder track-copy helpers | existing tracks preserved; general explicit import pending; Moments composition excluded |
| R03 | Delete metadata/selected tracks with reports | StripMetadataTrack and builder internals | bounded explicit metadata removal implemented; more general selection/reference handling pending |
| R04 | MP4 optimization, support query, track summary | OptimizeMp4File, IsOptimizationSupported, GetFileTrackSummary | fast-start/compaction pending; reference guard rejects hvc1 specifically |
| X01 | Moments video assembly and generated Moments metadata track | AddMomentsMp4FileTrack/AddMomentsMetadataTrack and builder -mv/-mx | **excluded by explicit user instruction**; not a gap or completion gate |
| C01 | Builder command, still/movie inputs, timestamp and output diagnostics | motion_photo_builder target/function/main | public convert example exists; complete equivalent CLI pending; Moments arguments excluded |
| C02 | Checker command and exit codes/diagnostics | motion_photo_checker target/function/main | inspection example exists; production command pending |
| C03 | Extractor command, optional image/video/XMP outputs | motion_photo_extractor target/function/main | library extraction exists; production command pending |
| C04 | Metadata command, details/classification, -agtm and -ts | motion_photo_metadata_engine target/function/main | test adapter exists; production command pending; reference -json flag is parsed but not acted on |
| J01 | Java builder path/memory and callback/error result | lib/java MotionPhotoBuilder, MotionPhotoJni; JNI exports including builderToByteArray | pending equivalent maintained Java/native interface; Moments inputs excluded |
| J02 | Java checker and extractor with diagnostics | MotionPhotoChecker, MotionPhotoExtractor, MotionPhotoJni | pending |
| J03 | Java metadata path/fd/range, collection classification and plugin controls | MetadataEngine, MetadataEngineJni | pending |
| J04 | Java AGTM path/bytes/fd/range/explicit timestamp; raw metadata extraction | MetadataEngine and JNI exports | pending |
| A01 | Isolated metadata parser service using ParcelFileDescriptor; bounded collection validation and ownership | IMetadataParserService.aidl, MetadataParserService, manifest | pending; actual Android execution must be recorded separately |
| A02 | Builder service path/memory | IMotionPhotoBuilderService.aidl, MotionPhotoBuilderService | pending; Moments parameters excluded |
| A03 | Checker service | IMotionPhotoCheckerService.aidl, MotionPhotoCheckerService | pending |
| A04 | Extractor service with optional output paths | IMotionPhotoExtractorService.aidl, MotionPhotoExtractorService | pending |
| V01 | Java/native and Android service integration tests/build packaging | lib/java/build.gradle, lib/android/build.gradle, javatests, core CMake JNI target | reference host JNI compiled; LibLivePhoto packaging/tests pending |
| V02 | Linux/macOS/Windows existing consumers and fixture/fuzz gates | user repository verify.yml; Google build_and_test.yml | first-stage Linux/macOS/Windows, fixtures and sanitizer CI passed at 88e0fdd (receipt ci-88e0fdd.json); newer changes require their own checks |

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
