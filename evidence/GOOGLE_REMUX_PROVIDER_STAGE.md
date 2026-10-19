# Google provider/remux implementation stage

Reference remains `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`.
Moments construction remains excluded. Google work continues alongside the Apple
adapter/local frontend; neither replaces the complete Google inventory.

New core APIs:

* `inspect_oplus_metadata`: JPEG OpCamera namespaces, attributes/elements and all
  three timestamp aliases; plain BMFF, short/fixed JXRS footer, both JSON-directory
  footer-length conventions; complete bounded backward record extents and unknown
  JSON values. Metadata-only identification is distinct from a movie. Prefix aliases
  resolve by namespace. Conflicts/overlap/truncation/overflow fail explicitly.
* The normal compatible reader now uses these evidenced ranges before legacy
  Oplus recovery, retaining existing Google-directory/LPEX precedence. Unknown
  records are generically named resources, never invented depth or HDR meanings.
  Default conversion rejects unmapped vendor resources; explicit sidecars preserve
  every record/header/footer without claiming native vendor Gallery compatibility.
* `remux::import_tracks`, `remove_tracks`, `optimize`, `optimization_support` and
  `track_summary`: explicit track selection/ID mapping, exact movie-timescale
  conversion, offset rewriting, fast-start and opt-in unused-media compaction.
  Existing metadata-only removal remains available. No operation is implicit in
  extraction/composition. Encoded samples are byte-checked after rebuilding.

Optimization relocates media and promotes `stco` to `co64`; it does not decode or
re-encode. Default optimization retains unreferenced mdat bytes. Compaction needs
an explicit option and reports discarded bytes; free/skip/wide padding removal is
reported separately. Import returns source movie-level metadata as **sidecars**,
which callers must publish with the result; it does not attach those properties
silently to a different asset. Destination metadata is retained. Fragmentation,
track references, external data references, unknown offset-bearing structures and
non-exact/overflowing movie-time conversion explicitly fail. Source/aggregate
budgets apply. This bounded support is not a general-purpose MP4 editor.

Google `MotionPhotoMp4FileBuilder::AddPrimaryMp4FileTracks` actually copies a whole
file then deletes `meta` tracks; its generic track-copy call occurs in the Moments
helper. We provide explicit general import because the user retained that scope;
we do not reintroduce Moments construction. Google's `IsOptimizationSupported`
checks specifically `hvc1`; our raw chunk relocation has no inherent hvc1 decode
restriction. The synthetic hvc1 structural test is not a real-codec/device claim.

Validation before the implementation checkpoint: 122 workspace/all-feature Rust
tests passed, clippy with warnings denied passed, five same-input Oplus cases ran
against the built pinned Google facade, and three real H.264/remux cases compared
encoded packet hashes plus rational PTS/DTS/durations with ffprobe. Exact-commit
receipts and fuzz results are recorded separately after checkpointing.

The differential found a reference boundary defect: for a short JXRS footer on a
movie longer than 68 bytes, the pinned provider's fixed-footer fallback removes
60 media bytes even though XMP declares the correct VideoLength. Our parser checks
complete BMFF candidates and retains the full movie. Google memory extraction can
also count a suffix as movie bytes; file extraction is compared separately. These
are recorded differences, not reasons to reproduce truncation in this library.

Remaining: broader provider/corpus/HEIF variants, full metadata engine/registry,
scoring descriptor projection, complete CLI and Java/JNI/Android wrapping, and
native Gallery acceptance. See the complete inventory; this stage does not claim
full ecosystem completion.
