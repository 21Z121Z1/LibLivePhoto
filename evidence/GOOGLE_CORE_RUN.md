# Google core run receipt

Executed on Linux x86_64 in the selected independent container on 2026-10-19.
This is **partial Google core coverage**, not complete ecosystem compatibility.

Validated implementation commit: `b223a462468eb66c9545cbc4a6b9b4c160e7f03a`.
Starting PR #1 head: `794ab552596db0f66095600d23cf7937a631b9ec`.
Google checkout: `12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3`.
Read-only XDRemux reference: `0688ea7d6270df52cc5968e2c36dd016f13bd31b`.
Branch: `codex/google-core-coverage-20261019`. Later receipt commits change only
documentation/evidence; the code under test is the implementation commit above.

The [machine receipt](google-core-run.json) contains exact commands, working
directories, return codes, UTC start times, durations and log SHA-256 values.
The [differential receipt](google-core-differential.json) contains input hashes,
field differences, extraction comparisons and rebuilt-reference results.

## Results

| Executed command | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked --workspace --all-targets` | 103 passed; 3 external-corpus tests ignored by default |
| `cargo test --locked --workspace --all-targets --all-features` | 103 passed; same 3 ignored |
| `cargo test --locked --doc` | pass; no doctests registered |
| `python3 scripts/fixtures.py` | 14 original SHA-256 identities verified |
| `cargo test --locked -p liblivephoto --test fixtures -- --ignored --nocapture` | both ignored tests passed; all 14 originals and HEIF/QuickTime contract |
| `python3 scripts/external_consumer.py` | public API outside workspace and Android → Apple → Android → Apple passed |
| `python3 scripts/consumer_replay.py --xdremux-repository /workspace/xdremux-consumer-reference` | unchanged pinned XDRemux public contract passed in external dependency-alias harness; not the full product build |
| `/workspace/reference-build-make/bin/motion_photo_tests` from `/workspace/reference-run` | 99/99 reference tests passed |
| `python3 scripts/google_differential.py --reference-source /workspace/libmotionphoto-reference --reference-build /workspace/reference-build-make` | 9 cases passed; all rebuilt outputs recognized; known reference differences retained |
| `bash scripts/fuzz.sh 30 /workspace/LibLivePhoto/evidence/local/final-validation/fuzz` | ASan/libFuzzer, 30 seconds per target; no crash or sanitizer finding |
| `git diff --check` | pass |

Final ASan campaign executions: `asset_input` 41651, `container_headers` 850782,
and `remux` 96763. Each target ran for 31 wall-clock seconds under its 30-second
libFuzzer budget. The machine receipt includes per-target log hashes and peak RSS.

The new tests cover snapshots/ranges/budgets, exact composition, AVIF, attribute
and element XMP, numeric conflicts, MIME diagnostics, 8/16-byte mpvd headers,
Honor trailers, AGTM time intervals/ctts/truncation, metadata-track removal,
existing Apple preservation, EXIF orientation 0 and 1–8, ICC/CICP/PQ, unknown
properties, allocation amplification, AAC facts and caller capability blockers.
The pre-change default suite had 86 passing tests; the final suite adds 17.

The differential adapter separately runs Google file and memory extraction.
It compares encoded A/V packet hashes and exact rational PTS/DTS/durations,
including the file extractor on Honor. Google memory extraction produces invalid
Honor movies at this SHA; those outputs are reported, not treated as an oracle.
AGTM baseline fields agree for the original signaled sample. All LibLivePhoto
video resources remain exact bytes before rebuilding. The generated AVIF still
also passes an independent FFmpeg pixel decode after the motion-photo build.
See the coverage matrix for the MIME/gain-map and Honor time findings.

## Environment and reference build

The first checks found a clean preloaded `/workspace/libheif` worktree on branch
`work`, no applicable AGENTS.md, accessible GitHub remotes, and no Rust/CMake in
PATH. The library was cloned into `/workspace/LibLivePhoto` and branched from
the actual PR ref, rather than empty main. Toolchains/development packages were
installed/extracted only under `/workspace/toolchains`. No saved environment
configuration, preloaded libheif source, Google reference checkout or remote repository was changed.
Google's CMake applies its own mp4v2 compatibility patch inside the separate build
directory; that generated dependency checkout is not the preloaded libheif tree.

Tools used: rustc 1.99.1 (`58cf8f9c3`), cargo 1.99.1 (`5f94df478`), CMake 4.4.4,
GCC/G++ 14.2.0, Python 3.12.14, FFmpeg 7.1.5, cargo-fuzz 0.13.2, and nightly
rustc 1.101.0-nightly (`b6eeb93c9`, 2026-10-18) for ASan instrumentation.

Reference configure prerequisites were initially absent: Protobuf headers/compiler
and JNI/AWT headers. They were resolved with locally extracted Debian development
packages, not changes to the saved environment. The successful configuration used:

```sh
PYTHONPATH=/workspace/toolchains/python \
LD_LIBRARY_PATH=/workspace/toolchains/debian/sysroot/usr/lib/x86_64-linux-gnu \
/workspace/toolchains/python/bin/cmake \
  -S /workspace/libmotionphoto-reference -B /workspace/reference-build-make \
  -G 'Unix Makefiles' -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
  -DProtobuf_INCLUDE_DIR=/workspace/toolchains/debian/sysroot/usr/include \
  -DProtobuf_LIBRARY=/usr/lib/x86_64-linux-gnu/libprotobuf.so.32 \
  -DProtobuf_LITE_LIBRARY=/usr/lib/x86_64-linux-gnu/libprotobuf-lite.so.32 \
  -DHOST_PROTOC_EXECUTABLE=/workspace/toolchains/debian/sysroot/usr/bin/protoc \
  -DLIBHEIF_SYSTEM_INCLUDE_DIR=/workspace/toolchains/debian/sysroot/usr/include/x86_64-linux-gnu \
  -DLIBHEIF_SYSTEM_LIBRARY=/usr/lib/x86_64-linux-gnu/libheif.so.1 \
  -DJAVA_INCLUDE_PATH=/workspace/toolchains/debian/sysroot/usr/lib/jvm/java-21-openjdk-amd64/include \
  -DJAVA_INCLUDE_PATH2=/workspace/toolchains/debian/sysroot/usr/lib/jvm/java-21-openjdk-amd64/include/linux \
  -DJAVA_AWT_INCLUDE_PATH=/workspace/toolchains/debian/sysroot/usr/lib/jvm/java-21-openjdk-amd64/include \
  -DJAVA_AWT_LIBRARY=/workspace/toolchains/debian/sysroot/usr/lib/jvm/java-21-openjdk-amd64/lib/libjawt.so \
  -DJAVA_JVM_LIBRARY=/usr/lib/jvm/java-21-openjdk-amd64/lib/server/libjvm.so
PYTHONPATH=/workspace/toolchains/python \
LD_LIBRARY_PATH=/workspace/toolchains/debian/sysroot/usr/lib/x86_64-linux-gnu \
/workspace/toolchains/python/bin/cmake --build /workspace/reference-build-make --parallel 6
```

Reference dependency identities and license observations:

| Reference-only dependency | Actual identity | Separately checked terms |
| --- | --- | --- |
| Google core and image_io snapshot | Google pinned SHA above | Apache-2.0 license files |
| libheif | system 1.19.8; matching local Debian headers | libheif LGPL-3+; codec/plugin and example terms remain separate |
| mp4v2 | `f1b42be9d405a955def38325dffac08ec217dcb6` | `COPYING` and `src/mp4.cpp`: MPL-1.1 |
| modp_b64 | `e1448a9415f4ebf6f559c86718193ba067cbb99d` | `LICENSE` and `src/modp_b64.c`: MIT |
| Abseil | `d7aaad83b488fd62bd51c81ecf16cd938532cc0a` | Apache-2.0 |
| Protobuf | compiler/runtime/headers 3.21.12 | Debian upstream entry: BSD-3-Clause Google; ancillary build files have separate entries |

The Google CMake mp4v2 tracked compatibility diff has SHA-256
`e22e15032daead16a594883643ec205592ae2e90a1cb2519b138489cbb355f77`.
It also generates build configuration files. The modp_b64 and Abseil tracked
diffs are empty. modp_b64's upstream master is not a stable version promise; this
receipt records the actual fetched SHA. No reference binary, dependency source,
codec, JDK or external photo is committed or added to LibLivePhoto dependencies.
Rust dependency manifests and Cargo.lock are unchanged. This is not a license
opinion for redistributing a complete native codec toolchain.

## Limits and work not run

- macOS and Windows CI jobs remain configured, but were not executed here. No
  Android Java/JNI service tests or native Gallery/Photos acceptance were run.
- The separate 10-file external-analysis corpus is not present in this container;
  its ignored test was preserved and was not run. The 14 pinned originals were run.
- Full AGTM curves/mixes, Google Moments semantic metadata construction,
  cross-file track import and MP4 fast-start/compaction remain missing.
- Pure OpCamera/JXRS variants, broader Honor/Huawei files and original Honor/AVIF
  device corpora remain missing or unverified. Dedicated Oplus/Xiaomi/vivo/Honor
  native writers are not added. Unknown stream roles remain unknown.
- A bounded sanitizer campaign is not proof of parser safety. Structural and
  sample/byte evidence does not establish original-device acceptance.

There were no unresolved access/approval failures. No push, PR update, merge,
publication, remote comment, additional Codex task or model/service switch was
performed. No additional implementation round is started by this delivery.
