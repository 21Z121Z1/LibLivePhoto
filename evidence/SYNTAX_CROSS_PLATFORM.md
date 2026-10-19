# Round4 acceptance contract

This round validates the round3 shared parsing changes. It adds no codec/colour
semantics, vendor writer or Apple feature. The shared native operation protocol
has one minimal WASM export so Node and the actual browser can exercise the same
existing conformance/remux operations, rather than reimplementing them in JS.
The prior inspection WASM/UI stays on its original module. The only dependency
graph change reuses the already locked wasm-bindgen0.2.104; no new dependency
version is introduced.

The42-case owned fixture manifest is `fixtures/syntax/cases.json`, SHA256
`0fc2be1b1b309f03d638cd5655007e93d162061da706ceaf5b80ef1cf4970226`.
It requires complete results on each entry, including ICC valid/Unsupported/
malformed/budget, AAC-LC PCE/wave and normal direct esds, strict-version/duplicate/
extension diagnostics, malformed PCE, mixed stsz+stz2 rejection, actual103-byte
discard, stsz and stz2 preserve/compact/remove/import, stz2 AGTM extraction,
default-policy preservation of malformed descriptors, exact1/10 photo time,
video sample times1/16 and3/8, and the unchanged1/3-microsecond refusal.

Core Rust regressions additionally cover stz2 widths4/8/16 and legacy stsz with
real unreferenced media, exact raw stts/ctts, per-sample bytes and index targets,
track/metadata removal and mixed tables. Strict codec policies reject newly
uninterpretable esds with per-resource Unmapped; the default policy still
preserves the complete movie. Existing direct non-PCE AAC only interprets the
old declaration header and retains later raw syntax; it is not a complete
GASpecificConfig/extension conformance checker. ICC remains a selected structural
subset, without full profile semantics, transforms or rendering certification.

| Actual entry | New vector requirements |
| --- | --- |
| Three host C ABI |42 cases via real dynamic Rust library |
| Three host JNI |42 cases plus unchanged host regressions |
| Three extracted SDKs |42 each through extracted CLI, compiled external C against packaged header/library, external Java compiled against only packaged JAR |
| macOS Swift |new XCTest through actual dynamic C ABI; original tests stay |
| iOS Simulator |same42 cases in real Swift process linked to static Rust, original runtime/time/extract/AGTM/error assertions stay, owned simulator deleted |
| Android |original four scopes65 assertions plus new syntax scope50 (42 cases and eight connection assertions), total115; actual JNI for conformance/remux, existing Binder services for parse/build/extract/AGTM using FD ranges and cursor checks; original deadlines/PID logs/cleanup stay |
| Node WASM |new actual shared-operation export, all42 cases; old four timing tests stay |
| Chrome WASM |new actual shared-operation test, all42 cases; old seven browser/UI/media tests stay; GET/HEAD only |
| CLI independent reader |old12 CLI calls plus42 manifest cases and eight outputs/198 packets with independent ffprobe SHA256, rational DTS/PTS/duration and file-index targets |

`scripts/test_syntax_contracts.py` is a required Ubuntu portable-CI step and emits
a source-SHA-bound receipt, including the packet facts and tool version. Other
entry tests print every executed case and the manifest hash where supported.
The hosted workflows must complete against a frozen code SHA before a fresh
cross-platform acceptance claim is made. Historical4d02be4 evidence is retained
as historical and cannot certify these new shared parser paths.

`AacConfiguration` gained public `channel_configuration` and `program_config`
fields in round3. Existing read-side methods/fields and destructuring with `..`
remain source compatible and are exercised by a separate Rust workspace.
Old exhaustive struct literals require the two added fields; full Rust source
compatibility is therefore not claimed. XDRemux remains read-only and at its
existing pin; only its unchanged public-boundary test is replayed externally.
This is not a full XDRemux product build or automatic pin update.

No physical native Gallery/Photos acceptance, full colour/AAC rendering,
unsupported remux reference contract, additional vendor or original Honor
corpus claim follows from these transport tests. Historical Android readiness
timeouts/process crashes remain causally unresolved; new passing runs do not
retroactively explain them.

## Executed result

Fixed source **bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7** completed all three
workflows/eight jobs successfully. [Terminal receipt](platform-bd6f85b.json),
[detailed packet/reference/consumer facts](syntax-cross-platform-details-bd6f85b.json)
and [handoff](HANDOFF_ROUND4_20261019.md) record every listed actual entry.
The first source a8a6dff failed Windows C fopen warnings and Chrome.mjs MIME;
fopen_s and the correct JS MIME repaired them while retaining-Werror/nosniff.
The first a8a6dff and final bd6f85b snapshots ran full CI; the intermediate repair is included in the final tested snapshot. Android's final gate requires all42 individual
case IDs/hash in addition to115assertions and owned-package cleanup.

The actual original XDRemux facade's9source/example/test files also compile in
an external copy using the explicit local-path dependency override and its
original xdremux-compat feature; its unchanged interop test passes through that
facade. No original consumer source/repository/pin is edited, and the full
XDRemux application is not built. Use `--check-consumer-sources` to reproduce.
