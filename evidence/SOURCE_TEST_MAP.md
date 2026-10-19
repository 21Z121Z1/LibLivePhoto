# Pinned source, public API and test map

This index makes the inventory rows reviewable. Authority is the pinned source,
not README samples. Status and actual execution SHA remain in the
[complete inventory](GOOGLE_COMPLETE_INVENTORY.md),
[acceptance checklist](ACCEPTANCE_CHECKLIST.md), stage receipts and CI.
The separate [normative map](NORMATIVE_CONFORMANCE.md) distinguishes format
requirements from pinned-source parity and Apple system acceptance.
The [76c25e4 execution receipt](standards-edit-76c25e4.json) covers the added
standards/edit regressions. [Timed metadata differential](../scripts/timed_metadata_differential.py)
runs20 same-input selections against the exact
[Google reader primitive](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo_mp4/mp4_box_reader.cc#L218).
Expected ctts/edit/after-end differences are asserted; Google's output is not
used as an unconditional correctness oracle.

## Per-operation input/output distinctions

The pinned public header is at the repository root, `libmotionphoto_api.h`.

* Parse supports paths, memory and fd/range and returns metadata.
* File check invokes the checker. Fd/memory check reparses and returns the
  `is_motion_photo` flag; it does not invoke that full file checker. Our shared
  check validates supported resources and known score semantics for each input.
* Image/video extraction accepts each input form but writes output paths. The
  combined file extractor passes `metadata_output_path`; combined fd/memory
  extractors only call the requested image/video paths. Those implementations
  do not establish universal fd/memory XMP output. LibLivePhoto exposes raw XMP
  on the parsed source and reports absence separately.
* The AGTM facade parses a motion photo and sends JSON through its callback.
  Standalone video plus explicit timestamp is a MetadataEngine capability.
  Our standalone video API requires exact time and actual timed metadata tables.
* Public facade builders accept still/movie paths, memory, or fd/ranges and
  write `output_path`. Google fd build snapshots with lseek/read. Our snapshots
  preserve caller descriptor ownership/cursor under the documented platform
  conditions; plan/execute returns owned files, and extract accepts a caller sink.
  CLI publication and arbitrary caller-provided Write sinks have separate
  rollback/atomicity contracts. No random-access output-range writer is claimed.

| Inventory IDs | Pinned Google authority | LibLivePhoto entry point | Concrete regression/differential | Boundary |
| --- | --- | --- | --- | --- |
| N01 / N02 | [public parse/check declarations](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/libmotionphoto_api.h#L89); [operation-specific check implementation](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/libmotionphoto_api.cc#L574) | `MotionPhoto::parse/open/open_file/open_fd`, `validate`, shared `check` | [core_contract](../crates/liblivephoto/tests/core_contract.rs); [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs); shared protocol tests | Verified supported structures and ranges; decoder/spec conformance and native Gallery are separate. |
| N03 | [extract declarations](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/libmotionphoto_api.h#L139); [file vs fd/memory combined extractor](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/libmotionphoto_api.cc#L712) | `extract`, `still_bytes`, `motion_video_bytes`, `xmp_bytes` | [reader](../crates/liblivephoto/tests/reader.rs); [conversion](../crates/liblivephoto/tests/conversion.rs); [core_contract](../crates/liblivephoto/tests/core_contract.rs) | Original video retained; no implicit metadata removal/optimization. Absence of XMP is explicit. |
| N04 | [AGTM facade](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/libmotionphoto_api.h#L194); [timed metadata engine](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo/metadata_engine.h#L53); [AGTM decoder](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/agtm.cc) | `agtm_at`, `extract_agtm_from_video`, `decode_agtm`, exact `MediaTime` | [metadata_contract](../crates/liblivephoto/tests/metadata_contract.rs); [agtm_complete](../crates/liblivephoto/tests/agtm_complete.rs); [timed_metadata_edits](../crates/liblivephoto/tests/timed_metadata_edits.rs); `agtm_differential.py` | Version-0 field decode/raw extraction; unit-rate empty/trim/repeat edits map exactly. Raw packet media times stay separate. Non-unit edits/nonintegral conversion/fragmentation/unknown versions and budget excess fail; no pixel rendering. |
| N05 / P01 | [build declarations](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/libmotionphoto_api.h#L215); [build facade implementation](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/libmotionphoto_api.cc#L819); [builder implementation](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo_builder/function.cc) | `plan` / `execute`, `FileInput::Parts`, Google JPEG/HEIF/AVIF profiles | [conversion](../crates/liblivephoto/tests/conversion.rs); [core_contract](../crates/liblivephoto/tests/core_contract.rs); [media_contract](../crates/liblivephoto/tests/media_contract.rs) | Lossless original coded items/packets; differs intentionally from Google HEIF decode/RGB/re-encode. No automatic transcode or overwrite. |
| N06 | [version declaration](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/libmotionphoto_api.h#L242) | core/shared `version`, `llp version`, `llp_api_version`, Java engine version | CLI command tests; actual C ABI/JVM and extracted SDK consumers | Library/API versions identify this maintained interface, not C++ or Google Java ABI compatibility. |
| M01 / M02 | [parser](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/motion_photo_parser.cc); [serializer](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/motion_photo_serializer.cc); [metadata engine](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo/metadata_engine.h); [collection schema](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/proto/metadata_collection.proto) | `XmpDocument`, `MetadataEngine`, `MetadataCollection`, `extract_block` | [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs); [core_contract](../crates/liblivephoto/tests/core_contract.rs); 38-case `metadata_differential.py` | Collections include ordinary images. Declaration/serialized evidence never establishes verified current media; raw manifest prefixes differ from complete block extraction. |
| M03 | [provider registry](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo/metadata_provider_registry.h); [registry implementation](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/metadata_provider_registry.cc) | `MetadataProvider`, `register`, `HandlerOptions`, `parse_with_handlers` | [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs); canonical/alias/enable-none differential controls | Custom declarations do not create native facade composition support. |
| M04 | [video metadata](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo/video_metadata.h); [decoder](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/video_metadata_decoder.cc) | `metadata_packets`, `decode_video_metadata`, `scoring_metadata`, semantic validation | [video_metadata](../crates/liblivephoto/tests/video_metadata.rs); nine-vector `video_metadata_differential.py` | Existing scores/flags only. Unknown bytes retained; aggregate limits explicit; Moments construction excluded. |
| P02 | [MicroVideo provider](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/legacy_micro_video/legacy_micro_video_provider.cc); [provider tests](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/legacy_micro_video/legacy_micro_video_provider_test.cc) | Legacy JPEG parsing/filtering plus metadata/raw extraction | [reader](../crates/liblivephoto/tests/reader.rs); [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs); provider differential | Canonical and namespace-alias behavior compared; no universal legacy-writer or Gallery claim. |
| P03 | [Samsung provider](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/samsung_sef/samsung_sef_provider.cc); [provider tests](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/samsung_sef/samsung_sef_provider_test.cc) | `inspect_samsung_metadata`, SEF/mpv2/mpv3/HEIF `sefd` reader; limited direct JPEG writer | [samsung](../crates/liblivephoto/tests/samsung.rs); pinned original fixtures and provider differential | Tested raw/pointer/inline/stub layouts; native Gallery and wider original-device corpus unverified. |
| P04 | [Oplus provider](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/oplus_olive/oplus_olive_provider.cc); [provider tests](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/oplus_olive/oplus_olive_provider_test.cc) | `inspect_oplus_metadata`, OpCamera XMP/JXRS; retained ColorOS LPEX reader | [oplus_olive](../crates/liblivephoto/tests/oplus_olive.rs); [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs); original XDRemux contract replay | HEIC declarations distinguished from verified video; checksum bytes not called verified; unknown streams not called depth; no general native writer. |
| P05 | [Honor provider](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/honor_live_photo/honor_live_photo_provider.cc); [provider tests](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/honor_live_photo/honor_live_photo_provider_test.cc) | `inspect_honor_metadata`, LIVE 20/40 and backward TLV | [metadata_contract](../crates/liblivephoto/tests/metadata_contract.rs); provider differential including malformed OfflineData | Synthetic supported variants verified. Legal redistributable original Honor corpus/native Gallery acceptance blocked on external input. |
| R01–R04 | [MP4 builder public class](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo_mp4/motion_photo_mp4_file_builder.h#L52); [track copy internal use](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo_mp4/motion_photo_mp4_file_builder.cc#L211); [optimization support/deletion](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo_mp4/motion_photo_mp4_file_builder.cc#L319) | `track_summary`, `import_tracks`, `remove_tracks`, `optimize`, `optimization_support`, packet/timing APIs | [remux_complete](../crates/liblivephoto/tests/remux_complete.rs); [frame_timing](../crates/liblivephoto/tests/frame_timing.rs); [metadata_contract](../crates/liblivephoto/tests/metadata_contract.rs); six-case `mp4_differential.py` | Class helpers are public and internally used by builder/Moments; no generic copy facade is inferred. Rust explicit general import is bounded. Google hvc1 guard/copy time shift differ; unknown/reference/fragmented contracts rejected. |
| C01–C04 / J01–J04 / A01–A04 | [CLI functions](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo_builder/function.h); [Java sources](https://github.com/google/libmotionphoto/tree/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/java); [Android service/AIDL sources](https://github.com/google/libmotionphoto/tree/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/android/src/main) | `llp`, maintained shared JSON/LLP1, C ABI, Java/JNI, four FD services | [CLI tests](../crates/liblivephoto-cli/tests/commands.rs); [shared protocol](../crates/liblivephoto-interface/tests/protocol.rs); [JVM smoke](../java/src/test/java/org/liblivephoto/NativeSmoke.java); [installed service scopes](../android/src/test/java/org/liblivephoto/android/ServiceAcceptance.java) | Host package/JNI proof and installed Android runtime are separate. Original Google Java/JNI ABI/protobuf compatibility is not promised. |

## Stability and native acceptance

H05's optional `validate_heif_color_information` checks the user-provided
ISO23008-12:2025 §6.5.5 count/pair/paired-nclx declarations. The five new
`standards_contract` regressions bind per-item/multiple-ipma behavior, legal
single declarations, both pair orders, normative violations versus unsupported
facts, preservation and cumulative budgets. Whole ICC/BMFF conformance and
rendering are separate; the original standard is not redistributed.

See [STABILITY_GATES.md](STABILITY_GATES.md) for public-range preflight, JSON and
metadata amplification, cross-track frame budgets, FFI ownership, cancellation
and failure rollback. Exact frame times, media packet hashes, original auxiliary
bytes and independent decoder facts are tested; AGTM extraction is not rendering.

Apple public-framework and browser adapters have their own actual operation
tests. Emulator/Simulator or Chrome success does not verify a permissioned
Photos library, physical HDR/gesture/capture, vendor Gallery behavior, or the
unavailable LivePhotosKit runtime. Moments construction remains the explicit
feature exclusion. No board is called 100% while a required acceptance row is open.
