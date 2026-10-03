# Architecture

`liblivephoto-format` owns bounded cursors, errors, FourCC, JPEG segments, raw EXIF
and BMFF structures. It has no dependencies. XDRemux can re-export this boundary
for codec modules that use the same primitives. Do not duplicate these parsers.

`liblivephoto` owns the asset facade, Android XMP contract, Apple pairing metadata,
resource extraction, structural validation, conversion plans and vendor extensions.
`vendor/oplus` interprets LPEX and concatenated-stream differences.
`vendor/samsung` implements the independent SEF byte specification.
Both use the common container parsers.

Inputs are borrowed bytes or bounded file reads. The parsed object owns immutable
source views. The model has no file publication, network access or platform handles.
Conversion plans borrow the parsed object. A caller cannot clear plan blockers.
Output is an in-memory resource set with preservation and disposition reports.

The library does not decode compressed pixels or video samples. It does not choose
HDR processing, codec policy, stabilization policy, Vision inference or Gallery
publication. XDRemux owns these product decisions and atomic filesystem operations.
Apple geometry metadata is accepted only as explicit caller-supplied values.

Metadata rewrites preserve container offsets where possible. HEIF rewrites append
new metadata payloads and an active `meta`; the original media locations stay valid.
JPEG rewrites adjust MPF relative offsets. MOV remux keeps media data at its original
offsets and appends timed metadata and an active `moov`.

The raw EXIF writer redirects structural pointers without decoding unknown entries.
Apple pairing replaces the active MakerNote. The conversion report exposes the old
EXIF as unmapped; explicit sidecar policy retains its original bytes. A near-capacity
JPEG EXIF can require a minimal active EXIF plus that sidecar.

Research specifications contain field behavior, provenance, limits and unresolved
questions. Production adapters depend on these specifications and independent test
vectors. Proprietary binaries and decompiled source are not production source.
