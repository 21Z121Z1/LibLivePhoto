# Round6 acceptance closure

Implementation: **03d28dcafa5dc6c7a1163b2dbb673ee1cc2c605b**, on the existing
`codex/vendor-preservation-round5-20261019` branch, based on delivery10db9d8.
The production core/guard tree `crates/liblivephoto` is identical to0ece07a;
no scoring/time protection, native writer, transcode or unknown-role mapping was
changed. Changes close the reference-harness and platform-vector gaps, plus a
narrow explicit extraction selector in the existing shared operation interface.

## Complete reference differential

[Exact-source local receipt](round6-local-verification.json) records all10 planned
inputs executed through pinned Google12fb3cb file/memory/fd-range parsing,
file/memory extraction and the applicable rebuild checks. Seven builds succeed;
three fixed synthetic Honor inputs reject rebuilding with the exact existing
PhotoTimeMismatch error. Reading/extraction still runs for all3 rejections. There
are16 explicitly checked expected reference differences and0 real failures.

The allowed rejection tuples are exact name + SHA256 + complete error text,
including both original track4 and5 conflicts. Changed hashes, unrelated names,
extra/different errors or unexpected output fail the guard tests. Honor20's
baseline evidence remains in round5; the preserved bd6f85b SDK was also checked on
the40-byte and TLV inputs during this round, with the same refusals. No metadata
track is removed to make them build. The positive40-byte Honor input declares
500ms, matching the original metadata. Its full rebuilt movie remains byte-exact,
including scoring metadata; the reference recognizes the rebuilt output.

The harness keeps per-case failures, continues independent later cases, writes a
terminal receipt and exits unsuccessfully for real failures. Known MIME/time and
Honor memory-extractor tail differences have explicit checks, not an Honor-wide
exception or an assumption that all reference outputs are correct. AVIF's later
build/reparse/independent FFmpeg decode also executes. The unchanged pinned
reference test executable reports99 passes, including its known HEVC-encoder
early-return case; this does not establish HEVC encode coverage.

## Locked platform contracts

The original42 syntax and13 vendor manifests are byte-identical to round5:

| Suite | Cases | Exact manifest SHA256 |
| --- | ---: | --- |
| syntax |42|0fc2be1b1b309f03d638cd5655007e93d162061da706ceaf5b80ef1cf4970226|
| vendor |13|f83b4f36ca42622c8eccaa437451e975b2f343b9a4571b8274d5809d46cbe15b|
| vendor-selection |20|2ab5c19cc207d9c9d5445ee5cc3047d1bac4a3926b2973bcde8f339a68280600|

`fixtures/contract-suites.json` requires each complete ordered ID list and digest.
Actual runners check every field and returned byte before emitting per-case
reports. Android and iOS host receipts additionally require the actual process's
ordered case IDs/digests, so totals alone cannot substitute for execution.

The separate20-case suite adds9 original-movie selections,10 refusals and one
vivo alias/source-range/raw-JSON check. Two different full movie payloads catch
wrong/default selection; two equal byte copies at different locations catch
wrong source extents; two pointer records aliasing the same physical payload
retain separate declarations and the default parser still refuses ambiguity.
Metadata tracks and opaque payloads remain in the selected original bytes.

The existing `extract` action accepts `samsungRecordIndex` as a decimal string,
for example `{"action":"extract","samsungRecordIndex":"0"}`. It reuses the
bounded read-only inventory/selection before full-asset parsing. It emits one
original validated movie, its source extent and uninterpreted wire label/type;
it neither repairs/reassigns the asset nor supplies a native playback role.
Source limits remain the shared128MiB input and inventory64 distinct ranges /
64MiB aggregate unique candidate bytes. Index/range/format errors and unsupported
records produce failure with no media output. The selector is exclusive with
another input, resourceIds, still/XMP output, time and strict asset-parser flags;
third-party disable controls still apply. Normal parse/default extract is unchanged.

Android uses the existing isolated parser for metadata/parse and the existing
extractor service for selection, through actual FD ranges at an enclosing-file
offset13. The expected coordinates remain relative to the snapshot. There is no
JNI fallback for the new vendor suites and no production service allowlist change.
The old5scopes /115assertions are retained; a new41-assertion scope requires all
13+20 cases plus8 actual connection assertions. Original20/40/240/120-second
future/operation/scope/readiness budgets and40-minute job limit are unchanged.

## Executed verification

Local source03d28dc passed formatting, strict Clippy,242 tests in both default and
all-feature runs, public-original2 tests with14 SHA-locked inputs,18 host harness
fault-injection tests,2 reference-guard tests, unchanged XDRemux facade replay,
actual C/JNI, independent CLI packet/time checks and extracted release CLI /
compiled-C/external-JAR consumers. The original consumer repository/pin is
unchanged and the full XDRemux product is not built.

All desktop entries and Node execute42+13+20, with exact IDs/digests and11
returned original movie outputs across the vendor suites. The112-file local SDK
has47 separate dependency notices, source03d28dc and a clean-tree manifest;
SHA256 `6210f857d5f509f7d3c869b5ad25ff0be8826f916735f05d1583913a7b6c2ca3`.
The round4 and round5 SDKs remain intact in separate dist subdirectories.

The [terminal platform receipt](platform-03d28dc.json) binds all three workflows /
eight jobs to exact source03d28dc, each on attempt1:
[portable/original/ASan](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39331001138),
[Apple/web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39331001070),
[Android](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/39331000997).
All jobs completed successfully. Android API35/x86_64 executes all six scopes /
156assertions, explicitly retaining115 legacy assertions. Its raw per-case log
matches all42 syntax and33 vendor IDs in locked order, including both vendor
manifest digests. All33 new operations have remote Binder PID traces:9metadata,
3parse and21extract. The24 connection records have distinct client/remote PIDs
and ready waits3196–15933ms. Installation is confirmed, its outcome is not
uncertain, and owned-package removal is confirmed on this runner. This is a
separate successful attempt from both round5 outcomes below.

Completed desktop runs have Linux/macOS242 and Windows241 per default/all-feature
run, with3 ignored defaults and originals executed separately. macOS Swift13
tests and actual arm64/iOS26.2/23C54 execute all3 suites; owned simulator deletion
is confirmed. Xcode26.6 / compiler SDK26.5 is distinct from that actual26.2 runtime.
Node's4 original timing tests and3 actual WASM suites pass. Chrome executes10
tests including all3 suites with GET/HEAD only. Three bounded ASan targets ran
105950/2313992/206428 executions, total2626370, peak RSS473/750/468MiB, no crash.
These are bounded regression checks, not physical Gallery or renderer acceptance.

Representative executed commands (toolchain paths are environment-specific):

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets
cargo test --locked --workspace --all-targets --all-features
cargo test --locked -p liblivephoto --test fixtures -- --ignored --nocapture
python3 scripts/test_google_differential_contract.py
python3 scripts/google_differential.py --reference-source /workspace/libmotionphoto-reference --reference-build /workspace/reference-build-make
python3 scripts/test_android_harness.py
python3 scripts/external_consumer.py
python3 scripts/test_c_abi.py
python3 scripts/test_java.py
python3 scripts/test_syntax_contracts.py
python3 scripts/test_packaged_sdk.py dist/liblivephoto-linux-x86_64-sdk.zip
```

The workflow links and receipt retain actual native-runtime scope, job IDs,
per-entry case counts, lock hashes and decoded-log hashes. CI artifact digests
are explicitly GitHub-reported; those artifact payloads were not downloaded or
locally hashed. The separately built/extracted/tested local SDK hash above is a
local file hash. The evidence-only delivery commit leaves tested code, fixtures,
workflow configuration and interface sources identical to03d28dc.
The reference's99-test run invokes
`/workspace/reference-build-make/bin/motion_photo_tests` from a fresh scratch
directory whose `lib` symlink points at the pinned source's test inputs; neither
the Google source tree nor its samples are changed. Its exact log hash and the
known HEVC early-return limit are in the local receipt.

## Prior Android outcomes and hard evidence blockers

Round5 attempt1, source0ece07a/job118304402380, returned224 / package-service
Broken pipe during APK installation, followed by DeadSystemException/system died.
It executed0assertions; installation outcome is uncertain and cleanup unconfirmed
on the terminated runner. Its absence flag predates installation. Round5's
attempt2/job118309926045, on the same source, separately passed115 assertions
and confirmed installation/removal. [The original receipt](vendor-round5-android-attempt1.json)
and [round5 platform receipt](platform-0ece07a.json) remain unchanged. Later passes
do not establish or claim a fix for that first failure's causal root.

No new APK request, prior CONNECT403 retry/bypass, network/KVM/device permission
change, upstream message, merge, release, force push, new Codex task/agent or
model/service-tier change occurred. The saved libheif environment/worktree and
original/private research/media remain intact.

vivo imageTime units and stream roles; Oplus checksum/second-stream purpose;
Honor originals and legacy/native variants; authenticated package/component /
firmware mappings; physical Gallery/Photos import/edit/export and HDR/colour /
audio rendering remain hard evidence gaps. Existing public package pages are
not APK bodies. Those resource gaps are not closed by CI, and no new writer or
unsupported semantic inference is introduced. The round ends after the requested
acceptance closure; no further round is automatically started.
