# Maintained Java/JNI interface

Build the isolated native boundary with `cargo build -p liblivephoto-native`, then
run `python3 scripts/test_java.py` using a full JDK 21 (compiles `--release 8`).
Java 8-compatible sources use only public APIs; Android currently targets API 26+
for the file/path/channel surface. Actual Android builds/execution are separate
from the host JVM result.

```java
NativeEngine engine = new NativeEngine();
NativeResult parsed = engine.parse(Input.path(path));
NativeResult built = engine.build(Input.bytes(still), Input.bytes(movie),
    new ExactTime(13, 16), "google-jpeg");
```

Set `-Dliblivephoto.native.path=/absolute/path/to/the/library` or configure the
normal platform library search path for `livephoto_native`. Methods expose actual
parse/check/extract/build/metadata/AGTM capabilities; `extractMetadata` copies
complete source blocks with raw/coded-content labels and a cumulative budget.
`Request` + `execute` exposes
provider filtering, explicit independent parts, Apple pair IDs, sidecar/drop policy,
selected resources, track import/removal and optimization. Request JSON follows the
shared `liblivephoto-interface` protocol. No Google Java/protobuf ABI compatibility
is promised. There is no Moments construction method.

`Input.range(FileChannel, offset, length)` is the descriptor/range contract: use
the channel belonging to your public FileDescriptor/FileInputStream or Android
ParcelFileDescriptor adapter. Reads are positional and bounded, and never close
a caller-supplied channel. A path input opens/closes its own channel. Retain the
original descriptor for the operation lifetime and synchronize external changes.
No reflection into private FileDescriptor fields or guessed raw descriptor is used.

Outputs are owned byte buffers with name/MIME/resource identity and an exact JSON
report. `data()` is read-only; `copyBytes()` returns a new array. `writeNew` refuses
to overwrite existing paths. Success includes a valid check result; consult
its semantic result (`valid`, `samples`, plan blockers and resource dispositions).
No method uploads, publishes, transcodes or implicitly deletes tracks. Original
inputs are cloned/snapshotted and remain unchanged.

Async `submit` uses the caller's Executor and explicit progress/completion. Reading
reports real bytes; native parsing is indeterminate. Cancellation is terminal and
late native results are discarded, but synchronous native code cannot be interrupted
inside its phase. No output is published by the native call. Completion callbacks
run on the caller/executor thread that completes or cancels the operation.

Core/Formats/Remux, the operation interface and WASM bridge continue to forbid
unsafe Rust. Only `liblivephoto-native` owns the documented foreign pointer ABI
and JNI export. C callers use `native/include/liblivephoto.h`: valid immutable spans,
opaque Rust-owned response, matching free exactly once. The LLP1 envelope retains
raw file payloads and decimal-string 64-bit timing/offsets. No native pointer or JNI
local reference escapes a call. Panics are translated into explicit failures.

The host smoke test actually loads the shared library, builds a motion asset,
parses memory/path/descriptor-range inputs, checks exact time, extracts identical
movie bytes, rejects non-exact time, tests cancellation and response lifetimes.
Native Gallery, device codec and Android service acceptance are separate gates.
