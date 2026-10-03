# Extraction evidence

The source baseline is XDRemux GitHub main `c24c8eb`.
The candidate is `4a8568f5842b47a51787966d40e8b27c2dcd8aa3`.
The candidate adds a reader facade, two time/identifier fixes and runtime integration.
The current main has the same underlying Motion Photo parsers as the candidate base.
LibLivePhoto uses these parsers and the two fixes. It replaces the candidate asset model.
The candidate merges physical layout with legacy and recovery provenance.
It also exposes `OppoMetadata`, `android_asset()` and a guessed geometry role.
These fields are not part of the LibLivePhoto stable facade.
The existing replay branch contains source imports only. It has no facade or writer plan.

Baseline checks:

- Candidate: 44 unit tests, two real-fixture tests and 12 facade tests pass.
- Current main: 42 unit tests and two real-fixture tests pass before migration.

The generic format subset contains cursor, error, FourCC, JPEG segments, EXIF and BMFF.
Codec profile inspection, HDR processing, platform code and filesystem publication remain in XDRemux.
The `xdremux-compat` feature provides temporary migration interfaces.
The default feature set excludes those interfaces from the public API.
