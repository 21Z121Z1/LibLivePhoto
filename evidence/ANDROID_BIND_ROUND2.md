# Bounded continuation of Android IPC acceptance

Starting point: `2a8fcbc934d2555adb2ef85f957e6e1d552dcc77`, same environment
and branch. Prior results remain immutable:
[71b778f](platform-71b778f.json) timed out waiting for the first isolated
MetadataParserService; [c15648c](platform-c15648c.json) passed core20 and then
reported an unclassified metadata instrumentation process death. These are
independent failures. Bluetooth crashes are not library-crash evidence.

The first reproduction adds diagnostics, without a claimed functional fix:

* A package-scoped, optional client observer records bind entry/return value,
  callback entry/disconnect/death/null binding, handshake entry/return/error,
  ready-future completion and close/unbind. Each record includes monotonic
  elapsed time, process/thread/UID and whether the callback is on the main
  looper. Default clients install no observer. Acceptance retains64 events per
  connection and emits them after the readiness wait, avoiding extra status
  Binder calls inside a ServiceConnection callback.
* Debuggable-host service traces record create/bind/unbind/destroy, actual
  service PID/UID, Binder caller PID/UID and native-work boundaries. Release
  hosts do not emit these traces. No media bytes, request JSON or identifiers
  enter the new trace.
* Each scope retains package-scoped bound-service and exit-history snapshots,
  selected lifecycle logcat rows and system boot/package/activity states.
  Evidence is captured before uninstall, even when instrumentation dies.

Official [ServiceConnection documentation](https://developer.android.com/reference/android/content/ServiceConnection)
states that callbacks normally use the main thread and that an accepted bind
can still fail to deliver a connected callback. The trace distinguishes those
possibilities from a blocked protocol handshake; neither is assumed in advance.

The original20-second bind,40-second operation,240-second scope and120-second
readiness deadlines remain unchanged. Installed acceptance still requires
core20 + metadata11 + lifecycle16 + concurrency18 =65 assertions and confirmed
owned-APK cleanup. Fourteen host fault tests passed, including retention of
per-scope evidence before cleanup after simulated metadata process death;
these tests do not establish device acceptance.

This continuation permits at most two installed-runtime attempts. The first
collects these missing facts. A second requires a concrete fix or materially
new runtime/source evidence. Repeated system startup/dispatch failures are
classified from actual traces and handed off with a bounded verification plan;
no infinite rerun, timeout increase, assertion removal, permission change or
artifact CONNECT403 bypass is used.
