# Vendor preservation round5

Base delivery:474b0dadac2351b4ca3be11958c7c83ac7711b3b, whose13source/test/
configuration/fixture roots match verified bd6f85b. Work is isolated on
`codex/vendor-preservation-round5-20261019`; prior results are retained.

## Audited gaps and evidence

| Contract | Existing behavior | Chosen work / limit |
| --- | --- | --- |
| Samsung normal multi-video asset | Already exposes MotionPhoto_Data as primary and complete extra records such as MotionPhoto_AutoPlay as UnknownVideo, with own container/tracks. Generic resources/extract already allow explicit ResourceId selection. | Retain, do not reimplement or infer a Gallery autoplay/substitution/depth role. |
| Samsung inspection without a unique primary | `inspect_samsung_metadata` rejects duplicate primary declarations before exposing a usable complete-video catalogue. A metadata-only SEF with a valid other video cannot be parsed as a full MotionPhoto. | Add a read-only catalogue and explicit record-index selection that work without choosing a default primary. Preserve ordinary parser default rejection of ambiguity. Reuse the existing SEF wire parser, keep legacy public fields unchanged. |
| Samsung per-stream preservation | Existing normal-asset extraction and source copying exist; limited direct-JPEG writer retains raw nonprimary records. | Verify every original stream hash and full record payload; add positive, missing/duplicate-primary, wrong-index, malformed/bounds/overlap and unknown-record vectors. No new writer/time mapping. |
| vivo UUID/framing | Already bounds top-level UUIDs, directory markers/counts, duplicate keys, lengths and footer containment. Streams remain opaque. | Retain and use existing source mapping; do not redo the framing decoder. |
| vivo cross-declaration audit | Footer internal ID agreement exists. Metadata projections expose global footer and nested UUID footer separately, without a unified physical-origin/alias/conflict report or comparison of standard XMP's stated media span with the selected resource. | Add bounded source-scoped observations and explicit declaration conflicts. Deduplicate aliases of the same physical footer, compare like identifier fields, and compare XMP media spans only where the standard appended-file layout supplies that mapping. Keep media validity, declaration agreement and native pairing separate. |
| Unproved semantics | vivo imageTime unit/stream roles; Oplus checksum/second-stream role; Honor legacy/native variants lack proof. | No guesses, native writer or new native-experience claim. |

Fresh baseline observations use the existing legal SHA-locked public cache:
Samsung6files represent4distinct hashes (two names are aliases), including two
independent JPEGs and two HEIFs. All four have two separately located complete
BMFF payloads: primary MotionPhoto_Data and MotionPhoto_AutoPlay-labelled record.
The two independent vivo originals retain31opaque records each and matching
global/nested footer views. Raw media, identifiers and researched implementation
bodies remain private; only range/hash/factual summaries may be committed.

## Implementation invariants

* Catalogue selection is explicit by a validated SEF record index. A name is a
  wire label, not a playback role. Missing or ambiguous primary declarations
  cannot silently fall back to AutoPlay or the first recognisable stream.
* Returned media comes from a borrowed immutable source. Validate all source
  extents and record overlap before exposing results; bound aggregate work and
  report invalid/unsupported candidates without losing opaque raw records.
* No encoded stream rewriting, timestamp guessing or unknown-field mutation.
  Existing plan/execute/default-reject/sidecar policies remain authoritative.
* vivo comparisons concern matching declaration fields and supported source
  layouts only. Unknown imageTime units are not compared with XMP seconds.
  Equal IDs are not unique capture-pair proof. Distinct physical declarations
  remain distinct even when values match; aliases are reported as aliases.
* A valid core container can carry conflicting proprietary declarations. The
  audit reports that difference rather than silently choosing a winner or
  recasting parse success as Gallery/Photos compatibility.

## Public-source acquisition policy

Record app role, exact declared package/version, official/reliable public page,
retrieved representation/bytes/hash and access result separately. An extracted
web page is not an APK/native library. Prior CONNECT403 targets are not retried
or bypassed. New public transfers are bounded and ordinary; no unknown binary
is installed/executed, network configuration changed or private service used.
If no new package is obtainable, independent originals and already lawful source
facts still support the chosen software contracts, with missing package identity
and original-device/native acceptance explicitly retained as hard evidence gaps.

Old exhaustive AacConfiguration literals still require the two observed fields;
the round4 migration evidence remains valid. No XDRemux repository/pin changes.
No merge/release/force push/upstream communication or new Codex task/model/tier
change is authorised by this implementation step.

## Implemented contract / verification map

| Item | Status / API | Evidence / boundary |
| --- | --- | --- |
| Existing Samsung normal multi-stream graph | Existing, regression verified | `resources`, `extract`, existing `samsung.rs` round trip; extra video remains UnknownVideo with its wire label. |
| Missing/duplicate-primary catalogue | Added, local verified | `inspect_samsung_videos(&[u8])`, `SamsungVideoInventory::select(Some(record_index))`; every complete video has an explicit index and source extent. `select(None)` needs exactly one declared, valid primary. Default MotionPhoto parser still rejects ambiguity; duplicate primary now returns AmbiguousRecovery. |
| Per-record original bytes and unsupported candidates | Added, local verified | `record_payload`, `raw_record`, borrowed `SamsungVideo.bytes`, per-record diagnostics. No partial directory accepted. Input256MiB,4096 records,64 distinct candidate ranges and64MiB unique movie bytes; aliases share validation but retain distinct records. Structurally supported BMFF/video track required, no codec decode. |
| Samsung input/output distinction | Explicit | New Rust catalogue accepts memory only; callers read their own bounded snapshot with caller-controlled I/O. MotionPhoto open/file/fd parsing still fails on an ambiguous asset. Existing native metadata's memory/path/fd-range snapshot route can expose the catalogue projection. The inspector does not open/write or repair a file. Existing plan/execute may build from explicitly selected bytes with an explicit caller time, under existing unmapped-resource rules. No new native writer or implicit time. |
| vivo physical origin / source ranges | Added, local verified | `MotionPhoto::vivo_declaration_report`; terminal and UUID observations map footer/JSON/binary-ID ranges to `source_bytes(source_index)`. Same physical source+extent merges origins; distinct physical copies remain distinct. Shared range mapper now checks overflow/reversed extents. |
| vivo declaration conflicts | Added, local verified | Internal JSON/envelope and cross-footer like-field IDs, plus directory and MicroVideo EOF-relative spans for a single appended JPEG. Explicit Matched/Mismatch/OutOfRange/InvalidDeclaration/NotComparable states. Parts and HEIF spans are not mapped as appended JPEGs. No XMP ID or time-unit equivalence is invented. |
| Metadata binding exposure | Added, local verified | Existing metadata operation gains `samsung_video_inventory` and `vivo_media.value.declarations`. Inventory has motion_declaration=false and no verified-video extents. Group controlled by samsung_sef or its explicit inventory provider; vivo/camera_album controls stay effective. Core verifiedMotion means valid media, never vendor ID pairing. |
| Original corpus | Local verified | Existing14-file hash-locked corpus; fresh assertions on Samsung6filenames/4distinct SHA values,2complete movies each, and vivo2originals/31opaque streams each. Two physical-origin views are one footer; both original XMP spans match. This is not new device/firmware acceptance. |
| New malformed/selection contracts | Local verified | Samsung11tests (6new), vivo consistency7new, existing vivo6; shared source-mapping overflow1new. Covers missing/duplicate declarations, explicit invalid selection, bad ranges/pointers/overlaps, alias-vs-distinct budgets, internal/cross-footer conflict, stale XMP, Parts, nested magic and provider filters. |
| Actual binding transports | Verified on exact0ece07a | New13 authored cases with SHA-locked owned neutral fixtures passed local and all3-host C ABI/JNI/release CLI/compiled C/external JAR consumers. Source0ece07a's latest3workflow/8logical-job attempts passed; Android required one unchanged-source rerun after a pre-test installation failure. Mobile/WASM existing suites remain separate and do not count these13 as executed. See HANDOFF_ROUND5_20261019.md and platform-0ece07a.json for exact results and first-attempt uncertainty. |
| APK/native evidence | Blocked | [Acquisition ledger](vendor-round5-acquisition.json):8exact package/version listings,13hashed returned-text bundles;10ordinary HTTP requests failed CONNECT403 before a body. APK body bytes0; verified package hashnull. Listing-advertised hashes are not measured binary hashes. Prior blocked targets were not retried. |
| Oplus/Xiaomi/vivo/Honor native semantics | Partial / missing / blocked | No new writer or role/time/checksum inference. Honor Camera/Gallery and OnePlus Camera package/version are unresolved by bounded sources. Original Honor corpus, authenticated firmware/component mapping and physical Gallery/Photos acceptance still absent. |

The pinned Google [Samsung implementation](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/samsung_sef/samsung_sef_provider.cc),
[proto](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/samsung_sef/samsung_sef_metadata.proto)
and [tests](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/samsung_sef/samsung_sef_provider_test.cc)
remain the Google coverage baseline. The new explicit catalogue is an independent
preservation/selection contract beyond that provider projection. vivo is outside
the pinned provider set: its boundaries rely on existing original bytes and the
previously recorded static facts, not a fabricated Google vivo API.

Selected page facts are source-specific, not universal package identities:
[Samsung Gallery15.9.00.45](https://www.apkmirror.com/apk/samsung-electronics-co-ltd/samsung-gallery/samsung-gallery-15-9-00-45-release/samsung-gallery-15-9-00-45-android-apk-download/) declares com.sec.android.gallery3d;
[Samsung Camera17.0.01.8](https://www.apkmirror.com/apk/samsung-electronics-co-ltd/samsung-camera/samsung-camera-17-0-01-8-release/samsung-camera-17-0-01-8-android-apk-download/) com.sec.android.app.camera;
[vivo Albums3.2.10.5](https://www.apkmirror.com/apk/vivo-co-ltd/vivo-gallery/vivo-albums-3-2-10-5-release/vivo-albums-3-2-10-5-android-apk-download/) com.vivo.gallery;
[vivo Camera13.0.50.32](https://www.apkmirror.com/apk/vivoglobal/camera-25/vivo-camera-13-0-50-32-release/vivo-camera-13-0-50-32-android-apk-download/) com.android.camera;
[Xiaomi Gallery4.3.1.15-global](https://www.apkmirror.com/apk/xiaomi-inc/miui-gallery/xiaomi-gallery-4-3-1-15-global-release/xiaomi-gallery-4-3-1-15-global-android-apk-download/) com.miui.gallery;
[Xiaomi Camera6.5.000280.1](https://www.apkmirror.com/apk/xiaomi-inc/camera-xiaomi/xiaomi-camera-6-5-000280-1-release/xiaomi-camera-6-5-000280-1-android-apk-download/) com.android.camera;
[OPPO Photos16.35.10](https://www.apkmirror.com/apk/coloros/oppo-photos/oppo-photos-16-35-10-release/oppo-photos-16-35-10-android-apk-download/) com.coloros.gallery3d;
[OPPO Camera3.303.36](https://www.apkmirror.com/apk/oppo/oppo-camera/oppo-camera-3-303-36-release/oppo-camera-3-303-36-android-apk-download/) com.oppo.camera.
[Official OPPO Play listing](https://play.google.com/store/apps/details?hl=en_US&id=com.coloros.gallery3d)
separately confirms that package, without exposing a versioned APK. Namespaces,
permissions, wire version2202 and a listing date cannot identify the component
which authored the existing originals. No new dependency/license obligation is
introduced; proprietary text/media bodies remain outside Git.
