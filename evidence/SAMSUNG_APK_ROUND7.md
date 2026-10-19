# Signed Samsung package observations and bounded regression controls

Base delivery:272879c52ee60542754f0fdfbc3c24ccfdb7a196. The production Rust,
bindings, fixtures and workflow configuration remain those of verified03d28dc.
This round adds two owned synthetic tests and factual research evidence; no
vendor implementation or APK body is committed, and no writer is added.

## Acquisition and identity

The [acquisition ledger](vendor-round7-acquisition.json) distinguishes ordinary
container requests, connector metadata, page claims, Git objects and actual APK
bodies. Twelve ordinary probes include three returned text responses and nine
proxy CONNECT403 failures before origin TLS/body. GitHub HTML/Git, PyPI and
Debian downloads work, so this is not a complete egress outage. The precise
policy and the refused origins' availability remain unknown. No previously
refused URL is retried; no proxy, resolver, CAPTCHA, permission or network
configuration is changed. No external runner is used to fetch refused targets.

Two real APKs are obtained from the independently accessible public
[Samsung A55 firmware repository](https://github.com/RandomPush/samsung_a55x_dump/tree/609a8ec6ec3b9c8bb1e4a13199272269725f4fe2)
using an ordinary filtered/no-checkout Git clone and `git show` of pinned blobs:

| Measured package | Version name / code | Bytes | Measured APK SHA256 | min / target SDK |
| --- | --- | ---: | --- | --- |
| `com.sec.android.gallery3d` |15.7.00.43 /1570000043|51208290|13256b229fb21525f226d909aeaaedbec04aa0202290455743e992a2eaa5b960|30 /36|
| `com.samsung.android.motionphoto.app` |5.0.46 /504600000|3981375|61914b7a3eff06b8a8a4af28bd5e4f6cd20bd4f36cf216937b0f111f9a8d73d3|34 /36|

Total vendor APK bytes:55189665. These are different artifacts from the proposed
15.9.00.45 Gallery listing; that listing's file hash/size is not a match claim.
Both downloaded files are actual ZIP/APK bodies, not HTML, metadata or Git-LFS
pointers. Native libraries in their ZIPs are arm64-v8a; no native library or APK
code is loaded, installed or executed.

Debian apksigner35.0.2-1 verifies both APKs successfully using **v3**. Its v1,
v2, v3.1, v4 and SourceStamp verification results are false. A parser's detection
of old signature-block structures is not verification of those schemes. Each
has one signer, measured certificate SHA256:

`34df0e7a9f1cf1892e45c056b4973cd81ccf148a4050d11aea4ac5a65f900a42`

The certificate matches the fingerprint advertised by the supplied
[Samsung Gallery mirror listing](https://www.apkmirror.com/apk/samsung-electronics-co-ltd/samsung-gallery/samsung-gallery-15-9-00-45-release/samsung-gallery-15-9-00-45-android-apk-download/).
That comparison is a mirror declaration, not an independently authenticated
official trust anchor. Signature integrity is verified; official OTA provenance
and the repository's firmware identity remain **unverified**. A self-declared
certificate DN alone does not authenticate the publisher.

Measured `build.prop` says SDK36/release16, incrementalA556EXXSDCZB5 and
security patch2026-02-05. The README's fingerprint instead contains Android14 /
UP1A.231005.007, while its headline says16. The ledger retains both claims and
their hashes; it does not resolve that inconsistency by guessing a device build.

## Version-scoped static observations

Androguard4.1.3 reads DEX data only. The
[method/DEX identity receipt](vendor-round7-static-observations.json) records
class, descriptor, code offset and bytecode SHA256 for each factual observation.
Full disassembly, package bodies and raw research remain in ignored private
storage. No vendor implementation is copied into the library or public evidence.

| Observation | Actual static route | Library implication / boundary |
| --- | --- | --- |
| Separate primary and AutoPlay records | MotionPhoto5.0.46 `MPMergeTask.addSEFInfo` takes `autoPlayVideoPath` for type2611/0x0a33, `MotionPhoto_AutoPlay`; it separately adds its video as type2608/0x0a30, `MotionPhoto_Data`. | Type and exact name are a pair. The existing parser/inventory already preserves and selects them separately. This does not identify all auxiliary streams or their image semantics. |
| Separate readers | `MotionPhotoParser.parseJpegSefTail` /`parseHeicSefTail` and the separate `parseJpegAutoPlaySefTail` /`parseHeicAutoPlaySefTail` validate their respective type/name and yield a source offset and length. | No missing-primary fallback is justified. Explicit inventory selection still requires a complete validated movie. |
| Gallery has an explicit selection condition | Gallery15.7.00.43 `module.motionphoto.MotionPhotoUtils.getVideoStreamInfoFromMotionPhoto` checks `VideoReqInfo.isMotionSefPlay`. When true, it tries the separate AutoPlay offset/length and returns that non-null result; otherwise or when absent it calls the primary-video helper. | This is static bytecode evidence for this version's conditional selection. Assignment of that request flag, feature/device gates and actual playback remain unverified. The library's raw-label catalogue and explicit record-index selection retain both streams without a generic `UnknownVideo` role or an implicit fallback. |
| Standard photo time uses microseconds | MotionPhoto5.0.46 `MotionPhotoVideoUtils.getCaptureTimestampUs(ExifInterface)` reads the Google Camera namespace's `MotionPhotoPresentationTimestampUs` and parses a long without another scale conversion. | Existing XMP microsecond parsing is retained. Absence remains unknown in this library; the observed helper's fallback0 is not copied. No vivo `imageTime` unit is inferred. |
| Shallow identification differs from validation | Its `isSupportedMotionPhoto(File)` checks a positive selected length and `ftyp` at selected offset+4. | Library inventory still validates the complete bounded BMFF and a video track. A header match is not full format, codec or Gallery compatibility. |
| Version constants are not layouts | MotionPhoto5.0.46 declares format enums through V4/`mpv4`. | Enum presence supplies no V4 byte-layout proof. No V4 support or native writer is claimed. Version records and `mpv2`/`mpv3` data descriptors remain independent. |

These observations are specific to the two measured APKs. The app's system
framework dependencies, feature gates and physical playback are not exercised.
No checksum, vivo time-unit, Oplus second-stream or Honor native-layout gap is
closed by a Samsung symbol or call route.

## Regression scope and reproduction

[Two authored tests](../crates/liblivephoto/tests/samsung_apk_contracts.rs) use
generated neutral movies and JPEG/XMP/SEF bytes, with no vendor material:

* Crossed type/name declarations stay separate, explicitly selectable unknown
  movies; neither becomes the default primary. A correctly matched third record
  is selected and the other original byte streams remain extractable.
* Missing XMP time, explicit0 and123457microseconds remain distinct, with exact
  integer timescale1000000 and the original movie bytes preserved.

The existing syntax42/vendor13/selection20 manifests are unchanged. Production
sources and public signatures are unchanged; the full03d28dc eight-job platform
receipt remains the evidence for those binaries. This round's new tests are
executed on Linux; they are not new Android/iOS/Chrome execution claims. The
[final local receipt](vendor-round7-verification.json) binds commands, results,
test-source identity and unchanged production roots to the committed checkpoint.

Actual static verification command, once the two Debian tool packages are
extracted into the separate research toolchain:

```sh
java -cp /workspace/toolchains/apksig-round7/sysroot/usr/share/java/apksigner.jar:/workspace/toolchains/apksig-round7/sysroot/usr/share/java/apksig.jar com.android.apksigner.ApkSignerTool verify --verbose --print-certs research/private/round7/packages/SamsungGallery2018.apk
java -cp /workspace/toolchains/apksig-round7/sysroot/usr/share/java/apksigner.jar:/workspace/toolchains/apksig-round7/sysroot/usr/share/java/apksig.jar com.android.apksigner.ApkSignerTool verify --verbose --print-certs research/private/round7/packages/MotionPhoto.apk
cargo test --locked -p liblivephoto --test samsung_apk_contracts -- --nocapture
```

The APKs remain proprietary; public availability is not a redistribution
license. Apksigner's inspected copyright and Androguard's separately checked
v4.1.3 LICENCE-2.0 are Apache-2.0. Their analysis environment is isolated and not
bundled in the SDK; no single license is imputed to its complete dependency set.

OPPO/OnePlus, Xiaomi, vivo and Honor original packages remain unobtained in this
round. The accessible Oplus port repository explicitly excludes proprietary
blobs and contains port patches, so it is not substituted for an original APK.
Previously refused targets, including the parent's Honor4PDA403 result, remain
unretried. No APK body, decompiled implementation, private media or credentials
is staged or uploaded. No merge, release, force push, upstream message, new
Codex task, model/tier change or automatic next round is performed.
