# Vendor preservation handoff: round5

Implementation: **0ece07a053403801f13bb421dad09a57613a3d4e**, isolated branch
`codex/vendor-preservation-round5-20261019`, based on delivery
474b0dadac2351b4ca3be11958c7c83ac7711b3b. The prior bd6f85b source and474b0da
delivery are retained. No new Codex task, agent, model/tier change, PR, merge,
release, force push, upstream write or XDRemux edit was performed.

## Result

Samsung normal assets already supported a primary plus other complete-video
resources. That code is retained. The new read-only catalogue works when a
primary declaration is absent or duplicated, exposes every structurally
supported video with a SEF record index, and permits explicit byte-exact selection.
Default parsing/selection keeps rejecting ambiguity. Invalid candidates receive
per-record diagnostics; opaque payloads and raw records remain extractable.
Pointer aliases stay distinct declarations and share bounded validation.

vivo already validated UUID/framing/JSON boundaries and exposed opaque streams.
The new audit maps terminal and UUID declarations to actual source indices and
footer/JSON/ID ranges. It merges two views of the same physical footer, retains
distinct copies, reports internal and cross-footer ID conflicts, and checks
directory/legacy XMP media-span declarations for a single appended JPEG source.
Parts/HEIF receive NotComparable for that mapping. No time unit, capture-pair
proof, depth, autoplay/native role or unknown metadata is inferred.

[Coverage/evidence matrix](VENDOR_PRESERVATION_ROUND5.md),
[original range/hash observations](vendor-round5-original-observations.json),
[local verification receipt](vendor-round5-local-verification.json) and
[public-source acquisition ledger](vendor-round5-acquisition.json) are the
reviewable supporting deliverables. The full Google inventory remains an open
work list; this round does not claim complete ecosystem/native compatibility.

## Executed evidence

Local fixed-source verification passed formatting, strict Clippy,241 tests in
each default/all-feature run,2 public-original tests on14 SHA-locked files,
the external consumer and the unchanged actual XDRemux facade replay. Samsung
has11 targeted tests (6new); vivo has7 new declaration tests plus6 existing
framing tests; one new shared-mapper test covers overflow/discontiguous ranges.
Samsung6file names represent4distinct inputs, not6independent captures. Each has
2complete movies; both vivo originals have31opaque records, one physical footer
with two origins, matching standard spans and no measured declaration conflicts.

The new13-case manifest SHA256 is
`f83b4f36ca42622c8eccaa437451e975b2f343b9a4571b8274d5809d46cbe15b`.
Its54 authored field checks,3 default parse refusals and2 exact movie extraction
outputs passed actual local C ABI/JNI, CLI and extracted release CLI/compiled
C/external JAR consumers. The existing42-case syntax suite still passes at each
applicable host entry. FFprobe independently verified8 original packet outputs /
198 packets and exact rational times; explicit compaction removes103 bytes.
The11 binary vendor fixtures also passed the rebuilt AddressSanitizer target
as actual fixed inputs; this is separate from random fuzzing. CI's3 randomized
targets completed45462/1993058/96406 executions (2134926total), peak RSS
458/727/454MiB, with no sanitizer crash.

Fresh CI for this source passed all3 workflows /8 logical jobs in their latest
attempts: [Verify39323820301](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39323820301),
[Apple/Web39323820313](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39323820313),
[Android39323820296, attempt2](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39323820296).
The [terminal platform receipt](platform-0ece07a.json) retains8artifact metadata
digests, including the first failed Android attempt's separate artifact; these
are GitHub-reported hashes, not a claim of local artifact downloads.

Linux/macOS passed241 and Windows240 in each default/all-feature run. All3 hosts
executed the new13 C/JNI and extracted release CLI/compiled-C/external-JAR cases,
alongside the existing42. macOS Swift12, arm64 iOS26.2/23C54 linked process42,
Node's4 existing tests plus its42-case WASM test, and actual Chrome155.0.8059.84's
8 tests passed. The iOS build SDK26.5/Xcode26.6 is distinct from actual26.2 runtime;
the owned simulator was deleted. Android x86_64/API35 passed115 assertions in all
5 scopes, all42 individually gated syntax IDs/digest, and20 real remote-PID /
ready-wait records (3499..17058ms); installation and removal were confirmed.
Original20-second future /40-second operation /240-second scope /120-second
readiness budgets are unchanged.

Android's first attempt failed **before any assertion**: APK installation returned
224 / package-service Broken pipe, followed by DeadSystemException/system died.
Installation outcome is uncertain and removal unconfirmed on that terminated
runner. Its package-absence flag was a pre-install readiness observation, not a
post-failure cleanup check. [First-attempt receipt](vendor-round5-android-attempt1.json)
retains those exact facts. Only the failed job was retried once on the same source;
the successful run does not establish the first failure's root cause. No timeouts,
KVM/device permissions, network configuration or production Android code changed.

The13 new cases execute on desktop host bindings/SDK consumers. Mobile, Swift
and WASM keep their existing suites and count; their runs are regression evidence,
not execution of these13 cases or device Gallery acceptance.

Fixed Google12fb3cb provider differential:38 cases passed. Its test executable
reports99 passes, including the previously documented HEVC-encoder early-return
case. Initial local setup attempts missed the protobuf include and the reference
test's relative-fixture cwd; corrected commands use the existing sysroot and a
fresh cwd with the upstream lib input symlink, without upstream changes.
Expected namespace-alias/ordinary-HEIF declaration differences remain in the
receipt; differential success does not assert that every Google output is correct.

The older broad facade script is **partial**: its first5 original inputs passed;
it then assumed a synthetic Honor20 rewrap could always rebuild and stopped at
the existing PhotoTimeMismatch guard. The preserved, extracted bd6f85b SDK and
this source both refuse that identical input. Remaining broad-script cases were
not executed in this run. The harness is unchanged; no default metadata deletion,
fabricated cover time or relaxed preservation contract was added to make it green.

The local112-file SDK contains47 separate dependency notices; SHA256
`3c62dbd3e9d17dd0bdf7da374665e1e908cb17d42b60692b03f740de061f7e6c`.
Its manifest truthfully reports untracked evidence during packaging: only the new
original-observation JSON was present and tracked diff SHA256 is empty. Source
is0ece07a; all extracted release consumers passed. The earlier bd6f85b SDK was
copied intact to `dist/round4-bd6f85b` before packaging.

## Integration and migration

For an ordinary parsed Samsung asset, keep using `resources()` / `extract()`
with its asset-owned ResourceId. For a missing/ambiguous primary, call
`inspect_samsung_videos(snapshot)`, inspect records/diagnostics, then
`inventory.select(Some(record_index))`. Returned `bytes` are borrowed original
movie bytes; `record_payload(index)` / `raw_record(index)` retain opaque data.
No name-based default is selected. The catalogue accepts a byte snapshot, not a
new path/fd or writer API. Its bounds are256MiB source,4096records,64distinct
candidate ranges and64MiB aggregate unique candidate bytes. Codec decoding,
fragmented/index/reference rewriting and arbitrary vendor/native writing remain
outside this structural selection contract.

For a parsed asset, call `vivo_declaration_report()` and handle None/error
separately. `footers[*].source_index` and ranges refer to the immutable source
snapshot, not the enclosing OS-file offset. `origins` describes aliases;
`identifier_comparisons` does not count independent evidence or unique captures.
Raw JSON retains unknown fields/numeric lexemes. XMP raw bytes stay in their
source resource. Audit conflicts do not change core media validity. The existing
metadata operation exposes `vivo_media.value.declarations` and
`samsung_video_inventory`; the latter never sets a motion declaration or verified
video extents. Provider filters remain effective. New public structs are
non-exhaustive; existing SamsungMetadata fields were not changed.

The earlier `AacConfiguration` source migration remains: old exhaustive literals
require `channel_configuration` and `program_config` from the real ASC/PCE.
Do not insert invented default channel facts. Existing reads and `..` patterns
remain usable. Its known E0063 reproduction is retained in the round4 receipt;
the original9-file XDRemux facade/source/example/test replay freshly compiles
and its unchanged interop test passes using a temporary local-path override.
XDRemux's repository/pin and full application are unchanged/unbuilt.

## Blockers and remaining boundaries

Ten ordinary public HTTP page/handoff requests failed before response bodies
with `<urlopen error Tunnel connection failed: 403 Forbidden>`; the ledger names
every exact target. This is a confirmed connection rejection, not pending user
approval, an uncertain transfer or an automatic approval-review denial. No APK
body was obtained (0bytes, measured package hashes null). Eight exact package /
version listings and13 actually retained UTF-8 tool-text/excerpt hashes exist;
those excerpts are neither raw HTML bodies nor APKs. Page-advertised APK hashes
are labelled separately. Prior CONNECT403 addresses were not retried or bypassed;
network/access controls were not changed and no unknown binary was executed.

Honor Camera/Gallery and OnePlus Camera versioned package identity remain
unresolved by bounded sources; HUAWEI/OPPO is not substituted. Existing legal
originals and pinned static-source facts support the delivered contracts, with
no new dependency or redistribution of proprietary source/media.

vivo imageTime units/stream roles, Oplus checksum/second-stream purpose, Honor
legacy/native variants, wider firmware/corpus mapping, native Gallery/Photos
recognition/edit/export, physical HDR/colour/audio rendering and Android arm64
device execution remain unverified. No new vendor writer is opened. Full ICC /
AAC semantics and other previously recorded core limits remain open. The saved
libheif environment/worktree was left untouched. This authorised round ends at
delivery; another round is not automatically started.
