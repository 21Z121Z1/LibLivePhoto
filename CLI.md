# Maintained CLI

Build/install with `cargo build --release -p liblivephoto-cli` or
`cargo install --path crates/liblivephoto-cli --locked`. The binary is `llp`.
Run `llp --help` for the complete options and per-operation input contract.

```sh
llp version
llp parse motion.jpg
llp check motion.jpg
llp metadata ordinary.heic
llp build still.jpg --movie video.mp4 --time 13/16 --target google-jpeg --out new-built
llp extract motion.jpg --still --video --xmp --out new-extracted
llp agtm motion.jpg --time 13/16 --out new-agtm
llp agtm movie.mp4 --video-input --time 13/16 --out new-agtm-from-video
llp xmp metadata.xmp
llp optimize movie.mp4 --out new-optimized
llp import-tracks destination.mp4 --movie source.mp4 --tracks 1 --out new-imported
llp remove-tracks movie.mp4 --tracks 3 --out new-removed
llp conformance still.avif --options-json '{"profile":"heif-color-2025"}'
llp conformance movie.mp4 --tracks 1,2 --options-json '{"profile":"google-video-container-1.0"}'
```

Default building from separate still/movie inputs explicitly declares a parts
association. `parse` of two files validates an Apple pair unless `--parts` is set.
A mismatching Apple identifier is not silently re-associated. Exact time uses an
integer fraction; 1/3 second is refused when a microsecond representation is required.
Use `--dry-run` to inspect a build plan before generating files. Container/codec
transcoding is not implicit; unsupported targets/metadata/resources are diagnosed.

JPEG/HEIC/AVIF, existing Apple support, provider filters, explicit sidecar/drop
policies and supported track operations use the same library code as Rust/JNI.
`--disable-3p`, `--enable-provider`, `--strict`, source `--range OFFSET:LENGTH`,
selected resource IDs and `--options-json` are exposed. Ranges refer to the selected
snapshot and `inputRanges` preserves the enclosing file offset separately. Use `-`
for bounded stdin. Standalone AGTM explicitly requires its time; no fabricated zero.
AGTM output is metadata, not rendered pixels. No Moments construction arguments.

`conformance` is an optional read-only profile query, separate from default
preservation/check behavior. Video track IDs explicitly name primary and
optional secondary roles. Reports distinguish required violations, unsupported
facts and recommendations; they cover only their listed rules. See
[the profile boundaries](CORE_INTEGRATION.md), including AAC declarations and
the public44kHz/44100Hz ambiguity. A successful report is not decoder, complete
format or native Gallery certification.

Reports are JSON on stdout. Exit 0 means the operation/check passed; 1 means a
library/plan/check failure with details; 2 means command/I/O/publication misuse.
Generated/extracted files require `--out NEW_DIRECTORY`. The command creates a
new directory and every file with CREATE_NEW; originals and existing outputs are
never overwritten. Ordinary write failures roll back only files created by that
invocation and remove its directory only if empty; cleanup failures report the
remaining paths. `report.json` is written last. This does not promise atomic
visibility to other processes or recovery after power loss/process termination;
callers should read the report only after successful exit.
Sidecars are published beside the main result and are not native Gallery claims.

The optimizer/importer is bounded to validated standalone layouts. It reports
unreferenced media/padding and source metadata sidecars. Unsupported references,
fragmentation, indexes or unknown offset contracts are explicit failures; a passing
support query is not universal MP4 compatibility. Read [the acceptance checklist](evidence/ACCEPTANCE_CHECKLIST.md).
