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

## Existing bounded contracts retained

| Boundary | Retained limit / ownership | Verification |
| --- | --- | --- |
| File/fd input | Checked nonempty exact range, regular file, overflow/EOF checks; combined caller-configured snapshot budget | Range/borrowed descriptor regression and JVM/Android acceptance; Unix positional read; Windows cursor restoration requires caller synchronization |
| Shared operation/C/JNI | 1 MiB request, combined 128 MiB input; C null/length checks, panic capture, owned response handle; pending JVM exceptions retained | Real C ABI and Java/JNI smoke, repeated lifetimes and cancellation |
| BMFF inspection | 256 tracks and 1000000 samples across tracks, checked table extents, offsets and exact timestamps | Track/media regressions and seeded ASan fuzz targets |
| Remux | Combined 256 MiB import inputs; unsupported offsets/references/layouts rejected; encoded packets verified after rewrite | Six same-input Google/ffprobe packet comparisons, track regression, remux fuzz target |
| Metadata engine | Source 256 MiB, 4096 blocks, cumulative 16 MiB raw manifest bytes, 64 KiB prefixes for large coded/trailer blocks | Raw-block/range/serialization tests and 38-case pinned provider differential |
| Timed metadata extraction | AGTM and score extraction each bound selected/copied payload across tracks to 16 MiB, 4096 samples and 65536 semantic records | Overlapping-interval/descriptor amplification regressions and seeded ASan target |
| LLP1 protocol | 64 MiB header, 4096 files, cumulative 320 MiB envelope; consumers validate fields, UTF-8 and trailing bytes | Malformed/streamed protocol and native consumer tests |
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
