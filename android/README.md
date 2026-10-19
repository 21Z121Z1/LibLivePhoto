# Android bound services (API 26+)

The AAR packages the maintained Java API, Rust JNI library (`arm64-v8a` and
`x86_64`), AIDL and four non-exported services. `MetadataParserService` runs in an
isolated process; builder, checker and extractor use separate application
processes. No media permission, Internet permission, upload or file publication
is performed. Moments construction is excluded.

Build with an **already installed and licensed** Android SDK/NDK, full JDK 21 and
Rust targets `aarch64-linux-android,x86_64-linux-android`:

```sh
python3 scripts/build_android.py
python3 scripts/test_android.py # only on your authorized test device/emulator
```

The script does not accept SDK licenses. It reports missing paths. The resulting
`android/build/liblivephoto-android.aar` can be consumed as a local AAR by a host
Android application; it includes its service manifest and shrinker JNI rule.
The separately signed acceptance APK uses only the project's synthetic media.
Its generated test key is ignored and is not uploaded. No release is published.

```java
MotionPhotoClient client = new MotionPhotoClient(context, MetadataParserService.class);
client.ready().thenAccept(connected -> {
    // Keep this descriptor open until execute() returns; the host owns it.
    try (ParcelFileDescriptor fd = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)) {
        connected.execute(new Request("metadata"), MotionPhotoClient.Source.whole(fd), null,
            (phase, done, total) -> { /* dispatch UI updates yourself */ })
            .result().thenAccept(result -> { /* owned metadata and original file bytes */ });
    } catch (Exception error) { /* explicit connection/request/I/O error */ }
});
// Close when the host no longer needs this binding. Active operations cancel.
```

Use `MotionPhotoClient.Source.range(fd, offset, length)` for bounded descriptors.
Reads are positional, leave the caller's cursor unchanged and never close the
caller descriptor. A path is opened by the host; in-memory byte inputs can use
`NativeEngine` directly or a host-owned temporary FD. The service limit is
**32 MiB aggregate input**, 64 KiB request JSON, one running and two queued jobs
per service. These are explicit wrapper limits, distinct from core/JVM limits.
Pipes and non-seekable inputs are rejected by range validation. Responses use
LLP1 over a reliable output pipe, with a 160 MiB client response budget; Binder
carries only small request/progress/status values. A result is decoded fully
before the host can explicitly write a new output file. The service rejects a
regular output file, so it cannot overwrite an input by aliasing its descriptor.

Build: `new Request("build").inputKind("parts").target("google-jpeg")`
with two sources and an `ExactTime`. Check: `new Request("check")`. Extract:
`new Request("extract").option("video",true)...`. Metadata parser also supports
`parse`, `metadata`, `classify-metadata`, `xmp`, `agtm` and provider filters in
the same request schema as the Java API/CLI. A service rejects actions outside
its role. `NativeResult.success` and structured metadata contain core failures;
connection, IPC and truncated response failures complete the future exceptionally.
Cancellation stops reads/queued work and discards late results. A synchronous
Rust phase is not interruptible internally. Callback process death cancels work.

See [official AIDL threading/ownership guidance](https://developer.android.com/develop/background-work/services/aidl)
and [ParcelFileDescriptor](https://developer.android.com/reference/android/os/ParcelFileDescriptor).
The build receipt is separate from installed-APK runtime evidence. Emulator
service success does not verify vendor Gallery support, camera capture or
physical-device HDR rendering. No device access or permission bypass is used.

Runtime acceptance runs four independent instrumentations (`core`, `metadata`,
`lifecycle`, `concurrency`). All four must pass. Each retains a 240-second whole
case deadline, 40-second operation deadline and 20-second bind deadline; service
connections are established sequentially. This avoids one aggregate deadline
expiring across eight complete parse/lifetime iterations on a software emulator.
The lifecycle case still performs all eight parses; concurrency additionally
checks three queued pipe readers and two callers racing to cancel one operation.
Per-case logs, failure phase/stack and logcat are retained. SDK/AVD setup or a
partial sequence cannot produce a runtime success receipt.

After boot, the harness waits at most 120 seconds/40 attempts for package and
activity services and a successful, clean `pm list packages` response. Exit224,
timeouts and malformed/error output cannot mean "package absent". It never
restarts ADB or changes system permissions. Each run writes a separate
`android/build/acceptance-*/` directory with command stdout/stderr/exit status,
bounded logcat, scope logs and a receipt, including pre-install failures.
Success requires all 65 assertions and removal of the confirmed owned test APK.
`python3 scripts/test_android_harness.py` injects harness failures without a
device; its success is not Android runtime acceptance.
Failed ADB commands and bounded post-failure service/logcat snapshots also appear
in CI stdout, so a failed artifact transfer cannot hide installation stderr.
Host fault injection suppresses its synthetic service-success markers; only the
installed-device phase and its runtime receipt can establish service acceptance.
Instrumentation phases also travel through status bundles, so the last completed
phase remains visible if the process dies before returning a failure bundle.
The harness retains a separate bounded crash-buffer snapshot and this package's
`dumpsys activity exit-info` before cleanup; unavailable diagnostic commands
remain visible and never imply a pass. It changes no log-buffer sizes or device
permissions. [Android logcat](https://developer.android.com/tools/logcat) describes
the separate crash buffer; the [API35 ActivityManager implementation](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android15-release/services/core/java/com/android/server/am/ActivityManagerService.java)
supports package-scoped exit history. Exit descriptions are diagnostic text, not
a stable machine-readable protocol or an inferred root cause.
The final runtime receipt is also printed to CI stdout, including completed
scopes, exact assertion total and confirmed owned-package cleanup.
