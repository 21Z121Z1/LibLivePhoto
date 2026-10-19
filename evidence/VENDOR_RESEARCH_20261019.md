# Bounded vendor research and implementation gaps

This continuation reuses [the existing vendor assessment](VENDORS.md), the
owned fixture manifest and pinned Google providers. Evidence classes are:
**O** original-byte observation; **S** static source inference; **D** official
public documentation; **L** download listing only; **R** actual device/Gallery
acceptance. None of the new work below supplies R evidence. Component versions
and firmware builds remain unknown when the bytes do not identify them.

| Family | Existing implemented subset | New evidence / exact identity | Concrete gap and bounded action |
| --- | --- | --- | --- |
| OPPO / OnePlus | ColorOS15/16 originals, ordered resources/LPEX, OpCamera XMP and bounded JXRS readers; unknown second stream retained | O: five owned fixture hashes in [manifest](../fixtures/manifest.json). L: OnePlus Photos15.30.30 (150300300), `com.oneplus.gallery`, arm64/API30+ [listing](https://www.apkmirror.com/apk/oneplus-ltd/oneplus-gallery/oneplus-gallery-15-30-30-release/oneplus-photos-15-30-30-android-apk-download/). The Package line corrects the earlier mistaken `com.oplus.gallery` namespace/permission attribution. Ordinary container GET failed proxy CONNECT403; no APK identity/hash or static binary result. | Gallery stream selection/checksum semantics remain unresolved. Do not relabel stream2 as depth, infer native writer support, or run an unrelated third-party Camera APK. |
| Xiaomi | Owned Google-style UltraHDR original, namespace/directory/unknown-resource preservation | D: official [client guide](https://dev.mi.com/xiaomihyperos/documentation/detail?pId=2003) and [history](https://dev.mi.com/xiaomihyperos/documentation/detail?pId=2001), SDK1.0.3, client guide updated2026-02-05. SDK retrieval explicitly requires approval/account credentials. | Header parsing and MediaStore recognition are distinct. Keep exact declared microseconds and HDR resources; portrait originals/secondary semantics and native Gallery acceptance are missing. No attempt to bypass the protected SDK repository. |
| vivo | Two owned Android-style originals already parse and preserve bytes | O: both contain terminal `cameralbum!` envelopes,43-byte tails/length2=47, version2202, `vivoMediaEStream` and `vivoMediaExtInfo` markers. S: [VLiveConvert](https://github.com/brovast/VLiveConvert/tree/deec1e7c2470abefad63351247d31f8f04041d2f), GPL-3.0, static reading only. L: Gallery3.2.10.4 `com.vivo.gallery` [listing](https://vivo-gallery.en.uptodown.com/android/download); package not obtained (proxy CONNECT403). | Add bounded raw footer/UUID inspection and explicit consistency diagnostics. Preserve unknown JSON and streams. Both originals reuse one declared ID, so ID equality alone cannot establish a capture pair. `imageTime` remains a raw integer; frame-ordinal semantics are an S hypothesis, not an inferred microsecond time. |
| Samsung | SEF106/107 direct/pointer/inline and HEIF sefd/mpvd readers, limited direct JPEG writer | O: owned originals plus [previous SEF observations](SAMSUNG_SEF_SPEC.md); S: pinned Google and ExifTool. L: [Gallery15.9.00.35](https://androidapks.com/samsung-gallery/com-sec-android-gallery3d/) advertises versionCode1590000035/arm64/API30+, not a verified local package. | `MotionPhoto_AutoPlay` is an observed second encoded movie; consumer selection and native timestamp/writer mapping remain unverified. Current writer is not broadened on listing evidence. |
| Honor | Bounded LIVE20/40 and backward TLV/OfflineData reader, synthetic reference controls | D: official [MediaData Kit](https://developer.honor.com/cn/docs/11032/guides/introduction) and [LivePhotoUtils](https://developer.honor.com/cn/docs/11032/reference/com.hihonor.mcs.media.datacenter.livephoto/LivePhotoUtils) identify dynamic-photo APIs. Reader returned a shell; ordinary container GET failed proxy CONNECT403. SDK/package version and binary SHA remain unknown. | Obtain field-level public API/package evidence and legal original variants before further parser/writer claims. Product dynamic-photo features do not establish LIVE/TLV byte parity or Gallery acceptance. |

## Acquisition and provenance

The user authorized necessary public packages without the previous1GB approval
threshold. Initial free space was24GiB. This pass starts with small source/docs
and component listings; whole firmware is not needed for the first repair.
Proprietary binaries, original media and decompiled source remain outside Git.
Downloads use normal public endpoints; CONNECT403/authentication gates are
recorded without proxy, login, paywall, DRM or security-setting bypass.

VLiveConvert was fetched at the existing pinned SHA above;28 selected
source/license files were retained privately. Footer/MP4/dual/single code and
targeted tests were inspected. GPL code is not copied or translated into this
Apache library. The new contract is based on independent original-byte
observations plus explicit framing facts; synthetic vectors will be separately
authored. The upstream tests refer to an author's private X200Ultra directory;
those originals were not obtained, executed or treated as public fixtures.
The55-byte/length2=2 variant therefore remains S evidence until another original
can verify it. No vendor application or native library was executed.

## CameraAlbum footer contract to verify before implementation

Observed source layout is `prefix | JSON | JSON-byte-count:u32be | cameralbum! |
length2:u32be | identifier-field | FF*4 | 11-byte magic`. Both owned vivo files
have the20-byte `vivoMediaExtInfovivo` prefix, JSON lengths2118/2141, a28-byte
identifier,43-byte tail and length2=47. JSON/raw extents and the full encoded
movie must stay inspectable. Unknown keys/stream payloads retain opaque meaning.

The two `imageTime` values are31 and12. They are not scaled into microseconds;
exact source timestamps continue to come from declared standard metadata and
validated sample tables. HDR/color/orientation metadata is not synthesized from
the footer. Pair IDs must be compared as declarations and checked for conflicts,
with uniqueness and actual pairing left explicit.

Implementation requires positive/negative framing, overflow, malformed JSON,
duplicate-key, identity-conflict, unknown-version and bounded-allocation tests,
plus both independent owned originals. A writer's generated output alone is
insufficient interoperability evidence. Actual Gallery playback/edit/export is
still an external R gate on identified packages/firmware/devices.

The pinned ExifTool `Trailer.pm` at
`2200871d9cef988051d2a99d67df3bda6cbb30a8` was also read: its older Vivo trailer
recognition uses a magic endingB1 and scan-based streamdata/JSON behavior. The
two owned CameraAlbum originals endB3. These are separate observed/static
contracts; B1 support is not inferred from B3 tests. The repository LICENSE is
GPLv3, while the two read Perl modules state Perl's terms individually; no
implementation is linked or copied. Their source hashes are recorded in the
[acquisition ledger](vendor-acquisition-20261019.json).

## Implemented subset and executed evidence

`inspect_camera_album_footer` now exposes original ranges/JSON, version/raw
`imageTime` and declaration-ID agreement. `MetadataEngine` adds the optional
`camera_album` provider and a complete extractable footer block; filtering,
snapshot ranges and borrowed-FD cursor ownership are tested. Unknown JSON is
returned as exact text instead of a normalized generic numeric tree. Known
fields have explicit type checks; duplicate/escaped-alias keys and allocation
or nesting limits fail diagnostically. This introduces no new dependency.

Eight authored tests cover both framed layouts, malformed/unsupported cases,
identity conflicts, raw unknown text/numbers, provider controls and file/range
ownership. The existing explicit public-corpus regression additionally verifies
both independent vivo originals, their actual top-level UUID associations and
footer/movie-byte preservation after build. The55-byte variant remains labelled
static/synthetic. C/JNI uses a separately owned neutral binding vector; that
vector is not substituted for original interoperability evidence.
