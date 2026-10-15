# Format support

Statuses refer to the contracts below. A vendor name does not establish device support.

| Contract | Read | Write | Structural validation | Original consumer |
| --- | --- | --- | --- | --- |
| Android 1.0 JPEG directory | Real fixtures | Round-trip, media and time comparisons | Tests and malformed cases | Unverified |
| Android HEIF `mpvd` | Real fixtures | Append-only metadata; item and time comparisons | Tests and malformed cases | Unverified |
| Apple JPEG/HEIF + MOV pair | Original iPhone 15 pair and generated fixtures | Pairing metadata; exact timeline; media comparisons | Pairing and supported timed metadata | Unverified |
| Legacy Google MicroVideo | Synthetic contract tests | Android conversion and source copy | Bounds and directory checks | Unverified |
| OPlus ColorOS 15/16 extensions | Original fixtures, including dual streams | Source copy; explicit sidecar conversion | LPEX, topology and boundary tests | Unverified |
| Samsung SEF 106/107 | Original direct and pointer records, including versionless HEIF | JPEG direct SEF with no known time; explicit unmapped policy | Directory and malformed tests | Unverified |
| Xiaomi Android-style Live Photo | One original fixture | Android target and explicit sidecars | Standard contract checks | Unverified |
| vivo Android-style single file | Two original fixtures | Android target and explicit sidecars | Standard contract checks | Unverified |
| Xiaomi portrait/depth; vivo dual/portrait | No original corpus | No dedicated writer | Unknown bytes can be preserved | Unverified |
| HarmonyOS MovingPhoto | Platform API researched | Caller-owned platform integration | No private on-disk contract claimed | Unverified |
| Huawei/Honor private dialect; Android AVIF | No matching original corpus | No dedicated writer | Not claimed | Unverified |

Fixtures do not cover all firmware, modes, codecs or metadata encodings. Strict mode
requires the implemented standard contract or explicit Apple pairing. Compatible
mode can use a valid SEF directory when the Android directory is not usable.
Forensic mode reports recovered provenance and does not invent a timestamp.

The Apple time reader supports the evidenced `mebx` still-time edit-list forms.
Other timed metadata encodings, fragmented movie remux and unsupported edit lists
remain explicit limits. Large files require an increased input budget; streaming
random-access parsing is not implemented.

The repository contains synthetic test vectors and hashes of external originals.
It does not redistribute the external photos or proprietary components.
