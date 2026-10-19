# Stability gates and remaining acceptance

This is an audit of supported contracts, not a claim that every input, device or
vendor layout is supported. Google remains pinned to
`12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`. The full functional mapping remains in
[GOOGLE_COMPLETE_INVENTORY.md](GOOGLE_COMPLETE_INVENTORY.md); native acceptance
remains in [ACCEPTANCE_CHECKLIST.md](ACCEPTANCE_CHECKLIST.md).

## Boundary fixes in this stability pass

* `MetadataCollection::extract_block` validates every public extent before
  writing. Hand-constructed reversed/out-of-bounds extents return an error with
  no written prefix. Custom provider relative extents receive the same reversed
  range check. `ByteRange::new` remains the normal checked constructor.
* Collection JSON has a pre-allocation aggregate structural/string-token limit
  (262144), in addition to 64 MiB input, 16 MiB decoded raw data, per-array limits
  and serde nesting validation. Quoted/escaped punctuation does not consume
  structural budget; a large opaque value still round-trips unchanged.
* Existing score metadata has limits of 1 MiB and 16384 descriptors per sample,
  including unknown zero-length descriptors. Whole-movie scoring extraction
  allows at most 4096 samples, 16 MiB referenced payload and 65536 semantic
  records across tracks; repeated source references count repeatedly. Budget
  errors retain `InputLimit`. No metadata or Moments construction was added.
  Timed AGTM selection has the same 4096-sample/16 MiB aggregate bounds and
  65536 decoded rule/control-point budget. Tests select genuinely overlapping
  samples using signed composition offsets; a small source cannot expand into
  unbounded wrapper JSON or copied metadata outputs.
* CLI publication prevalidates names/duplicates, uses CREATE_NEW, writes the
  report last, and rolls back only files it created after ordinary write errors.
  Injected failure during the second file verifies removal of both a completed
  first file and partial second file. A concurrent unrelated file is preserved,
  with incomplete cleanup reported. This is not atomic cross-process visibility
  or crash/power-loss recovery. General caller-provided `Write` sinks cannot be
  rolled back; their I/O error and publication policy remain caller-owned.
* Browser reparse keeps the selected local files and explicit association/time
  options. Replacement/cancellation releases object URLs and pending seeks;
  delayed events without an actual current media error cannot invalidate new
  media. Actual HEIC/movie decoder errors and original byte downloads remain
  separately exercised, rather than treating dispatched events as codec tests.
* Android operation/service terminal guards now also handle `Throwable`, so a
  decoder or linkage error cannot silently abandon a result future. Instrumented
  phases, full stack traces and logcat identify actual Binder/FD/pipe failures.
  The service acceptance result itself remains a separate required gate.
  Concurrent cancellation returns true only for the winner of the terminal
  transition. Java memory inputs reject oversize arrays before retaining a copy.
* The native/web inspection manifest shares a 100000-frame projection budget
  across tracks. A second 60000-frame track cannot allocate another full tree;
  its timing projection reports unavailable/limit while original movie bytes
  remain intact. Full media timing is still available through the bounded core
  track API. The Java wire reader separately caps JSON at 1048576 values and
  numeric literals at 1024 characters before BigDecimal allocation, in addition
  to its length/depth/duplicate-key/UTF-8 checks.

## Existing bounded contracts retained

The standards/edit continuation fixes HEIF mirror-axis interpretation and retains
the old meta payload when replacing it with free, because absolute item extents
may reference those bytes. Construction1 extents cannot escape their idat into
another box. The format metadata payload helper preflights all ranges and caps
copied/repeated bytes at16MiB/4096 extents before allocating. AGTM unit-rate edit
selection validates exact conversion/overflow and chooses one sequential edit
before scanning samples, avoiding samples×edits amplification. Raw packet media
times and scoring-payload timestamps remain distinct; unsupported scoring edit
validation, metadata fragmentation and non-unit rates fail explicitly. New
positive/negative bindings are in [NORMATIVE_CONFORMANCE.md](NORMATIVE_CONFORMANCE.md).
The [76c25e4 receipt](standards-edit-76c25e4.json) records171 default/all-feature
tests, an explicitly executed public corpus pair of tests, unchanged XDRemux
contract replay, real C/JNI and three-host packaged consumers, all four Android
scopes and the iOS26.2 Swift/Rust process. The private-original analysis test is
not executed. Local ASan ran1179105 cases; exact-SHA CI ASan ran1541136,
each at60 seconds requested per target/61 seconds actual, with no crash.

| Boundary | Retained limit / ownership | Verification |
| --- | --- | --- |
| File/fd input | Checked nonempty exact range, regular file, overflow/EOF checks; combined caller-configured snapshot budget | Range/borrowed descriptor regression and JVM/Android acceptance; Unix positional read; Windows cursor restoration requires caller synchronization |
| Shared operation/C/JNI | 1 MiB request, combined 128 MiB input; C null/length checks, panic capture, owned response handle; pending JVM exceptions retained | Real C ABI and Java/JNI smoke, repeated lifetimes and cancellation |
| BMFF inspection | 256 tracks and 1000000 samples across tracks, checked table extents, offsets and exact timestamps | Track/media regressions and seeded ASan fuzz targets |
| Remux | Combined 256 MiB import inputs; unsupported offsets/references/layouts rejected; encoded packets verified after rewrite | Six same-input Google/ffprobe packet comparisons, track regression, remux fuzz target |
| Metadata engine | Source 256 MiB, 4096 blocks, cumulative 16 MiB raw manifest bytes, 64 KiB prefixes for large coded/trailer blocks | Raw-block/range/serialization tests and 38-case pinned provider differential |
| Timed metadata extraction | AGTM and score extraction each bound selected/copied payload across tracks to 16 MiB, 4096 samples and 65536 semantic records | Overlapping-interval/descriptor amplification regressions and seeded ASan target |
| LLP1 protocol | 64 MiB header, 4096 files, cumulative 320 MiB envelope; consumers validate fields, UTF-8 and trailing bytes | Malformed/streamed protocol and native consumer tests |
| Inspection / Java JSON | 100000 projected frames across tracks; Java reader 1048576 values and 1024-character numeric literals | Actual two-track manifest regression and JVM flat-tree/huge-number rejection |
| Android service | Combined 32 MiB source, 64 KiB request; one worker/two queued operations; duplicated descriptors, reliable output pipe, callback-death/cancel cleanup | Actual installed APK gate; per-operation bounds are not a universal process-memory ceiling |
| Android client | Two readers/eight queued reads, 160 MiB response cap, input descriptors borrowed and duplicated | Actual installed APK gate; complete envelope required before result publication |

Source byte budgets bound inputs and copied payloads, not all allocator/runtime
overhead. Process memory, caller concurrency and operating-system limits still
matter. Cancellation cannot interrupt synchronous Rust work inside a native
phase; late results are discarded, and native phases remain bounded by inputs.

## Evidence rules

Only executed commands can close a gate. Local logs reside under ignored
`evidence/local/`; checked-in receipts record their command, result, source SHA
or tracked-diff hash, and source limitations. CI must check the final code SHA
on Linux/macOS/Windows and run installed Android and statically linked Swift
Simulator operations. SDK compilation, mocks, a passing browser decoder, and CI
success do not establish native Gallery/Photos or HDR display acceptance.

Permissioned Photos-library import/edit/render, physical playback/camera/HDR,
legal original Honor corpus/native Gallery behavior, and the unavailable Apple
LivePhotosKit SDK fetch remain explicit acceptance gaps. The CDN request was
blocked with proxy tunnel HTTP 403; no bypass or SDK redistribution occurred.
The software Android emulator boots without changing `/dev/kvm` permissions.

Verified baseline [4848e5f](stable-baseline-4848e5f.json) subsequently passed all
three workflows, including all four installed Android scopes/65 assertions and
the original Swift/Rust process on SE3/iOS26.2. The boot monitor completed without
timeout recovery and the owned device was deleted. The compiler used SDK26.5;
the actual runtime identified itself as 26.2/23C54. This does not resolve the
older iOS26.5 startup failures described below. The receipt preserves exact job,
artifact, source-tree and ASan execution details for later regression comparison.

At `a10e5d2`, all four installed Android scopes passed (65 assertions). Apple
passed 11 macOS tests, iOS SDK compilation and XCFramework packaging, but two
fresh iPhone17e/iOS26.5 boots exhausted `simctl bootstatus`'s 300-second monitor
deadline before executing the runtime process. The next harness retains full
boot output, owned-device state and scoped CoreSimulator logs; prefers a
compatible SE configuration; and permits one actual process attempt after a
monitor timeout only when its owned device reports `Booted`. The original
Swift/Rust assertions and 120-second process deadline remain required. Neither
the fault-injection unit tests nor `Booted` state count as native acceptance.
Failure and cleanup receipts persist, and only the created UUID is deleted.
The [7748d04 platform trace](apple-platform-7748d04.json) confirmed compatible
SE3/iOS26.5 still stalled during CoreLocation migration/System App readiness;
even the bounded device-list query timed out, so no process pass was claimed.
Subsequent packaging/acceptance scripts isolate inherited SDKROOT, validate
the target SDK and select an installed iOS runtime >=16 by the deployment floor,
with an explicit runtime override available. No migration, permission or runtime
image is modified to make a test pass. Seven local fault-injection gates cover
recovery failure, stale receipts, SDK selection and runtime compatibility; actual
Simulator execution remains independently mandatory.
