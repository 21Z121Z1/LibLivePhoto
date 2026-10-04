# Android reference consumer

This document records static source observations. No Google Photos or OEM Gallery
acceptance test is implied. Production code uses the Android format contract and
independent test vectors; it does not translate this Java implementation.

## Source identity

| Release | Commit | Examined component |
| --- | --- | --- |
| Media3 1.3.1 | `d833d59124d795afc146322fe488b2c0d4b9af6a` | JPEG extractor, XMP parser and directory resource selection |
| Media3 1.8.0 | `b7bbc6e2bc3e45ff3ed99884c114c50f03bba5c9` | Same components |

The sources are published by AndroidX under Apache-2.0. References for 1.3.1 and
1.8.0 are [the earlier JPEG package](https://github.com/androidx/media/tree/d833d59124d795afc146322fe488b2c0d4b9af6a/libraries/extractor/src/main/java/androidx/media3/extractor/jpeg)
and [the later JPEG package](https://github.com/androidx/media/tree/b7bbc6e2bc3e45ff3ed99884c114c50f03bba5c9/libraries/extractor/src/main/java/androidx/media3/extractor/jpeg).

`JpegExtractor.java`, `XmpMotionPhotoDescriptionParser.java` and
`MotionPhotoDescription.java` are byte-identical between those two commits.
The XMP parser SHA-256 is
`ed84cbbb56e2464f217412bb43510b2f8ebbd695edb41a9a043f962f59198263`.
`JpegMotionPhotoExtractor.java` differs between versions; do not infer full
consumer equivalence from the three unchanged files.

## Observed selection contract

- Version 1.3.1 probe requires an EXIF APP1 after the optional JFIF APP0. Version
  1.8.0 accepts an APP1 at that position without the EXIF payload requirement.
  A standard-valid file does not therefore imply acceptance by both versions.
- The default JPEG extractor requests motion video and metadata. An explicit flag
  instead requests the image.
- Motion extraction reads an XMP APP1 packet, requires a known input length and
  checks the selected embedded input with an MP4 extractor before playback.
- The XMP parser accepts MotionPhoto and legacy MicroVideo flags. A flag of one
  is required. Missing time and time `-1` produce an unset value.
- Directory selection works backwards from the input end. A zero-length video
  item can share the preceding nonzero item. The earlier video item replaces a
  later selection. This behavior does not define OEM auxiliary-video semantics.
- The parser recognizes specific textual prefixes. LibLivePhoto resolves namespace
  URIs under the standard contract; it does not adopt this consumer restriction.

These observations have high confidence for the examined source. Runtime playback,
secondary-track switching, portrait use, editing and export remain untested.
The experiment needed for Google Photos or an OEM Gallery must identify that
application's package version and the exact input/output digests.
