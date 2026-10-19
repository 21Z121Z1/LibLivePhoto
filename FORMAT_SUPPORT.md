# Format support

Statuses refer to the contracts below. A vendor name does not establish device support.

| Contract | Read | Write | Structural validation | Original consumer |
| --- | --- | --- | --- | --- |
| Android 1.0 JPEG directory | Real fixtures | Round-trip, media and time comparisons | Tests and malformed cases | Unverified |
| Android HEIF `mpvd` | Real fixtures | Append-only metadata; item and time comparisons | Tests and malformed cases | Unverified |
| Android AVIF `mpvd` | Synthetic contracts and one generated AVIF/reference comparison | Append-only metadata; unchanged encoded item/video/time | Reference reparse and independent still decode | Unverified |
| Apple JPEG/HEIF + MOV pair | Original iPhone 15 pair and generated fixtures | Pairing metadata; exact timeline; media comparisons | Pairing and supported timed metadata | Unverified |
| Legacy Google MicroVideo | Synthetic contract tests | Android conversion and source copy | Bounds and directory checks | Unverified |
| OPlus ColorOS 15/16 extensions | Original fixtures, including dual streams | Source copy; explicit sidecar conversion | LPEX, topology and boundary tests | Unverified |
| Oplus OpCamera/JXRS provider layouts | Synthetic same-input reference cases; HEIC declarations distinguished from video | Source copy or standard target with explicit unmapped-resource policy | Plain/short/checksum/JSON-directory framing and typed fields; checksum bytes retained without claiming verification | Unverified; original LPEX corpus does not establish these variants |
| Samsung SEF 106/107 | Original direct and pointer records, including versionless HEIF | JPEG direct SEF with no known time; explicit unmapped policy | Directory and malformed tests | Unverified |
| Xiaomi Android-style Live Photo | One original fixture | Android target and explicit sidecars | Standard contract checks | Unverified |
| vivo Android-style single file / CameraAlbum43 footer | Two original fixtures; bounded raw footer/JSON inspection | Android target and explicit sidecars; no native footer writer | Standard contract plus exact footer ranges/bytes and declared-ID consistency | Unverified; imageTime unit and capture-pair uniqueness unresolved |
| CameraAlbum55 footer | Synthetic/static contract only | No native writer | Exact framing, limits, raw JSON and malformed declarations | Unverified; no original-device corpus |
| vivoMediaEStream / vivoMediaExtInfo UUIDs | Two independent originals,31 opaque streams each; ftyp-first contiguous snapshots with bounded ordered extraction and footer framing | Original movie bytes retained; no native UUID writer | Stream/directory/framing extents, sizes, duplicate-key reporting and malformed/budget checks | Unverified; keys/tags are not decoded stream roles or native-album evidence |
| Xiaomi portrait/depth; vivo dual/portrait | No original corpus | No dedicated writer | Unknown bytes can be preserved | Unverified |
| HarmonyOS MovingPhoto | Platform API researched | Caller-owned platform integration | No private on-disk contract claimed | Unverified |
| Honor JPEG `LIVE_` 20/40-byte footer and backward TLV | Synthetic contracts and same-input reference comparisons | Source copy or explicit sidecars; no native Honor writer | Bounded records, time, ambiguity and malformed cases | Unverified; no original Honor corpus |
| Other Huawei/Honor private dialects | No matching original corpus | No dedicated writer | Not claimed | Unverified |

Fixtures do not cover all firmware, modes, codecs or metadata encodings. Strict mode
requires the implemented standard contract or explicit Apple pairing. Compatible
mode can use a valid SEF directory when the Android directory is not usable.
Forensic mode reports recovered provenance and does not invent a timestamp.

The Apple time reader supports the evidenced `mebx` still-time edit-list forms.
Other timed metadata encodings, fragmented movie remux and unsupported edit lists
remain explicit limits. Large files require an increased input budget; streaming
random-access parsing is not implemented.

Google's Oplus JXRS checksum/JSON-directory cases and pure OpCamera XMP have
bounded synthetic/reference coverage, independently of the original ColorOS
LPEX corpus. Checksum semantics and native Gallery compatibility remain open.
No Oplus native writer or semantic role for stream2 is added. See the
[current source/test map](evidence/SOURCE_TEST_MAP.md) and
[five-vendor evidence matrix](evidence/VENDOR_RESEARCH_20261019.md).

The repository contains synthetic test vectors and hashes of external originals.
It does not redistribute the external photos or proprietary components.
