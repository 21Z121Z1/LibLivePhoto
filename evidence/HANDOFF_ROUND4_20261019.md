# Round 4 cross-platform acceptance handoff

Frozen code: `bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7`, branch
`codex/google-core-coverage-20261019`. This extends candidate PR1 at
`794ab552596db0f66095600d23cf7937a631b9ec`; main/PR1/XDRemux remain unchanged. Google
reference stays `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`. No new Codex
task/model/tier change, merge, release, force push, upstream message, original/private
research overwrite or preinstalled libheif/config change occurred.

This round adds validation and minimal WASM forwarding of the existing LLP1 operation
protocol. It does not extend codec/colour/vendor/Apple semantics. The core parser
implementation is unchanged from round 3. A target-specific wasm-bindgen dependency
edge reuses locked 0.2.104; no new dependency version.

## Exact source and reviewable evidence

All three terminal workflows and eight jobs succeeded at the frozen code SHA:

| Workflow | Run | Result |
| --- | --- | --- |
| Hosts/corpus/ASan | [39315561148](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39315561148) |5 jobs passed |
| Apple/Node/actual Chrome | [39315561134](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39315561134) |2 jobs passed |
| Installed Android | [39315561406](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39315561406) |1 job passed |

[Terminal platform receipt](platform-bd6f85b.json) contains step results, every case
group, Android's 42 individual IDs and 20 connections, all seven artifact metadata
digests, retained first failures and 13 source-object roots. [Detailed local
evidence](syntax-cross-platform-details-bd6f85b.json) contains 198 packet rows, 26
actual pinned-reference comparisons, original consumer file hashes, the demonstrated
source-compatibility failure and local release receipt. [Contract and source
locations](SYNTAX_CROSS_PLATFORM.md) and [fixture
manifest](../fixtures/syntax/cases.json) bind the exact tested subset.

The final evidence-only documentation commit has no source/test/config/fixture changes
relative to this verified code. Its 13 recorded Git objects are checked against the
code SHA. The CI claims belong to that code SHA, not a claim that a later
documentation commit independently reran every platform. Implementation commits contain no CI-skip marker. The first a8a6dff and final bd6f85b
snapshots ran full CI. The intermediate `266d2b9` repair is included in the final
tested snapshot; no separate `266d2b9` run is claimed. The documentation-only delivery may skip redundant CI while retaining
this explicit distinction.

The seven digests describe GitHub upload artifact ZIPs, which can contain an SDK ZIP
and its own .sha256/receipt files. They are not interchangeable with the inner SDK ZIP
digest. The local Linux release SDK's own SHA256 is
`b86ca91a52d665d2f4e4a9a2a2815b82cce55b6ab9116feaaa1e74a7969c43e6`. All packaged SDK
consumers actually extracted and exercised their corresponding release archives; no
body-download or transport bypass is inferred here.

## Newly executed vectors

`fixtures/syntax/cases.json` has 42 owned positive/negative cases; canonical raw
SHA256 is `0fc2be1b1b309f03d638cd5655007e93d162061da706ceaf5b80ef1cf4970226`. They
validate ICC selected 4.4 structure/Unsupported/malformed/budget results, AAC-LC
PCE/wave and existing direct esds fields, strict descriptor diagnostics, stz2/legacy
stsz preserve/compact/import/remove, mixed-table rejection, indexed AGTM bytes,
default preservation of uninspectable AAC, exact 1/10 photo time, video sample
times1/16 and 3/8, and explicit 1/3 refusal. Each relevant entry compares
original/golden output bytes and classified results. ICC input is the
extracted/reassembled raw profile, rather than an entire JPEG/HEIF image. A container
can still inspect successfully with unavailable AAC descriptions plus diagnostics;
explicit conformance/capability checks do not infer compatibility from that parse
success. Goldens establish transport parity; an independent reader verifies 8 movie
outputs / 198 packets: encoded SHA256, rational DTS/PTS/duration and mdat-index
target. A real 103-byte unreferenced payload survives preserve mode and is removed
only by explicit compact mode. Rust additionally exercises widths 4/8/16, raw
stts/ctts, legacy tables, metadata/import/remove paths and explicit capability-policy
Unmapped.

## Terminal evidence

* Hosts: Linux/macOS 227 and Windows 226 per default/all-feature run; 3 ignored
  defaults. Two public-original tests run separately with 14 verified source files;
  private-original analysis remains unrun.
* Each host: real C ABI and JNI 42; extracted release CLI 42, compiled external C
  using packaged header/library 42, external Java compiled against packaged JAR 42.
* macOS Swift 12 tests, including 42 vectors; real arm64 iOS 26.2/23C54 process 42,
  original time/movie/AGTM/error assertions retained, owned simulator deleted.
  Compiler SDK 26.5 / Xcode 26.6 is separate from the actually executed 26.2 runtime.
* Node: original 4 timing tests plus actual WASM 42-case test. Actual Chrome 155.0.8059.39:
  original 7 browser tests plus new 42-case test (8 passed); GET/HEAD only.
* ASan: asset 80,671 / headers 3,462,127 / remux 180,637 = 3,723,435 executions; peak RSS 468 / 685 / 459 MiB,
  no crash. This is a bounded campaign, not a proof of defect absence.
* Android: all five scopes passed 115 assertions (original 65 plus syntax 50:
  42 actual vectors and eight connection checks). All 42 case IDs were individually
  reported in manifest order with the exact digest; host gates require those
  reports, not just a total count. Installation and owned-package cleanup passed.
  Twenty real service connections retain client/remote PIDs, actual UIDs,
  ready-future waits 2993..19685 ms and handshakes 8..237 ms. Parser processes are
  PIDs 1960/2740/3143/3527/3845 with isolated UIDs 99000..99004. Existing 20-second
  future waits begin after synchronous bind returns; 240-second scopes include
  that bind call. Original 40-second operations and 120-second readiness stay.
  The 16 host harness tests are fault injection, separate from actual device results.
  First a8Android also independently passed 115 / cleanup with 20 PID/wait records;
  its weaker case-report gate cannot replace this final strict execution.

The first source `a8a6dff87bf03dfa69e98f08bdbdfb384b6a42a7` had real failures: Windows
C compilation rejected fopen deprecation under-Werror; Chrome's original 7 tests
passed but new 42 did not execute because .mjs was served as octet-stream with
nosniff. Commit `266d2b9` uses local Windows fopen_s, adds correct JS MIME and pins
the manifest LF. It retains `-Wall` / `-Wextra` / `-Werror`, nosniff and all
assertions. Final `bd6f85b` CI actually passed both repaired entries. That same commit
adds visible Android per-case IDs/hash gates and reproducible original-consumer source
checks. No historical Android timeout/process-crash root cause is inferred from
passes.

## Reproduce and integrate

```
cargo test --locked --workspace --all-targets
cargo test --locked --workspace --all-targets --all-features
python3 scripts/test_c_abi.py
python3 scripts/test_java.py
python3 scripts/package_native.py
python3 scripts/test_packaged_sdk.py dist/liblivephoto-linux-x86_64-sdk.zip
python3 scripts/test_syntax_contracts.py
python3 scripts/consumer_replay.py --xdremux-repository /path/to/read-only/XDRemux --check-consumer-sources
npm ci --prefix web --ignore-scripts
npm --prefix web run build
node --test web/tests/syntax.test.mjs
BROWSER_EXECUTABLE=/path/to/google-chrome npm --prefix web run test:browser
```

Full JDK21, Rust, a C compiler and ffprobe are used where required; web builds
additionally need the wasm32-unknown-unknown target and wasm-bindgen-cli0.2.104. The
hosted workflows contain their actual platform setup. Syntax CLI/independent packet
checks are now a required Ubuntu CI step. For Swift, CI sets both native-library and
syntax-fixture paths; iOS uses its real linked process script. Android builds an owned
APK/AAR and executes all five scopes without changing original deadlines. Production
Binder service allowlists stay unchanged: conformance/remux use real in-app JNI;
parse/build/extract/AGTM use actual Binder FD ranges. An arm64 Android build is not an
arm64 device run.

The WASM operation adapter is generated into web/dist/operation-pkg and
operation-node. `process_operation(requestJSON, primaryBytes, movieBytes)` returns the
same owned LLP1 bytes as C/JNI, forwarded to the existing Rust implementation. It adds
no browser filesystem publication API. Original inspection/UI module stays separate.

`AacConfiguration` adds `channel_configuration` / `program_config`. Old complete
struct literals demonstrably fail E0063; read-side fields/methods and destructuring
with `..` remain source compatible. A separate consumer workspace tests existing
fields on real direct AAC and PCE files. The 9 original XDRemux
facade/source/example/test files compile with an explicit local-path dependency
override preserving xdremux-compat; its unchanged interop test passes through the
actual facade. Its repository and pin are not changed, and the complete XDRemux
application is not built.

## Remaining scope

ICC is a structural subset, without full profile semantics/ID/LUT/rendering. AAC
remains bounded declaration inspection: non-LC/extended or SBR-present PCE,
QuickTime-v2 and full bitstream/rendering certification remain unsupported. The legacy
non-PCE declaration path retains later raw syntax without interpreting it. Unknown
reference-bearing MP4/fragmented/indexed/external layouts remain rejected.
Gallery/Photos import/edit/HDR/capture, physical Android arm64/iOS devices and other
runtime versions are unverified. iOS 26.5 startup history remains separate. No new
vendor writer or legal original Honor corpus has been supplied. Unknown streams remain
opaque; sidecars do not establish native album interoperability. Known
ISO/PDF/LivePhotosKit/saved-artifact CONNECT403 transfers were not retried or
bypassed. These do not prevent the completed scoped format/transport tests.

All legal-source/fixture provenance and separate dependency notices remain in existing
ledgers. FFmpeg/ffprobe are development tools. No researched ICC/MPEG/QT/ vendor
implementation or FFmpeg executable is copied, translated or linked into production;
ordinary locked dependencies retain their individual notices. No complete ecosystem or
100% or native Gallery/Photos certification claim is made. This closes the authorized
round without automatically starting another.