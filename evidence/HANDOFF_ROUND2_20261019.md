# Round2 handoff: bounded native acceptance and vivo extraction

Exact executed code: **4d02be46804c13c19657bbd6d865342f6be816b7** on
`codex/google-core-coverage-20261019`. All three workflows/eight jobs completed
successfully. [Terminal receipt](platform-4d02be4.json) records source objects,
job IDs, seven artifact digests and read stdout hashes. Final delivery commits
only update evidence/root documentation; they do not claim a new execution.
The earlier successful7affb28 baseline is preserved separately. PR1 remains
draft/open at794ab552596db0f66095600d23cf7937a631b9ec; main is unchanged.
Google remains12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3.

## What changed

Existing Core/Formats/Remux, Apple support, exact time, lossless media and
plan/execute/resource-disposition contracts remain. New work is incremental:

* Android optional client diagnostics distinguish bind entry/return, callback,
  synchronous protocol handshake, future completion and unbinding. Debuggable
  services record real PID/UID and lifecycle/native-work boundaries. Per-scope
  system/service/exit snapshots survive failures before owned-APK cleanup.
  Default clients have no observer; release hosts emit no new service traces.
* Read-only vivo EStream/ExtInfo UUID inspection exposes ordered opaque records,
  borrowed raw payloads and movie-relative framing/directory ranges. Metadata
  provider `vivo_media` maps full extractable blocks back to the original file
  snapshot through existing memory/path/fd/range and C/JNI request contracts.
  Duplicate keys and unknown encoding tags survive; no last-wins mapping,
  stream-role/depth/portrait, imageTime unit or unique-pairing inference exists.
* Inspection requires a complete contiguous ftyp-first BMFF snapshot, unique
  known top-level UUIDs, at most4096 streams and16MiB aggregate stream bytes.
  Framing/count/length/aggregate bounds validate before exposing results.
  Existing collection/input/footer limits apply separately. Malformed optional
  metadata is diagnosed while original trailer bytes remain extractable.
* Six authored regression cases, two independent existing originals with31
  records each and actual C/JNI opaque-byte tests exercise the new extraction.
  The owned neutral coded test vector reproduces alongside five unchanged old
  vectors. Synthetic framing is not a native vendor writer or Gallery proof.

## Actually executed

| Gate | Exact4d02be4 result |
| --- | --- |
| Rust default and all features | Linux/macOS211 each; Windows210 each; zero failures, three ignored per run. Windows excludes one Unix-only FD test. |
| Public originals | Two explicit fixture tests;14 SHA-locked files checked, including both31-record vivo originals. Private-original analysis test remains unrun. |
| Google same-input comparisons |85: provider38, AGTM12, score9, timed selection20, MP4six. [Full receipts](differential-4d02be4.json) preserve asserted differences and independent interval judgments. |
| Pinned Google test binary | Framework reports99 passed, including CLI_HeicBuild's explicit normal early return when HEVC is unavailable. Its custom framework calls that OK. HEIC dry-run did execute; no reference HEVC pixel-encoding coverage is claimed. |
| AddressSanitizer |2114027 actual executions across asset_input69552, container_headers1903446, remux141029;61 seconds each, zero crashes. Separate source-matched local asset95736 remains recorded independently. |
| Native SDK consumers | Actual C ABI/JVM JNI, external Rust, extracted release CLI/C ABI/external Java JAR and manifest hashes passed on all three desktop hosts. New vivo opaque bytes are exercised by C/JNI. |
| Android | Actual API35/x86_64 install; core20 + metadata11 + lifecycle16 + concurrency18 =65; confirmed owned-APK removal. Fourteen host fault tests are separate from those65 assertions. |
| Apple |11 macOS tests, SDK26.5 compilation and actual SE3/iOS26.2 (23C54) Swift plus statically linked Rust; boot and owned-device cleanup confirmed. |
| Browser | Four Node tests and seven actual Google Chrome155 tests passed; no LivePhotosKit runtime acceptance. |
| XDRemux consumer | Unchanged public-contract test replay passed at0688ea7d6270df52cc5968e2c36dd016f13bd31b, source SHA256 aa3a154f0da02aa1bf79566f7439b2129b56f36dfc0a73c2a58ef225fb1b437b. No consumer edit or full product-build claim. |

The reference's unavailable encoder is a test-environment limit. Source audit
establishes its HEIF RGB/re-encode design; LibLivePhoto retains its independently
tested original-item lossless path instead of introducing pixel re-encoding.

Both permitted installed-runtime attempts passed (7affb28 and4d02be4); no third
attempt was started. At4d02be4 all16 remote PIDs were reported directly, with
future waits4161..14829ms and handshakes12..290ms. Service traces independently
show parser PIDs1918/2781/3192/3597 with isolated UIDs99000/99001/99002/99003.
The20-second wait is the unchanged `ready().get(20)` after synchronous bind
returns; the240-second scope timer includes that call.7affb28's23.043-second
trace point includes time before the future wait starts and is logged after
future completion. Bounded logcat can omit earlier lifecycle rows.

These passes do not explain the prior failures:71b778f's first isolated future
timeout/zero assertions and c15648c's metadata `Process crashed` after core20
remain distinct unresolved causes. No timeout increase, assertion removal,
permission adjustment or Bluetooth attribution made either run pass.

## Vendor provenance, integration and remaining work

[Round2 matrix](VENDOR_RESEARCH_ROUND2.md), [source hashes/licenses](vendor-round2-acquisition.json)
and [independent stream facts](vendor-vivo-stream-observations.json) separate
original-byte observations from static inference and listing-only claims.
Selected public source text totals245065 bytes; new APK/native/firmware bodies0.
Source snapshots are not authenticated SDK/APK versions or license grants.
No decompiled/GPL implementation was copied, translated, linked or executed;
new dependency count is0. OnePlus listing identity was explicitly corrected to
com.oneplus.gallery; an Oplus code namespace is not a package identity.

Use [integration contracts](../CORE_INTEGRATION.md), [source/API/test mapping](SOURCE_TEST_MAP.md),
[coverage inventory](GOOGLE_COMPLETE_INVENTORY.md) and [SDK guide](../NATIVE_SDK.md).
Library APIs intentionally retain their own Rust/JSON/C/JVM design. AGTM/raw
auxiliary extraction is not rendering; preserved sidecars are not native-album
support. Unknown media/metadata remains original bytes with explicit errors and
resource-disposition reports where mapping is impossible.

[Remaining work](REMAINING_WORK.md) still includes AAC PCE/QT-v2/wave and full
codec/ICC semantics, non-unit/dwell/reverse/fragmented/external/indexed remux,
wider native vendor writers and legal original Honor corpus. Physical
arm64/Gallery/Photos/edit/capture/HDR, iOS26.5 process startup and LivePhotosKit
runtime are separate external gates. No board is labelled100% compatible.

Earlier container transfers of the91 Android artifact, supplied ISO text and
official LivePhotosKit SDK remain proxy CONNECT403-blocked. No blocked transfer
was retried/bypassed; no missing body was read or assumed. Existing authorized
ISO section reads remain usable. New implemented work has no pending approval.

Only the owned repository branch was pushed. No merge/release/upstream comment,
new Codex task, model/service-tier change, upstream/XDRemux write, original/private
corpus overwrite or saved libheif environment change occurred. Raw original
media, proprietary research bodies and credentials remain outside Git. This
finishes the authorized round; no automatic next round is started.
