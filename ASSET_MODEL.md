# Asset model

`MotionAsset` separates layout, format, dialect and provenance. Sources contain
bytes. Resources refer to sources. Containers refer to resources. Tracks belong
to containers. A track ID is the `tkhd` identifier; its index is its order.

`ResourceId(0)` is the still and `ResourceId(1)` is the primary motion video.
Other IDs identify auxiliary and unknown views. A child resource can overlap its
enclosing parent. Independent top-level extents must not overlap. Shared-data
relationships represent a zero-length Android directory item. Extraction copies
the referenced bytes in extent order, including multi-extent HEIF items.

Unknown bytes remain represented as resources or as bytes of an enclosing resource.
JPEG application metadata and HEIF items are independently extractable. SEF records
retain raw vendor names. Unclaimed source ranges retain headers and padding.

`MediaTime` stores a signed integer and a nonzero timescale. Exact rescaling uses
integer arithmetic. A convenience float is not the writer's source of truth.
MOV movie timeline fields can be rescaled to represent a fractional source tick.
The media timeline and encoded samples remain unchanged. Overflow, unsupported
edit rates and unsupported fragmented timeline rewrites cause explicit errors.

Track timing includes the media timescale, duration and edit list. Supported
nonfragmented sample tables are checked for count consistency, arithmetic overflow
and extents inside `mdat`. A track without `stsz` receives header inspection only.
Fragment sample placement is not validated.
The validator does not decode samples or establish codec compatibility.

Generic roles include primary, secondary, substitution, autoplay, depth, portrait,
vendor auxiliary and unknown video. The Android 1.0 two-video contract declares the
second track as substitution. OPlus concatenated stream 2 retains an unresolved
vendor auxiliary role. Samsung `MotionPhoto_AutoPlay` retains `UnknownVideo`.
Xiaomi `depthMotionPhoto` has no evidenced generic mapping in the available corpus.

Default limits: 256 MiB input, 64 MiB recovery tail, 128 recovery candidates,
4 MiB XMP, XML depth 128, 64 directory items, 256 tracks, one million samples,
4096 extents per HEIF item and 65536 extents per `iloc`. Recovery never chooses
between multiple complete videos without metadata. File sizes beyond the configured
input budget are rejected before reading or allocation.
