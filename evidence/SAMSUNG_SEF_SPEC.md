# Samsung SEF behavior specification

This specification defines byte behavior. It does not define Gallery acceptance.

Evidence: seven original files in `public-samples.json`, from Galaxy A34, S20,
S20 FE, S23 Ultra and Tab S9; SEF directory versions 106 and 107.
Independent reference: [ExifTool Samsung](https://github.com/exiftool/exiftool/blob/2200871d9cef988051d2a99d67df3bda6cbb30a8/lib/Image/ExifTool/Samsung.pm).
Confidence: high for the observed offsets; unresolved for autoplay and portrait use.

1. The last eight bytes contain a little-endian directory size and `SEFT`.
2. The directory begins that size plus eight bytes before EOF. Its header is
   `SEFH`, a little-endian version, and a little-endian entry count.
3. Each entry has 12 bytes: reserved u16, type u16, backward offset u32,
   and record size u32. All integers use little-endian order.
4. A record starts at directory start minus backward offset. It contains the
   entry's first four bytes, a little-endian name length, name bytes, then payload.
5. `MotionPhoto_Data` type 0x0a30 contains either a complete BMFF video or a
   12-byte descriptor: `mpv2`, a big-endian absolute offset, and a big-endian size.
6. A `MotionPhoto_Version` record can contain `mpv3` while the data descriptor
   still starts with `mpv2`. These fields must remain independent.
7. HEIF files can carry the video in `mpvd` or in `sefd`. These are observed
   layouts. `mpvd` is also a standard Android layout.
8. The implementation must reject out-of-range, overlapping, inconsistent and
   duplicate primary records. It must retain unknown record payloads and names.
9. No timestamp is inferred from SEF. The parser uses declared XMP time when
   present. A versionless SEF file can have no known presentation timestamp.

The XDRemux JPEG fixtures contain a complete second BMFF payload named
`MotionPhoto_AutoPlay`. The parser exposes it as `UnknownVideo`, with its own
container and tracks. Its name does not establish a generic autoplay,
substitution, depth or portrait role. Gallery selection remains unresolved.

The JPEG writer emits a fresh directory with direct video payload. It retains
unknown records byte-for-byte when the source has an SEF directory. It adjusts
all backward offsets. It does not claim original Camera or Gallery compatibility.
The SEF writer rejects a known timestamp because no target timestamp mapping is
implemented. The Android writers and source copy remain available for that case.
