# Google metadata and registry stage

The baseline remains Google `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`.
This stage adds actual core capabilities; complete CLI/Java/JNI/Android wrappers
are still separate unfinished work. Moments construction remains excluded.

| Capability | Implementation | Authority and limits |
| --- | --- | --- |
| Standalone XMP parse/projection/serialize/rewrite | `XmpDocument`, `XmpFields`, directory entries, exact timestamps, retained raw bytes and unrelated XML | Google `motion_photo` camera/container reader/writer headers and source. Motion-declaration validation is separate from ordinary XML parsing; actual resource bounds need the source asset. Non-exact microsecond conversion fails. |
| Ordinary JPEG/HEIC/AVIF metadata | `MetadataEngine::parse/parse_path/parse_range/parse_fd`, metadata block source/header extents, raw payloads, provider projections, JSON serialization and bounded validation | Google `MetadataEngine`, JPEG/HEIF splitters, `metadata_collection.proto`. JSON is this library's schema, not protobuf/JNI ABI compatibility. JPEG marker scanning reuses the original entropy/marker parser, including metadata between progressive scans. |
| Provider registration and enable/disable | Owned `MetadataProvider` trait objects, duplicate/reserved-name rejection; `HandlerOptions`; `MotionPhoto::parse_with_handlers` | Google `IMetadataProvider`, registry and handler options. Standard Google and Apple paths remain independent of third-party enable lists. A semantic motion declaration is distinct from a core-validated actual motion asset. |
| Persisted collection classification | `declares_motion`, serialized bounded collection import | Imported `verifiedMotion` never authenticates current source bytes and is cleared; extents and lengths are validated. Call the core on actual resources to establish a valid media asset. |
| Existing Google scoring/stabilization | `decode_video_metadata`, `VideoMetadata::validate`, `metadata_packets`, `extract_video_metadata`, `MotionPhoto::scoring_metadata` | `video_metadata.h/.cc`, `VideoMetadataDecoder`, checker. Tags C0–C5, descriptor lengths, model version, IEEE score bits, integer timestamps, ordered frame scores, both raw flag bytes, 0x80 stabilization bit, unknown descriptors/extensions. No scores or Moments are authored. |
| Composition consistency | Existing `plan` checks recognized scoring time/range/order issues before changing containers; exact preserve-source copying remains available | Known semantic conflicts/malformed declared Google descriptors block composition. Unsupported semantic layouts are reported while their encoded bytes are preserved. No implicit metadata deletion or time repair. |
| Standalone image display/color facts | `inspect_image_metadata` reuses the existing still inspector | Observed EXIF/HEIF transform and color facts only; no invented SDR/HDR or AGTM rendering. |
| Samsung additional variants | `inspect_samsung_metadata` exposes raw records and optional movie; compatible core reads inline mpv2/mpv3, pointer variants, sefd before/after mpvd and empty Smart Switch stub | Pinned Samsung provider/proto/tests. Raw MP4, both header magics, HEIF resource association, and metadata-only SEF are covered by structural tests. Broader corpus and native Gallery support are not inferred. |

Large metadata resources (video trailers, mpvd, HEIF coded items) are **bounded
views**: only a 64 KiB raw prefix is buffered; exact extents and `rawComplete=false`
are serialized. `MetadataCollection::extract_block` copies complete bytes from the
caller-retained original with an explicit output budget. This avoids copying an
entire large movie into a metadata response. EXIF/XMP/ICC/JFIF payloads remain
complete within a 16 MiB aggregate metadata budget. Raw binary JSON uses a hex
string, avoiding per-byte JSON object allocation. Input and descriptor budgets
are explicit. Serialization does not preserve an entire source file by itself.

Custom providers receive original source bytes and an evidenced block. Relative
video extents are validated before mapping to source ranges. A provider's semantic
payload is a declaration, not proof of decodability or native vendor acceptance.

Validation before checkpoint: full workspace tests and clippy; existing 14-file
fixture matrix still passes; nine independent descriptor vectors compare the
pinned Google decoder's model, score bit patterns, exact timestamps, frame lists
and stabilization values. Empty/unknown/truncated and semantic-error cases are
validated independently in Rust. Exact commit receipts are recorded after the
implementation checkpoint. Platforms and wrappers not actually run remain open.
