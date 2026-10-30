# Explicit record selection and source range contracts

The existing13 vendor cases and42 syntax cases are unchanged. This separate20
case suite adds9 successful original-byte selections,10 explicit refusals and
one vivo footer-alias/source-coordinate check. The fixtures are owned neutral
media and independently authored framing; they are not original-device evidence
or a native Samsung/vivo writer.

MovieA is the repository's own H.264 demo; movieB is its existing CC0 AGTM test
container. Their different full bytes catch selection of a wrong/default stream.
Two pointer records deliberately alias one physical movie; a separate ambiguous
directory has two distinct primary payloads. Expected source ranges distinguish
equal bytes at different positions. The vivo case retains exact raw JSON and
source ranges, without a guessed time unit or playback/stream role.

The shared LLP1 extract request uses `samsungRecordIndex` as a decimal string,
for example `{"action":"extract","samsungRecordIndex":"0"}`. It performs the
existing bounded read-only Rust inventory/explicit selection, copies the original
validated movie and never repairs/reassigns the asset, transcodes, or changes
native semantics. It is exclusive with another input, resourceIds, still/XMP
outputs, time, or strict asset-parser flags. Normal parsing and default extraction
still reject ambiguity. Android uses its existing extractor service and unchanged
allowlist; the new acceptance suites have no JNI fallback.

`../contract-suites.json` locks the manifest digest and exact ordered IDs at all
actual entry points. Fixture hashes, source coordinates and every returned movie
byte are checked. Generate into a **new** directory to reproduce:

```sh
python scripts/create_vendor_selection_fixtures.py --out /tmp/new-selection-fixtures
```
