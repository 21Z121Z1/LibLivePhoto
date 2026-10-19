# Google core coverage

This is a behavior comparison. It is not C++ ABI compatibility or device acceptance.
The reference is Google libmotionphoto commit
`12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3` ([source tree][google]).
The implementation starts at LibLivePhoto PR #1 head
`794ab552596db0f66095600d23cf7937a631b9ec`. The head was checked with
`git ls-remote` and the GitHub PR API on 2026-10-19. No newer changes were present.
XDRemux PR #58 head `0688ea7d6270df52cc5968e2c36dd016f13bd31b` is a read-only
consumer reference. Both of its library dependencies pin the starting library SHA.

Status terms: **existing** means source implementation exists; **partial** means
the stated subset exists; **missing** means there is no supported contract;
**verified** requires an executed test named in the evidence; **blocked** identifies
a concrete prerequisite. Implementation does not establish native Gallery support.

Current scope is defined by [the complete inventory](GOOGLE_COMPLETE_INVENTORY.md).
Java/JNI, CLI and Android wrappers are now in scope. **Moments construction is
excluded by explicit user instruction**, not a gap or completion gate. Earlier
initial-assessment rows below are historical.

## Delivered result

This phase is **partial coverage of the complete Google core**, with executed
evidence for the implemented subsets below. The initial tables that follow retain
the pre-change assessment; they are not a claim that delivered features are still
missing. Test paths are relative to `crates/liblivephoto/tests`, unless specified.
See [integration](../CORE_INTEGRATION.md) for input/output distinctions and limits.
The [run receipt](GOOGLE_CORE_RUN.md) binds results and remaining gaps to the
validated implementation SHA and exact commands.

| Capability | Delivered status | LibLivePhoto API and executed evidence | Remaining boundary |
| --- | --- | --- | --- |
| File/memory input | existing, verified | `parse`, `open`; existing reader/conversion/external consumer tests | bounded snapshots, not random-access streaming |
| fd + range input | added, verified on Linux | `open_fd`, `open_file(FileInput)`; `core_contract.rs` single/parts/Apple-pair range, cursor, overflow, truncation/budget contracts | Windows cursor restoration requires caller synchronization; macOS/Windows execution not run |
| Input/output per operation | verified subset | All inputs produce the same parsed resource graph; `extract(Write)` and in-memory `execute()`; path/range-parts builds compare equal | no direct streaming sink builder, no bounded output-range overwrite; Google fd extraction was source-reviewed, not run by the differential adapter |
| JPEG / HEIC | existing, verified | `AndroidJpeg`, `AndroidHeif`; 14 pinned originals, conversion tests, 3 Google same-input fixtures | original Gallery/Photos use unverified |
| AVIF | added, verified | `AndroidAvif`, AVIF brands/items/XMP; `core_contract.rs`; generated decodable AVIF → build → Google file/memory/range reparse → independent FFmpeg still decode | one generated encoder profile; no device-captured AVIF corpus; Apple AVIF output stays blocked |
| XMP and directory | strengthened, verified | `xmp_bytes()`, namespace-qualified attribute/element numbers, conflicting/overflowing declarations, MIME diagnostics; core contracts and reference comparison | not a full generic RDF/XMP semantic engine; provider protobuf projection remains partial |
| XMP writer interoperability | repaired, verified | active description precedes retained descriptions; all 9 rebuilt differential cases are recognized by the pinned reference | original unknown XMP attributes remain retained; no native Gallery claim |
| Bounds / padding / resources | existing + strengthened, verified | `validate`, checked range snapshots, 8/16-byte `mpvd` padding unit vectors, property-allocation bounds; malformed tests and ASan campaigns | structural validation is not complete Google-spec or decoder validation |
| Exact photo time | existing + Honor subset, verified | `MediaTime`, XMP time, caller time, supported Apple time and Honor cover milliseconds; unchanged 1/3-second rejection; encoded packet timestamps checked | no rounding, no unknown-time inference; AGTM metadata edit lists unsupported |
| Static image / video extraction | existing, verified | `still_bytes`, `motion_video_bytes`, `extract`; original resource bytes, 9 differential inputs, AV packet hashes and rational packet times | retains metadata tracks by default; Google optional extraction side effects differ |
| AGTM extraction | added, verified subset | `agtm_at`, `decode_agtm_header`; `metadata_contract.rs` intervals/ctts/truncation; original Google AGTM baseline comparison; Android and existing Apple route retain the sample bytes | complete Google version-0 field decoder added; all rules/slopes/custom chromaticities and encoded mix values retained; 12 reference vectors verified; no pixel rendering; unsupported edit lists fail |
| Still orientation / color | added, verified subset | `still_metadata`, `OrthogonalTransform`, `ColorDescription`; EXIF 0/1–8, ICC, ordered HEIF irot/imir/CICP/PQ and unknown properties; measured preservation evidence | crop composition, ICC interpretation, full gain-map/HDR semantics remain partial |
| Movie transform / codec / AAC | added, verified subset | `sample_descriptions`, `MediaCapabilities`, `plan_with_capabilities`; `media_contract.rs` unsupported entries, channels/rates, unknown PCE and malformed ASC | default plan is structural; VUI/SEI, QT v2/wave and full AAC/decoder conformance remain partial; no device profile inferred |
| MicroVideo | existing, verified synthetic contract | `android.rs` tests, legacy format/dialect and exact time | no new native legacy writer; not every legacy variant |
| Samsung SEF | existing, verified | direct/pointer/BMFF readers; `samsung.rs`, pinned originals; bounded direct JPEG writer | no larger Samsung writer claim; AutoPlay/time mappings remain limited |
| Oplus Olive | existing partial, verified existing LPEX corpus | current ColorOS 15/16 reader and original dual-stream resource tests; unchanged XDRemux public contract replay | pure OpCamera XMP, JXRS checksum/JSON directory variants remain missing or unverified; no native writer; stream 2 role unresolved |
| Honor | added, verified synthetic contract | `inspect_honor_metadata`; `LIVE_` 20/40-byte and backward TLV; sceneMode; metadata-only rejection; 3 reference comparisons | no original corpus or native writer; ambiguous video candidates fail; cover overflow fails; no broader Huawei claim |
| MP4 track preservation | existing, verified | whole movie copy on Android, original tracks/media retained on Apple; unchanged resource tests and AGTM regression | arbitrary cross-file track import missing |
| Metadata-track deletion | added, verified bounded subset | `remux::remove_metadata_tracks`; explicit IDs, raw media retention, removal/no-op/invalid selection tests | no implicit deletion; no fragmentation/index/reference rewriting; unreferenced bytes remain in mdat |
| Google Moments `mett` descriptors | partial byte preservation | existing movie/resource retention; Google fixture contains metadata tracks that remain in the exact extracted/built movie | score/stabilization/frame descriptor semantics remain pending; Moments construction explicitly excluded by user |
| MP4 optimization | missing, explicitly scoped | append-only, offset-stable rewriting does not invoke mp4v2 | fast-start/compaction absent; Google optimization rejects `hvc1` specifically; byte preservation does not need this restriction |
| Java/JNI, Android services, complete CLI | later work | local Google JNI target happened to compile as a reference prerequisite; no LibLivePhoto delivery claimed | not part of core coverage completion |
| Native Gallery / Photos | blocked on external evidence | no on-device acceptance run | device/corpus/version-specific recognition, editing and export remain unverified |

### Pinned reference findings

`scripts/google_differential.py` compares the public Google facade from an
unchanged checkout. It uses file, memory and wrapped-fd parse inputs. It separately
runs file and memory extraction, compares encoded A/V packet SHA-256 values and
exact rational PTS/DTS/durations, and checks recognition/length/time after rebuild.
The receipt records source hashes and all field differences. It does not treat
Google as a universal correctness oracle.

Observed differences at the pinned SHA:

- The reference facade reports `video_mime = image/jpeg` for the AGTM/no-AGTM
  JPEG fixtures. Their valid directory has a JPEG gain-map at index 1 and MP4 at
  index 2; `PopulateNativeMetadata` reads `items[1].mime` (`libmotionphoto_api.cc:98`).
  LibLivePhoto selects the MotionPhoto item and reports MP4. Video lengths and
  known times agree.
- Reference memory extraction computes video start from file end minus video
  length. On the three tagged Honor trailers this includes footer bytes and
  produces a video that FFprobe rejects. Its file extractor returns the correct
  A/V packets. LibLivePhoto uses explicit provider extents for all input forms.
  The fd extraction implementation has the same formula; that is a source-based
  observation, not an executed fd-extraction result.
- The Honor provider decodes the 40-byte footer cover time, but the public facade
  returns unknown time for that synthetic input. LibLivePhoto retains the declared
  450 ms. The rebuilt Google directory exposes 450000 us to the reference.
- Reference extraction removes metadata tracks and may optimize the movie.
  LibLivePhoto extraction retains the original movie bytes, including AGTM.
  A/V packet bytes and exact packet times still agree where reference output is
  valid. Explicit removal is a separate opt-in operation.
- The reference returns a default baseline for some missing/truncated AGTM paths
  and scans arbitrary memory for T.35. LibLivePhoto requires a timed, signaled
  sample and fails or returns absence instead. Truncation tests cover this choice.

### Exact source map

All links below use commit `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`.

- [Public facade declarations](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/libmotionphoto_api.h)
  and [wrapper implementations](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/libmotionphoto_api.cc).
- [Parser](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/motion_photo_parser.cc),
  [checker](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/motion_photo_checker.cc),
  and [HEIF splitter](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/heif_container_splitter.cc).
- [AGTM selection](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/metadata_engine.cc)
  and [payload decoder](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo/agtm.cc).
- [MicroVideo](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/legacy_micro_video/legacy_micro_video_provider.cc),
  [SEF](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/samsung_sef/samsung_sef_provider.cc),
  [Oplus](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/oplus_olive/oplus_olive_provider.cc),
  and [Honor](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/plugins/honor_live_photo/honor_live_photo_provider.cc).
- [MP4 builder helpers](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/includes/motion_photo_mp4/motion_photo_mp4_file_builder.h)
  and [implementation, including hvc1 guard](https://github.com/google/libmotionphoto/blob/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3/lib/core/motion_photo/src/motion_photo_mp4/motion_photo_mp4_file_builder.cc).

## Input and output contracts

Google positions below are relative to its pinned repository. `libmotionphoto_api.h`
is the public facade; its implementation is
`lib/core/motion_photo/src/motion_photo/libmotionphoto_api.cc`. README examples are
not the API authority. Header declarations alone do not establish implementation.

| Operation | Google source and actual scope | Starting LibLivePhoto contract | Initial status / required work |
| --- | --- | --- | --- |
| Parse / check | `libmotionphoto_api.h:89-131`: file, memory, fd + offset + length; check file and memory paths differ internally | `MotionPhoto::parse(Input, ParseOptions)`, `open(PathInput, ...)`, `validate()` | existing memory/path; missing fd range |
| Extract image / video | header `:139-171`, implementation `:570-709`; each input form writes a named output file; video extraction invokes `StripMetadataTrack` | `extract(&Resource, impl Write)`, `still_bytes()`, `motion_video_bytes()` | existing exact resource copy to memory/file/any Rust `Write`; fd input missing; no implicit track deletion |
| Combined extract | header `:176-188`, implementation `:712-749`; file uses full extractor, fd/memory wrappers only process image/video paths | same resources and caller-controlled sinks | partial; do not generalize metadata file output to all Google wrappers |
| XMP / metadata | `motion_photo_extractor/function.cc`, `MetadataEngine::Parse` and `ExtractMetadata` | JPEG/HEIF XMP resources and `extract` | partial; add explicit packet access; protobuf collection schema is not an ABI goal |
| AGTM | header `:194-208`, `MetadataEngine::ExtractAgtmAtTimestamp*`, `agtm.cc` | no explicit AGTM API | missing; extraction is metadata, not tone mapping or pixel rendering |
| Build | header `:215-236`: file, fd ranges, memory, all facade outputs are file paths; `motion_photo_builder/function.h` also has `BuildMotionPhotoDirect(MediaSource, DataSink)` | `Input::Parts`, `plan(...).execute()` -> in-memory `ComposedAsset` | existing memory build; missing path-parts and fd-parts convenience; output publication stays caller-owned |

## Formats, validation and construction

| Capability | Google source / tests | LibLivePhoto API / starting tests | Initial status / limits |
| --- | --- | --- | --- |
| JPEG directory | `motion_photo_parser.cc`, `jpeg_container_splitter.cc`; `motion_photo_checker_test.cc`, `motion_photo_builder_test.cc` | `parse`, `plan(AndroidJpeg, ...)`; `reader.rs`, `conversion.rs` | existing; encoded JPEG data and opaque auxiliary resources already have preservation contracts |
| HEIC `mpvd` | `heif_container_splitter.cc`, `mpvd_box.cc`, HEIF info/image builders; `heif_container_splitter_test.cc` | `parse`, `plan(AndroidHeif, ...)`, `heif_io`; `conversion.rs` | existing append-only item/XMP rewrite, no decode; retain this path |
| AVIF | `FileType::kAvif`, HEIF splitter/builders | `ContainerKind::Avif` exists, but facade still brands and target reject AVIF | missing end-to-end parse/build contract |
| Namespace / XMP / directory | `camera_metadata_reader.cc`, `motion_photo_parser.cc`, `google_motion_photo_provider.cc` | bounded namespace-aware `android.rs`, generic metadata resources | existing; keep unknown properties; audit malformed numeric values and inconsistent declared MIME |
| Video extents / padding | `MotionPhoto::GetMp4FileOffset`, `MotionPhotoChecker::CheckContainerMetadataAndSizes`, `CheckMpvdBox` | `ByteRange`, `validate`, `android.rs`, `heif.rs`; boundary tests | existing bounds and overflow checks; test gaps and extended headers explicitly |
| Presentation time | `camera_metadata.h`, `video_metadata_decoder.cc`, checker consistency rules | `MediaTime::rescale_exact`, presentation provenance, timeline checks | existing exact time; preserve 1/3-second rejection and out-of-header diagnostics |
| Camera / scoring / stabilization metadata | camera/video metadata headers and ISO descriptors; metadata engine provider protos | XMP/metadata bytes retained; pairing/time interpreted | partial; scores, stabilization descriptors and provider protobuf projection are not interpreted yet |
| Errors / validation | `MotionPhotoValidator`, `MotionPhotoChecker`, facade callbacks | typed read errors, `ValidationReport`, `Diagnostic`, immutable plan blockers | existing structural validation; not full Google specification conformance; codec/audio conformance missing |
| Orientation / transform | Google HEIF decode/encode builder, underlying container metadata | raw EXIF/items/track bytes retained | partial; unified facts missing; byte retention must not imply interpreted display semantics |
| Color / HDR | Google AGTM path and image builder | opaque ICC, EXIF, HEIF items and encoded movie bytes retained | partial; explicit color/HDR facts and preservation checks missing; no pixel conversion |
| Resource preservation | JPEG builder rewrites XMP/MPF; HEIF builder decodes RGB then quality-100 encodes | `WritePolicy`, `UnmappedPolicy`, per-resource reports, sidecar equality | existing; lossless container rewrite is an intentional implementation difference from Google |

## Format providers

These are concrete byte contracts, not claims about a vendor's complete camera,
gallery, editing or export behavior. Google provider sources and their adjacent
`*_provider_test.cc` files are under `lib/core/plugins`.

| Provider | Reference contract | Starting implementation / tests | Initial status / limits |
| --- | --- | --- | --- |
| MicroVideo | `legacy_micro_video`: XMP enabled flag, tail offset, presentation time | `android.rs`, `AssetFormat::LegacyMicroVideo`; synthetic parser tests | existing JPEG tail contract; not every legacy file variant |
| Samsung SEF | `samsung_sef`: recognized SEF blocks and motion offsets, including pointer records | `vendor/samsung.rs`; `samsung.rs` / original fixture gates | existing direct JPEG and bounded pointer/BMFF reader; writer only bounded direct JPEG SEF; AutoPlay unresolved |
| Oplus Olive | `oplus_olive`: XMP plus appended/video trailer variants, LPEX/JXRS paths | `vendor/oplus`, resource/track graph; pinned originals | partial overlapping formats; Olive3/JXRS not established as complete; stream 2 stays unresolved; native writer missing |
| Honor | `honor_live_photo`: 20/40-byte `LIVE_` footer and backward TLV chain, offline sceneMode | no explicit provider; recovery may find video | missing format-specific parse/time contract; no original corpus; synthetic support must be labeled |

## MP4 scope

| Capability | Reference source / actual scope | Starting LibLivePhoto | Initial status / limits |
| --- | --- | --- | --- |
| Preserve primary tracks | `MotionPhotoMp4FileBuilder::AddPrimaryMp4FileTracks` copies input file then deletes `meta` tracks | Android copies movie bytes; Apple appends pairing/time metadata while retaining original media | existing byte/sample preservation; defaults intentionally retain metadata |
| Additional tracks | `AddMomentsMp4FileTrack` copies tracks via mp4v2; internal builder header is exported in the source tree, not a top-level facade operation | multiple tracks are modeled/preserved | partial; arbitrary cross-file track import missing |
| Metadata track | `AddMomentsMetadataTrack` creates `mett` with Google binary descriptors; reader and checker inspect it | existing Apple timed `mebx`; generic metadata tracks modeled | partial; Google moments descriptor construction/interpretation missing |
| Delete metadata tracks | `StripMetadataTrack`, called from public video extraction; it mutates the named file and may optimize | no explicit deletion operation | missing explicit opt-in remux contract; default exact extraction must remain |
| Optimization | `IsOptimizationSupported` explicitly rejects sample entry **hvc1**, not every HEVC entry; `OptimizeMp4File` uses mp4v2 and a compile flag | offset-stable append-only rewriting, no fast-start optimizer | missing compact/fast-start optimizer; no imitation of hvc1 restriction needed for byte copy |
| Codec / AAC constraints | checker uses track types/counts and metadata consistency; not a universal device capability oracle | sample-entry FourCC exposed, AAC parameters not interpreted | partial; add inspectable codec/audio facts and conservative plan validation, preserve product policy ownership |

## Deliberate scope and evidence boundaries

Java/JNI, Android service packaging, and complete CLI delivery are **later work**.
Playback UI, editing SDKs and capture systems are out of scope. Existing Apple
support must regress cleanly; no Apple feature expansion is planned. Native
Gallery/Photos acceptance remains **unverified**, on every platform.

Test data must remain outside Git unless redistribution is established. No private
portraits, proprietary APKs or credentials are added. Reference dependencies have
separate licenses; the Google top-level Apache-2.0 license does not relicense
libheif, mp4v2, codecs or their dependencies. Reference build details and executed
test results will be recorded in a separate run receipt before delivery.

[google]: https://github.com/google/libmotionphoto/tree/12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3
