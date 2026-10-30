# Normative clauses, implementation and tests

Three independent questions must stay separate:

1. **Format requirement:** a clause in an identified public/user-provided text.
2. **Reference parity:** behavior of Google commit
   `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`, including its implementation limits.
3. **Apple compatibility:** actual public-framework operations on an identified
   runtime, with permissions/assets and hardware acceptance recorded separately.

`ParseOptions::strict()` validates the library's supported structural/profile
contract. It is **not** a complete Google, XMP, BMFF, HEIF or Apple conformance
certifier. `MediaCapabilities` is caller-supplied and optional. A rejected valid
layout can be an unsupported subset rather than a malformed standard file.
The [baseline receipt](stable-baseline-4848e5f.json) certifies only its SHA;
the [76c25e4 receipt](standards-edit-76c25e4.json) independently records the new
code/tests, exact-SHA native processes and all three successful workflows.
H05's later implementation has its own [local receipt](h05-69ebefe.json).
Optional Google declaration checks and explicit scoring projection passed the
three portable hosts and Apple/web at91fc380; actual Android acceptance failed
at installation before service scopes. [Exact terminal evidence](conformance-91fc380.json)
preserves that distinction;76c25e4 does not certify later code. CameraAlbum is a
separate observed vendor contract, not a new normative Google/HEIF profile.

## Authorities and edition boundaries

* [Google Motion Photo format 1.0](https://developer.android.com/media/platform/motion-photo-format),
  page last-updated 2024-07-10, read 2026-10-19. Section titles below identify
  clauses; source parity has its own [pinned source/test map](SOURCE_TEST_MAP.md).
* [Adobe public XMP specifications](https://developer.adobe.com/xmp/docs/xmp-specifications/):
  Part1 April2012 and Part3 January2020. Public PDFs were actually read locally;
  SHA256 Part1 `e0056b9d5c29d4d21eda1856cf947b54cee3013944638a9871aed7d67a949090`,
  Part3 `2e80d671f87501c91ab18c0722c233e35a72d1c9245c70accb743917ebe2eb18`.
  PDFs are not included in this repository.
* The user supplied **ISO/IEC23008-12:2025, edition3, July2025**, Chinese
  transcription `ISO_IEC_23008-12_2025_zh(1).md`, 339029 bytes, Library ID
  `libfile_1e0bff0056cc8191bafdd46f9cb3ff89`. Authorized Library reads verified
  its identity and the clauses below. The required materialization preparation
  succeeded; the returned signed transfer failed with proxy CONNECT403 before
  any local body file was written. [Access receipt](iso23008-materialization-access.json).
  This is **provided text with a blocked container transfer**, not an absent
  standard. No full body or long excerpts are committed/uploaded. Transcription
  uncertainties remain unresolved; figures/OCR do not create inferred rules.
* Google cites **ISO/IEC14496-12:2015** and **ISO/IEC23008-12:2022**. Public ISO
  catalogue checks on 2026-10-19 identify [14496-12:2026 edition8](https://www.iso.org/standard/85596.html)
  and [23008-12:2025 edition3](https://www.iso.org/standard/89035.html), with the
  latter listing amendments1:2025 and2:2026. These catalogue/version differences
  are recorded, not a clause-level revision comparison. No authorized
  14496-12 body, 23008-12:2022 body or amendment text was obtained here; no such
  clauses or complete compatibility claims are inferred, and nothing is bought.
* [Apple QTFF](https://developer.apple.com/documentation/quicktime-file-format)
  and [livePhotoMovieMetadata](https://developer.apple.com/documentation/avfoundation/avcapturephotosettings/livephotomoviemetadata)
  are the requested public authorities. This reader received JavaScript shells;
  advertised Markdown returned unsupported-content-type in the reader and
  CONNECT403 in the container. No full normative clause audit is claimed for
  those bodies. Available SDK declarations and actual framework tests are
  identified below as interface/runtime evidence.

## Google Motion Photo 1.0

Test names refer to the linked Rust source unless another harness is named.
Both positive and negative bindings are given; an untested condition is a gap.

| ID / section and clause topic | Function / contract | Positive and negative test bindings | Support status / limit |
| --- | --- | --- | --- |
| G01 Motion Photo Format: still plus appended movie | `MotionPhoto::parse`, plan/execute Android profiles | [core_contract](../crates/liblivephoto/tests/core_contract.rs): `avif_build_preserves_items_video_xmp_and_time_without_pixel_encoding`; [conversion](../crates/liblivephoto/tests/conversion.rs): `missing_pair_identity_or_time_and_transcode_targets_are_reported` | Verified JPEG/HEIC/AVIF supported layouts; coded bytes retained. No generic decoder validation. |
| G02 Camera Metadata: flag/version and actual video | `parse_android_motion_photo`, `MotionPhoto::parse` | [android unit tests](../crates/liblivephoto/src/android.rs): `parses_android_directory_and_timestamp`, `rejects_missing_version`, `rejects_stale_directory_without_real_video` | Supported declarations; a metadata flag is not source verification. |
| G03 Presentation behavior: exact time / score agreement | `MediaTime::rescale_exact`, `presentation_time`, `scoring_metadata`, plan | [conversion](../crates/liblivephoto/tests/conversion.rs): `apple_time_without_an_exact_microsecond_mapping_blocks_android_writers`; [video_metadata](../crates/liblivephoto/tests/video_metadata.rs): `composition_rejects_known_scoring_time_conflicts_but_preserve_copy_remains_exact`; [core_contract](../crates/liblivephoto/tests/core_contract.rs): `malformed_and_conflicting_declared_time_cannot_silently_become_unknown` | Exact observed time and conflicts verified. Missing/-1 remains unknown; the reader's recommended midpoint display fallback is not synthesized into source metadata. |
| G04 Container/Item: primary first, unique, optional primary Length | `validate_directory`, `XmpDocument::validate_motion` | [standards_contract](../crates/liblivephoto/tests/standards_contract.rs): `primary_length_omission_is_supported_but_positive_length_is_a_profile_limit`; [android units](../crates/liblivephoto/src/android.rs): `rejects_motion_photo_item_that_is_not_last` | Primary Length absent/0 supported. Requiring zero when supplied is a stricter implementation subset, not a universal normative MUST. Motion item must have a positive length in this subset. |
| G05 Item: padding placement and tight resource ranges | `derive_jpeg_style_ranges`, `XmpDocument::rewrite`, `android_description` | [standards_contract](../crates/liblivephoto/tests/standards_contract.rs): `directory_padding_is_preserved_and_secondary_nonzero_padding_is_rejected`, `directory_rewrite_emits_padding_only_on_primary_and_refuses_secondary_padding` | JPEG padding bytes preserved; writers emit Padding only on primary. Compatibility reads accept secondary zero attributes; their presence is not normative writer output. |
| G06 Item: shared secondary resource | `complete_model`, resource extraction | [reader](../crates/liblivephoto/tests/reader.rs): `zero_length_unknown_item_is_a_shared_resource_and_remains_extractable`; `unknown_video_directory_item_does_not_replace_primary_motion` | Tested shared auxiliary bytes. Not every zero-length container-sharing arrangement is supported. |
| G07 ISOBMFF-image-specific: final mpvd, explicit size and padding | `resolve_heif_motion_photo_ranges`, Android HEIF/AVIF writers | [standards_contract](../crates/liblivephoto/tests/standards_contract.rs): `heif_motion_photo_rejects_zero_sized_mpvd_even_when_generic_bmff_allows_it`; [heif units](../crates/liblivephoto/src/heif.rs): `extended_mpvd_requires_matching_directory_padding`, `extracts_mpvd_payload_without_trailing_vendor_box` | Standard writer uses 8-byte header/padding. Read compatibility also accepts matched16-byte headers and vendor trailers. The page's table8 versus general header-size wording is not resolved into a universal16-byte compliance claim; trailer compatibility is not strict final-mpvd compliance. |
| G08 Video container: video codec and optional AAC constraints | optional `validate_google_video_container`, `sample_descriptions`, caller `plan_with_capabilities` | [google_video_conformance](../crates/liblivephoto/tests/google_video_conformance.rs): codec/all-description, AAC48/96/mono/stereo/depth, negative multichannel/rate/depth, audio count, PCE/v2/wave/duplicate declarations and property budgets; [protocol](../crates/liblivephoto-interface/tests/protocol.rs) explicit profiles/errors | Selected declarations/count rules implemented and tested, separate from default preservation. Unknown/PCE/v2/wave/ambiguous44100Hz stays Unsupported. No codec decoding, actual source-depth certification or complete AAC/bitstream conformance claim. |
| G09 Video container: secondary video order/correspondence | optional explicit track association, `video_frame_times`, observed `VideoDimensions` | [google_video_conformance](../crates/liblivephoto/tests/google_video_conformance.rs): missing/extra/ambiguous roles, reversed order, ctts/edit/cross-scale rational matching, unmatched recommendations, fragmentation/nonunit and aggregate budget | Required order/count and recommended exact timestamp correspondence checked separately. No closest-frame selection. The public section leaves dimensions undefined, so dimensions are exposed without an invented fixed-size or secondary>primary pixel threshold. This does not certify correspondence of decoded image content. |
| G10 Video Metadata Track / Syntax | `metadata_packets`, `decode_video_metadata`, semantic validation; explicit `project_scoring_media_times` | [video_metadata](../crates/liblivephoto/tests/video_metadata.rs): MIME/semantics/budgets; [scoring_time](../crates/liblivephoto/tests/scoring_time.rs):9 positive/negative raw-global/media-CTS/ctts/empty/trim/repeat/association/exactness/budget cases | Existing C0–C5 payloads/MIME/raw unknown fields verified. Raw Google fields compare directly to XMP even with metadata-track edits. Opt-in projection requires caller-declared pre-edit video domain/track IDs; it never guesses that interpretation or rewrites bytes. Not a full one-sample/whole-bitstream certifier; no Moments authoring. |
| G11 Ultra HDR primary: gain-map directory/resource preservation | `resources`, extraction and unmapped policies | [android units](../crates/liblivephoto/src/android.rs): `preserves_positive_length_gain_map_in_static_resource`; [conversion](../crates/liblivephoto/tests/conversion.rs): `unknown_suffix_requires_an_explicit_sidecar_or_drop_policy` | Byte/resource preservation tested; gain-map application and rendered HDR fidelity unverified. |
| G12 Filename Pattern recommendation | caller/CLI output names | CLI no-overwrite tests; no dedicated filename-conformance test | Not enforced. Successful core parsing or CLI output is not proof that a native reader will accept the filename. |
| L01 Legacy compatibility, outside current Google requirements | legacy provider / enable-list; `LegacyMicroVideoV1b` | [android units](../crates/liblivephoto/src/android.rs): `parses_legacy_micro_video`; [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs): `custom_registration_and_filters_apply_to_decoding_and_core_parsing`; provider differential controls | MicroVideo is a separate legacy profile. The current standard deletes those fields; default compatibility and conflicting-legacy rejection are not universal modern-profile compliance. |

## Adobe XMP and user-provided HEIF clauses

| ID / authority clause | Function / contract | Positive and negative test bindings | Support status / limit |
| --- | --- | --- | --- |
| X01 Adobe Part1 §6.2, expanded names | `android::parse_xmp`, namespace-aware camera/directory parse | [reader](../crates/liblivephoto/tests/reader.rs): `namespace_uri_determines_motion_fields_and_prefixes_can_change` | Tested alias equivalence and unrelated namespace rejection; no full RDF processor claim. |
| X02 Adobe Part1 §7.3–7.5 serialization subset | `XmpDocument`, bounded XML reader/rewrite | [core_contract](../crates/liblivephoto/tests/core_contract.rs): `element_camera_properties_are_supported`; [reader](../crates/liblivephoto/tests/reader.rs): `malformed_nested_xml_and_input_budgets_fail_closed`; [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs): `empty_and_multiple_root_xmp_documents_are_rejected` | Attribute/element values and raw packet retention supported; DTD/entities rejected as a security subset. No full RDF graph equivalence. |
| X03 Adobe Part3 §1.1.3 JPEG storage | `xmp_bytes`, `jpeg_write::rewrite_packet`, raw block extraction | [metadata_engine](../crates/liblivephoto/tests/metadata_engine.rs): `standalone_xmp_rewrite_retains_unknown_values_and_exact_time_conversion`, `metadata_in_progressive_scan_tail_is_not_lost`; malformed JPEG/header regressions | Base APP1 packet/unknown bytes preserved. Extended-XMP graph assembly, reconciliation of every native schema and all Part3 containers are outside current coverage. |
| H01 ISO23008-12:2025 §6.2, item sources | `heif_io::items/replace_metadata`, format `read_item_payload` | [standards_contract](../crates/liblivephoto/tests/standards_contract.rs): `heif_extent_resolution_supports_methods_zero_and_one_but_blocks_other_sources`, `heif_idat_extents_cannot_escape_into_other_boxes_that_fit_the_file`; [format exif units](../crates/liblivephoto-format/src/exif.rs): `item_payload_bounds_idat_before_copy_and_counts_repeated_ranges` | Self-contained construction0/1 supported; absolute extents survive metadata replacement, relative extents stay in idat. Construction2/external references are valid possibilities in the general text but unsupported here; brand-specific/full-HEIF certification is absent. |
| H02 ISO23008-12:2025 §6.5.1, property sequence | `media::inspect_still`, original ipma/ipco retention | [standards_contract](../crates/liblivephoto/tests/standards_contract.rs): `heif_mirror_axes_and_order_match_asymmetric_coordinate_landmarks`; [media_contract](../crates/liblivephoto/tests/media_contract.rs): `heif_primary_property_order_hdr_signaling_and_unknown_properties_survive_build` | Orthogonal transforms follow association order. Raw descriptive/unknown essential properties are retained, not rendered or universally validated; preserving an input's order is not a claim that every input is a conforming writer output. |
| H03 ISO23008-12:2025 §6.5.10.2–.3, irot | `OrthogonalTransform`, transform property decoder | Same asymmetric/order test, including invalid reserved bits | Supported 90-degree CCW facts in x-right/y-down coordinates. No pixel rotation/crop renderer. |
| H04 ISO23008-12:2025 §6.5.12.2–.3, imir | `media::inspect_still` mirror mapping | Same test: each axis, reversed association order, invalid bits and exact retained properties | Axis0 exchanges top/bottom; axis1 left/right. Corrected the old reversed interpretation. This is also consistent with [libavif's public header](https://github.com/AOMediaCodec/libavif/blob/main/include/avif/avif.h); not inferred solely from a translated axis label. |
| H05 ISO23008-12:2025 §6.5.5.1–.3, color information | `validate_heif_color_information`, `heif_color_conformance`, `FormatConformanceReport` | [standards_contract](../crates/liblivephoto/tests/standards_contract.rs): `heif_color_count_and_legal_pairs_are_checked_without_changing_color_or_coded_bytes`, `heif_color_violations_are_explicit_and_never_repaired_by_preservation_writers`, `unknown_color_syntax_and_duplicate_associations_are_unsupported_not_guessed`, `auxiliary_image_colors_are_merged_across_ipma_boxes_and_checked_per_item`, `color_validation_preflights_rows_and_counts_repeated_payloads_across_items` | Count, ICC+nclx pairing and paired primaries/transfer2 constraints implemented as optional checks for supported image-item types, across all ipma boxes. Positive/negative byte-preservation cases pass locally. Normative violations and unsupported/unknown facts are distinct. Library body read at1097–1127; full referenced BMFF color syntax, ICC contents, rendering and track/image conversions are not certified. |
| H06 ISO23008-12:2025 Table3 / broader brands | bounded box/item parser | Existing malformed box/iloc count/extent tests and ASan campaign | Supported subset only. Table transcription/brand requirements are not used to invent absent fields or claim full mif1/HEIF/sequence/derived-image decoding. Full local materialization and unread/ambiguous clauses remain pending. |

The metadata payload helper caps referenced copies at16MiB/4096 extents before
allocation, including repeated ranges. Resource views can preserve larger coded
items without copying them through that metadata helper. Limits are implementation
budgets, not normative maximum image sizes.

## Apple system contract and independent acceptance

| ID / authority boundary | Function / contract | Positive and negative tests | Status |
| --- | --- | --- | --- |
| A01 QTFF atoms, sample/composition/edit times | bounded box parser, `video_frame_times`, raw packets and edited AGTM selection | [frame_timing](../crates/liblivephoto/tests/frame_timing.rs) unit-rate/ctts/exact fractional tests; [timed_metadata_edits](../crates/liblivephoto/tests/timed_metadata_edits.rs) empty/trim/repeat, unsupported rates, overflow, budgets and fragments | Implemented/tested supported timeline subset. Full QTFF body unavailable; no complete QTFF compliance claim or fragmented/dwell/reverse edit support. |
| A02 Apple still-time/pairing metadata | `read_live_photo_still_presentation`, Apple pair parse/write | [live_photo_mov units](../crates/liblivephoto/src/live_photo_mov.rs): `still_presentation_preserves_integer_movie_ticks`, `exact_still_presentation_does_not_invent_zero_for_missing_edits`, `exact_apple_still_time_rejects_nonunit_and_noncanonical_edits`; [conversion](../crates/liblivephoto/tests/conversion.rs): Apple byte/time regressions | Apple exact still-time reader supports its canonical one segment, or leading empty plus media-start0, unit-rate v0/v1 edits. General frame/AGTM edit support does not expand this Apple pairing subset. |
| A03 livePhotoMovieMetadata/public framework integration | host-owned capture settings, PHLivePhoto resources, Swift C ABI | [native resource test](../apple/Tests/LibLivePhotoAppleTests/NativeResourceTests.swift), [contract tests](../apple/Tests/LibLivePhotoAppleTests/ContractTests.swift), [simulator process](../apple/Acceptance/BridgeSimulator.swift) | Baseline4848 actually loaded/exported a local pair, ran11 macOS tests and the SE3/iOS26.2 Swift/Rust process. SDK26.5 is a compiler SDK. Earlier26.5 startup failures remain unresolved. |
| A04 import/edit/playback/HDR/capture acceptance | public PhotoKit/AVFoundation adapters | Compiled adapters and object/lifecycle tests; physical permissioned operations still unavailable | Blocked on authorized device/library/assets for actual import/render/commit/display/camera. Local resource export and CI cannot close those gates. |

Public UWA proposals are separately mapped in [UWA_TECHNICAL_REFERENCE.md](UWA_TECHNICAL_REFERENCE.md).
Their proposal summaries do not create a new implemented protocol or compliance
profile. Moments construction remains excluded.
