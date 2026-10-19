# Core integration

This phase extends the implementation at PR #1 commit
`794ab552596db0f66095600d23cf7937a631b9ec`. It does not replace the existing
resource model, Apple writer or plan/execute boundary. The [initial coverage matrix](evidence/GOOGLE_CORE_COVERAGE.md)
is a historical checkpoint; the [current inventory](evidence/GOOGLE_COMPLETE_INVENTORY.md)
and [source/test map](evidence/SOURCE_TEST_MAP.md) track implemented subsets,
operation-specific contracts and remaining acceptance requirements.

## Inputs and outputs

`parse(Input, options)` borrows memory. `open(PathInput, options)` owns a bounded
snapshot. `PathInput::Parts` adds explicit still/movie composition from paths,
with caller-supplied exact time. It does not infer pairing from filenames.

`open_file(FileInput, options)` snapshots exact ranges of open regular files.
`FileInput` supports single-file assets, Apple pairs and caller-declared parts.
`FileRange { file, offset, length }` requires a nonempty, in-file range. Offsets
in the resulting resource graph are relative to the snapshot. The combined
input budget applies to both parts before allocation. A shrinking input fails
with an I/O error. Callers must keep input contents stable during the read.

On Unix, `open_fd(BorrowedFd, offset, length, options)` duplicates a borrowed
descriptor and uses positional reads. The caller retains ownership and its
cursor. On Windows, `open_file` restores the cursor after `seek_read`; callers
must synchronize other operations on the file and cloned handles. Windows and
macOS tests and packaged consumers actually passed at verified baseline
`4848e5f`; see [the exact execution receipt](evidence/stable-baseline-4848e5f.json).
Other platforms report unsupported positional input.
Zero length does not mean the remainder of the file.

All input forms feed the same parser and resource extractor. `extract` writes
to any Rust `Write`, including a caller-opened file, descriptor-backed `File`,
or vector. `plan(...).execute()` returns `ComposedAsset` buffers and reports.
There is no direct streaming build-to-fd API, output-range overwrite or implicit
publication. Callers own publication and atomicity. `still_bytes()` returns the
still container, including its embedded auxiliary payloads, not decoded pixels.

`xmp_bytes()` returns the original primary packet without serialization. JPEG
returns a borrowed packet without the APP1 identifier. HEIF/AVIF assembles the
bounded XMP item extents. `None` means no packet; malformed or ambiguous metadata
returns an error. Container metadata resources remain independently extractable.

## Target selection and media requirements

`AndroidAvif` requires an AVIF still. `AndroidHeif` retains the HEIC contract.
Both rewrite metadata and append `mpvd`; neither decodes or re-encodes pixels.
An AVIF/HEIC/JPEG mismatch requires caller-controlled transcoding. The Apple
writer gains no AVIF target. Android time still requires exact integer
microseconds; 1/3 second remains blocked even under sidecar/drop policies.

`sample_descriptions()` exposes track IDs, every sample-entry FourCC, raw codec
and HDR properties, exact tkhd matrices, coded sample-entry dimensions, and
audio version/sample-size declarations. `inspect_video_sample_descriptions`
provides these facts for standalone MP4/MOV inputs under cumulative budgets.
AAC AudioSpecificConfig supplies object type, sample rate and channel count.
Channel configuration zero remains unknown because PCE parsing is absent.
QuickTime v2/wave audio, codec-level VUI/SEI interpretation and full decoder
conformance remain outside this inspection contract. Raw sample-entry audio
fields and AAC declarations are distinct facts; the library does not reconcile
all possible disagreements between them.

The caller can pass `MediaCapabilities` to `plan_with_capabilities`. For example,
a caller can permit `avc1`/`hvc1`, AAC object type 2, at most two channels, and
44.1/48 kHz. These are illustrative caller choices, not inferred device limits.
All sample descriptions must satisfy the contract. Known incompatible codecs or
audio parameters produce `TranscodeRequired` and block execution. Unknown AAC
facts produce an unmapped/blocking reason rather than an unsupported transcode
claim. The default `plan` still promises container/resource preservation only.
The library does not select an encoder or transcode media.

Optional `validate_google_video_container` / `google_video_conformance` checks
the explicitly listed Google1.0 declaration/count/order rules. The caller names
primary/secondary track IDs; unknown role assignments never become a pass.
`Violation`, `Unsupported` and `Recommendation` are distinct. Exact secondary
frame correspondence is a recommendation; it uses rational CTS after supported
edits and never selects a closest frame. Sample dimensions are observed facts:
the source gives no fixed resolution requirement. AAC48/96kHz and the literal
44000Hz are checked exactly;44100Hz is Unsupported because the public page says
44kHz, with no silent reinterpretation. A caller capability contract can still
explicitly permit44100. PCE, QuickTime v2/wave and unfamiliar codec declarations
remain unsupported by this optional profile. No bitstream/pixel certification
or native-device compatibility is inferred from its report.

CLI/C/JNI expose the read-only `conformance` operation for one immutable input:

```sh
llp conformance STILL.avif --options-json '{"profile":"heif-color-2025"}'
llp conformance VIDEO.mp4 --tracks 1,2 --options-json '{"profile":"google-video-container-1.0"}'
```

The video profile treats the first ID as primary and optional second as
secondary. It accepts a standalone movie; a Motion Photo's caller can first
extract its original movie or use the Rust method. Required-rule failure or
Unsupported returns `success=false` with rule diagnostics; recommendations
remain visible. Default `check`, parse/build and byte preservation are separate.
Android remote services retain their existing role allowlists; this new action
is not claimed as an installed-service feature.

## Display and color facts

`still_metadata()` exposes raw EXIF orientation and a shared orthogonal-transform
representation for the eight EXIF orientations and HEIF `irot`/`imir` properties.
`imir` axis0 exchanges top/bottom; axis1 exchanges left/right, following the
clear user-provided ISO23008-12:2025 §6.5.12 text and libavif's public declaration.
The matrix uses x-right/y-down coordinates. The API retains EXIF and HEIF
declarations separately. It does not invent a composition order between them,
apply a crop, or normalize an arbitrary movie matrix. An absent orientation and
an unknown vendor value such as zero remain distinct; neither becomes an
observed identity transform.

ICC profiles remain opaque bytes. CICP primaries, transfer, matrix and range are
explicit fields. Transfer codes 16 and 18 identify signaled PQ/HLG; absent or
uninterpreted color metadata does not mean SDR. Primary HEIF property order,
unknown properties, codec configuration and raw HDR properties are retained.
ICC interpretation, gain-map application and HDR pixel rendering are absent.
The [normative/source/runtime map](evidence/NORMATIVE_CONFORMANCE.md) records
which clauses are supported subsets. HEIF self-contained construction0/1 is
supported; construction2 and external item sources are rejected explicitly.
Absolute item data inside the old meta payload survives append-only metadata
replacement; construction1 ranges stay inside their idat. The bounded metadata
payload assembler preflights4096 extents/16MiB referenced bytes before copying.

`validate_heif_color_information(still)` and
`MotionPhoto::heif_color_conformance()` optionally check ISO23008-12:2025
§6.5.5 count, ICC/nclx pairing and paired primaries/transfer=2 declarations.
They merge supported image-item associations across all ipma boxes. The typed
report separates `Violation` from `Unsupported`; `checked_rules_satisfied()`
requires neither kind of issue. A standalone report covers its listed rules,
not complete HEIF/BMFF, ICC profile validity or rendered color. Unknown item or
color types/layouts and repeated associations are not guessed. Callers can
require this report before planning/publishing; default preservation never
silently changes a contradictory declaration to make the report pass.
The query bounds items/properties/association rows/issues to4096,
associations to65536 and referenced colr bytes to16MiB. These are explicit
implementation budgets, not normative maximum image sizes.
Fact extraction allows at most 4096 primary property associations and 16 MiB of
their payload bytes, so repeated references cannot amplify allocation without
bound. A conversion plan reports inspection failures before writing.

Writers compare still display/color facts after the rewrite, alongside existing
encoded media and exact-time checks. Reports include `StillDisplayMetadataEqual`
and `StillColorMetadataEqual` for these measured comparisons. These entries do
not certify every color, crop, gain-map or vendor display semantic.

## Timed AGTM and explicit remux

`inspect_camera_album_footer` reads the exact terminal CameraAlbum envelope
observed in two independent vivo originals. It exposes original JSON/ranges,
integer declarations and envelope/JSON ID agreement under explicit size, nesting
and duplicate-key limits. `MetadataEngine` exposes provider `camera_album` and a
fully extractable `camera-album.footer` block through its memory/path/fd/range
APIs and maintained `metadata` request. `disableThirdParty`/enable lists apply.
Unknown JSON stays original text in `rawJson`, including number spelling; no
unknown stream is called depth, no ID equality proves capture pairing, and raw
`imageTime` is not converted through average FPS. The separate55-byte/length2=2
variant has static/synthetic evidence only. This adds no native vendor writer or
Gallery playback/editing claim. See [vendor evidence and gaps](evidence/VENDOR_RESEARCH_20261019.md).

`inspect_vivo_media_extensions(movie)` and `MotionPhoto::vivo_media_extensions()`
read top-level `vivoMediaEStream`/`vivoMediaExtInfo` UUIDs from a complete,
contiguous BMFF snapshot with `ftyp` first. Leading-box/noncontiguous layouts
are outside this additional inspection contract. Both independent vivo
originals contain31 stream records. The API borrows exact payloads and reports
ordered numeric keys, opaque encoding tags, directory/framing ranges and the
existing CameraAlbum footer. Duplicate keys remain separate records. Unknown
tags do not become depth, portrait, gyro or rendering semantics; footer time
units and pair uniqueness remain unverified. Method ranges are relative to the
original movie, while provider `vivo_media` maps extractable `vivo.stream-payload`,
directory and footer-box blocks to the enclosing source snapshot. Path/fd/range
metadata calls and C/JNI share this adapter. Standalone container inspection is
not movie validation or native Gallery acceptance. Malformed optional vendor
metadata produces diagnostics while the original container trailer remains
extractable. Multiple known UUIDs require an explicit association and fail.
Limits are256MiB input,65536 top-level boxes,4096 streams/one of each known UUID
and16MiB aggregate stream payload; metadata collections also retain their
existing4096-block/16MiB cumulative bounds. No native writer was added. See
[independent source and original-byte evidence](evidence/VENDOR_RESEARCH_ROUND2.md).

`agtm_at(MediaTime)` returns original metadata sample bytes, track/sample IDs,
presentation interval and decoded version-0 baseline fields. It reads validated
sample tables and honors `ctts`. Explicit `it35`/T.35 signaling establishes the
association. Missing samples return an empty vector. Unit-rate edit lists map
empty, trimmed and repeated segments into movie time, clipping sample intervals
at edit boundaries without changing payloads or source sample indices. Edit
durations and queries must convert exactly to track ticks; non-unit rates,
overflow, fragmentation, unsupported versions, truncated baseline fields and
nonintegral time conversion fail explicitly. The method does not scan arbitrary movie bytes,
substitute sample 1 or fabricate a baseline for missing/truncated samples.

`decode_agtm` parses every version-0 field read by the pinned Google decoder:
all alternate-image rules, common/independent curves and mixes, x/y coordinates,
explicit/PCHIP slope selection, angles and tangent slopes, mode selectors and
custom chromaticities. Truncated fields fail. Full raw input and unknown trailing
bits remain preserved. Google ignores the six encoded mix coefficients and
leaves its mix struct at zero; this API exposes those original integers without
inventing weight scaling. It also retains unknown preset selectors as selectors.
`reference_json()` provides Google's first-rule projection; `details_json()`
exposes every rule. `extract_agtm_from_video` supports standalone MP4/MOV inputs.
The API extracts metadata, not rendered pixels. Existing Google scoring and
stabilization descriptors are decoded by `decode_video_metadata` and
`extract_video_metadata`: raw/unknown bits, exact times and semantic issues stay
available. Nine same-input decoder vectors match the pinned reference. Moments
construction remains explicitly excluded.

`metadata_packets` reports raw decode-order packets, including composition
offsets, on the track media timeline before edits. These raw packets remain
inspectable for unsupported edit rates. `extract_video_metadata` validates
payload-internal timestamps as declared movie times, including direct XMP
comparison; it never adds the metadata track's ctts or edit offset to them.
`project_scoring_media_times` is an explicit alternative for caller-declared
pre-edit video media CTS. The caller identifies primary/high-resolution video
tracks separately. It matches exact source CTS (including ctts), then maps
sample indices through unit-rate edits. Returned score references retain raw
bits; sorted movie times expose repeats, with explicit trimmed/unspecified
states. Missing frame matches, unsupported rates, overflow, fragmentation,
nonintegral microseconds and aggregate output limits fail. It does not author
a replacement payload or choose one repeated occurrence. Timed AGTM
selects one sequential edit before scanning samples, avoiding a samples × edits
allocation/work expansion; the existing aggregate extraction budgets remain.

`remux::remove_metadata_tracks(movie, ids)` is explicit opt-in deletion. Only
unique metadata track IDs are accepted. The operation rejects fragmented/indexed
movies, track references and unsupported movie-level boxes. It retains original
media data and appends an active `moov`, with before/after track validation.
Its report lists removed and retained IDs. Unreferenced sample bytes remain in
`mdat`; this operation does not erase their contents. It does not compact files,
import tracks from another movie or implement fast-start optimization.
Default extraction/building does not remove metadata tracks, including AGTM.

Additional `remux::import_tracks`, `remove_tracks`, `optimize`,
`optimization_support` and `track_summary` APIs provide bounded general remux.
They rewrite/promote chunk offsets, preserve encoded samples and exact timing,
allocate unique imported IDs and report metadata sidecars/discarded bytes.
Unreferenced media stays retained unless compaction is explicitly allowed.
Fragmented/indexed/external/reference-bearing unknown contracts are rejected.
Pinned MP4CopyTrack/optimizer/delete comparisons cover real H.264 B frames, AAC,
existing Apple metadata and hvc1. See `scripts/mp4_differential.py`.

`MetadataEngine` reads ordinary JPEG/HEIC/AVIF block/header extents, raw metadata,
provider projections and handler controls. Persisted collection classification
clears source verification. Complete selected blocks can be extracted from the
original source under a separate budget. Namespace-equivalent prefixes and
ordinary HEIF XMP without mpvd are exposed where the pinned provider/splitter
does not expose them. Raw metadata extraction does not decode pixels.

## Consumer migration

The existing public resource/track fields, exact-time rules and `xdremux-compat`
exports remain. `PathInput` adds a `Parts` variant; downstream exhaustive matches
must handle it. New target profiles and diagnostics use the existing
non-exhaustive enums. The public converter adds `google-avif` as a directory
writer, not a device-certification profile.
The unstable compatibility predicate `is_heif_mime` now includes `image/avif`;
downstream HEIC-only product routes must distinguish AVIF explicitly.

XDRemux PR #58 at `0688ea7d6270df52cc5968e2c36dd016f13bd31b` was read only.
It still pins the old library SHA. No consumer file or pin is modified. The
unchanged public interop test can be replayed against this library through
`scripts/consumer_replay.py`; that external dependency alias is not a full
XDRemux product build. The ordinary external consumer also exercises the
Android → Apple → Android → Apple route. Native Photos/Gallery acceptance,
private vendor writers and application codec/HDR policies remain separate work.
