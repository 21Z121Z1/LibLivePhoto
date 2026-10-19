# Round3 handoff: ICC, AAC/PCE/wave and stz2

Tested source: **7cc8f3316fd0031f574edfdb31ba2f331372a995**, same
`codex/google-core-coverage-20261019` branch. Production code originates at
3f226918a30f9745560088ef9e61625625bc697b; the only subsequent code-tree change
is an existing regression's stricter missing-PCE expectation. [Exact receipt](syntax-contracts-7cc8f33.json)
binds all source objects, command/log hashes and the earlier probes' matching
production/fuzz/fixture/wrapper objects. Final evidence/root-documentation edits
do not constitute a new execution. The all-platform4d02be4 baseline remains
preserved and is not reused as acceptance of the changed code.

## Implemented contracts

1. **ICC.1:2022 v4.4 selected envelope.** `inspect_icc_structure` borrows raw
   profile/tag bytes and reports profile-relative extents. `validate_icc_structure`
   and the existing `conformance` request's `icc-structure-v4.4` profile check
   selected fixed-header fields, exact length, bounded table/extents, identical
   offset+size sharing, alignment, partial overlaps, gaps, padding and truncation.
   Unknown/private tag bytes survive unchanged; a different version fitting the
   envelope is Unsupported. This is not full date/class/PCS/required-tag/profile-ID/
   LUT validation or colour accuracy. Default raw ICC preservation is unchanged.
2. **AAC-LC PCE and bounded QuickTime wave.** ChannelConfiguration0 now exposes
   declared SCE/CPE/LFE counts, raw group/tag/mixdown/data/coupling/comment facts
   and original ASC. Associated data/coupling does not add output channels.
   Duplicate channel tags, profile/rate conflicts, truncation and descriptor/
   byte budgets fail explicitly. Supported GA has no core-coder dependency or
   GA extension; an explicit no-SBR suffix is recognized. Non-LC/SBR-present/
   unfamiliar trailing forms remain unsupported. Wave inspection accepts entry
   versions0/1, one esds, matching frma and a final null marker, with an optional
   zero mp4a marker. Conflicting/direct+nested descriptors and unknown children/
   esds versions are rejected. Entry-v2 remains unimplemented; raw preservation
   does not establish compatibility.44100 is retained exactly; the Google
   page's44kHz ambiguity still produces Unsupported rather than normalization.
3. **One additional index layout: stz2.** Official MPEG ISO14496-12 edition7/2022
   machine-readable syntax and reference implementation were fixed before the
   index change. [Invariants](SYNTAX_ROUND3.md) require unique stsz-or-stz2,
   version0/flags0,4/8/16-bit sizes, exact counts/packed extents, self-contained
   chunk targets and unchanged exact sample times/bytes. Existing packet/frame,
   import/removal/fast-start/compaction paths share the new bounded size view.
   Segment indexes, fragmented/external/reference-bearing unknown combinations
   remain rejected. Existing unknown-box disposition policy is retained.

The [source/API/test map](SOURCE_TEST_MAP.md), [integration guide](../CORE_INTEGRATION.md)
and [CLI contract](../CLI.md) expose these scopes. Rust `AacConfiguration` gains
channel_configuration/program_config fields; callers constructing struct
literals must account for them. The maintained C ABI/request version is unchanged
and JSON audio facts are additive. Malformed/uninterpretable descriptions now
also receive an explicit per-resource Unmapped reason in strict capability plans.
No encoder or implicit repair was added to production paths.

## Actually executed

| Layer | Result and precision |
| --- | --- |
| Linux Rust |223 default and223 all-feature tests, zero failures; three ignored per run. Twelve new tests cover ICC3/AAC6/stz2 three. No existing test was removed or newly ignored. |
| Original corpus | Both explicit public-fixture tests passed,14 SHA-locked files checked. Private-original analysis remains unrun. |
| Actual C/JNI | Linux shared-library calls and JDK21/Java8-compatible client passed: ICC valid/unsupported/truncated/budget checks, real PCE/wave facts, unknown esds version rejection and byte-identical MOV through build/extract, alongside existing ownership/lifetime/cancellation tests. |
| Actual CLI |12 calls passed. A real owned two-track MOV was converted to stz2; every one of17 packets was checked with ffprobe for SHA256, exact demux DTS/PTS/duration and its actual indexed file extent before/after both optimizer modes. |
| Packet/time invariants | Rust separately compares raw stts/ctts/stz2, independently enumerated DTS/CTS and payloads across4/8/16-bit indexes, import/removal and unknown-box failures. The real input already has referenced-only media, so explicit compact and preserve modes can yield identical outputs. |
| AddressSanitizer |50673 asset_input +102392 remux =153065 executions,61 seconds each, zero crashes; peak459/452MiB. Fresh isolated corpora included owned ICC, PCE/wave and stz2 seeds. container_headers was not rerun. |
| Targeted reference |20 timed-selection plus six MP4 same-input cases passed against Google12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3, with expected differences retained. No new99-test/reference-encoder claim. |
| Consumer | XDRemux's unchanged public-contract test passed at0688ea7d6270df52cc5968e2c36dd016f13bd31b, source SHA256 aa3a154f0da02aa1bf79566f7439b2129b56f36dfc0a73c2a58ef225fb1b437b. The consumer repository and dependency remain unchanged; no full product build claimed. |
| WASM | wasm32-unknown-unknown bridge compilation passed; no browser runtime execution. |

Full [packet/probe receipts](syntax-contract-details-7cc8f33.json) retain execution
SHAs. ASan/reference/consumer/WASM ran at3f22691, whose measured production objects
match7cc8f33. The first3f default run found the old test expecting an Unsupported
report for a2-byte ASC declaring PCE with no PCE data. It now explicitly asserts
the parser's known truncation error; other unsupported/duplicate assertions remain.
The final full/default/all-feature suites pass without weaker rejection gates.

Per impact, **no fresh Windows/macOS/Android/iOS/browser or packaged-release SDK
runtime CI was requested**. Those tests/workflows remain intact. Android65 plus
cleanup and other platform successes belong to exact4d02be4 history, not this
implementation.71b778f's initial readiness timeout and c15648c's metadata process
death still have unresolved distinct root causes. No mobile lifecycle fix or
new physical Gallery/Photos/HDR/capture acceptance is claimed.

## Provenance and open boundaries

[Source/license ledger](syntax-round3-sources.json) fixes ICC4.4, FDK2.0.3,
FFmpeg n7.1 primary reader/writer and official MPEG syntax/reference snapshots.
The latter syntax is a committee-maintained extraction; full ISO AAC/BMFF
normative bodies were not acquired. Apple QTFF body reading was unavailable,
so the wave claim is the explicitly tested primary-implementation wire subset.

Five normal container transfers remain proxy CONNECT403-blocked, without retry,
proxy/auth changes or package access bypass. Existing authorized public cloud
source-reading tools supplied inspectable ICC/GitHub facts; decoded source text
is private, hashed and outside Git. No researched implementation was copied,
translated or linked into production. Source licenses are recorded individually;
new production dependencies:0. System FFmpeg/ffprobe7.1.5 is a GPL-enabled
development tool used only on repository-owned neutral video plus silence;
neither it nor third-party source is redistributed. Four owned vectors reproduce
exactly with that same recorded tool build, not every encoder version.

[Remaining work](REMAINING_WORK.md) includes full ICC/AAC semantics/rendering,
non-LC/extended PCE/QT-v2 and other unsupported remux layouts, alongside actual
claimed-platform acceptance. Samsung time/AutoPlay, Oplus checksum/second-stream,
vivo time/roles and legal original Honor corpus remain gaps; no new APK/native/
firmware, vendor writer or new vendor capability is inferred in this round.

Only the owned repository branch is deliverable. No merge/release/force push,
new Codex task/model/tier change, upstream message/write, original/private-research
overwrite or preinstalled libheif/saved-environment change occurred. This closes
the authorized bounded round; no automatic next round is started.
