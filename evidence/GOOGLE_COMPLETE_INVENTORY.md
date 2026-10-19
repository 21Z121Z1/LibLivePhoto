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
| N06 | Version query | facade GetLibMotionPhotoVersion | public core version() implemented; CLI/Java exposure remains pending |
| M01 | Ingest/export standalone XMP document, camera/container values and validation | MotionPhoto, MotionPhotoEngine, Parser, Serializer, Reader, Writer headers | standalone XmpDocument/raw serialization/projection/rewrite implemented; resource validation remains source-dependent |
| M02 | Metadata collection on ordinary stills and motion photos; EXIF/XMP/ICC/JFIF/proprietary blocks, extents and typed payloads | MetadataEngine, ContainerSplitter, metadata_collection.proto | ordinary JPEG/HEIC/AVIF metadata engine, block/header extents, bounded raw views, JSON round trip and validated-source distinction implemented; broader splitter/provider differential audit remains |
| M03 | Provider registration, disable-all and enable-list controls; serialized collection classification | MetadataProviderRegistry, IMetadataProvider, HandlerOptions, IsMotionPhoto | owned provider registration, disable-all/enable-list options, core parse filtering and collection declaration classification implemented; custom native facade composition is not inferred from semantic declarations |
| M04 | Existing scoring/stabilization metadata decode and semantic validation | video_metadata*, video_metadata_decoder*, MotionPhotoChecker | existing scoring/stabilization descriptor decode, raw bit preservation, semantic checks and metadata packet API implemented; nine vectors match pinned decoder; no Moments authoring required |
| P01 | Google JPEG, HEIC, AVIF, directory items, mpvd and padding | Google provider, JPEG/HEIF splitters/builders, mpvd headers/tests | implemented; exact property/media paths verified; avoid reference HEIF pixel re-encode |
| P02 | Legacy MicroVideo | plugins/legacy_micro_video provider/proto/tests | existing bounded JPEG contract; projection/filter/complete variants need audit |
| P03 | Samsung SEF trailer and HEIC sefd/mpvd, raw MP4/mpv2/mpv3 offsets, keys/version | plugins/samsung_sef provider/proto/tests | raw/direct, inline mpv2/mpv3, pointer, HEIF sefd before/after mpvd and empty stub reads plus metadata-only collection implemented/tested; native writer remains bounded direct JPEG; wider provider/corpus audit remains |
| P04 | Oplus OpCamera XMP attributes/elements, plain BMFF trailer, JXRS short/checksum/JSON-directory variants | plugins/oplus_olive provider/proto/tests | existing ColorOS LPEX retained; OpCamera and plain/short/checksum/JSON-directory JPEG variants implemented and same-input verified; broader HEIF/corpus audit remains |
| P05 | Honor/Huawei 20/40-byte LIVE footer, HiHonor_/LivePhoto_ TLV, sceneMode, metadata-only trailers | plugins/honor_live_photo provider/proto/tests | implemented synthetic contract; distinguish metadata-only identification from a motion asset; wider projection pending |
| R01 | Inspect and preserve all movie tracks, sample descriptions, timing, metadata samples | MP4 reader/box reader, TrackIsAgtmMetadata, sample APIs | metadata packet API and exact frame/sample timing implemented with declared MIME association; metadata edit/fragmented layouts remain explicit limitations; broad layout audit continues |
| R02 | Copy/import selected tracks across files with reports | MotionPhotoMp4FileBuilder track-copy helpers | general explicit import with unique IDs, exact movie-time mapping, byte verification and movie-metadata sidecars implemented for validated standalone layouts; track-reference layouts remain blocked; Moments composition excluded |
| R03 | Delete metadata/selected tracks with reports | StripMetadataTrack and builder internals | explicit arbitrary track selection/removal implemented for validated standalone layouts; referenced tracks/unknown contracts remain explicitly blocked |
| R04 | MP4 optimization, support query, track summary | OptimizeMp4File, IsOptimizationSupported, GetFileTrackSummary | fast-start/co64 relocation and opt-in compaction implemented with byte checks and reports; fragmentation/references/unknown offset contracts blocked; reference guard rejects hvc1 specifically |
| X01 | Moments video assembly and generated Moments metadata track | AddMomentsMp4FileTrack/AddMomentsMetadataTrack and builder -mv/-mx | **excluded by explicit user instruction**; not a gap or completion gate |
| C01 | Builder command, still/movie inputs, timestamp and output diagnostics | motion_photo_builder target/function/main | maintained llp build command implemented with plans, exact time/targets/pairing/resource policy and safe publication; host CLI tests passed; cross-platform delivery CI pending; Moments excluded |
| C02 | Checker command and exit codes/diagnostics | motion_photo_checker target/function/main | maintained llp check command and JSON diagnostics/exit codes implemented; integration tests passed; broader reference command comparison remains |
| C03 | Extractor command, optional image/video/XMP outputs | motion_photo_extractor target/function/main | maintained llp extract command with optional still/video/XMP and selected resource outputs implemented; exact-byte command tests passed |
| C04 | Metadata command, details/classification, -agtm and -ts | motion_photo_metadata_engine target/function/main | maintained llp metadata/agtm/xmp commands implemented; existing metadata/scoring codecs remain in the core; reference -json flag is parsed but not acted on |
| J01 | Java builder path/memory and callback/error result | lib/java MotionPhotoBuilder, MotionPhotoJni; JNI exports including builderToByteArray | maintained Java/JNI shared operation protocol implemented; actual host JVM build/parse/extract/time/range/cancellation/lifetime tests passed; cross-platform delivery CI pending; Moments excluded |
| J02 | Java checker and extractor with diagnostics | MotionPhotoChecker, MotionPhotoExtractor, MotionPhotoJni | implemented Java/JNI operation interface; host tests run; broader per-method/fixture validation remains |
| J03 | Java metadata path/fd/range, collection classification and plugin controls | MetadataEngine, MetadataEngineJni | implemented through public FileChannel descriptor/range snapshots and request controls; host cursor/ownership test passed; serialized collection API exposed by Rust and request wrappers still expanding |
| J04 | Java AGTM path/bytes/fd/range/explicit timestamp; raw metadata extraction | MetadataEngine and JNI exports | maintained AGTM JNI action implemented on the same core; exhaustive Java per-input AGTM cases remain to run |
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
