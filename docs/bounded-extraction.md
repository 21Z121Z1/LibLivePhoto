# Complete response checks before resource copies

The shared interface now rejects an oversized `resourceIds` selection before copying any selected payload into an output vector. CLI consumers receive the same rejection through `process`, even when they do not serialize LLP1.

The old path copied one resource at a time and checked the response only in `into_envelope`. Distinct SharedData IDs can refer to the same extent, so a bounded source can expand into many separate outputs. The private `CheckedExtraction` borrows the complete selection, asks the core reader for each exact extracted length with `io::sink()`, and checks the complete response before its consuming materializer copies bytes. Core remains responsible for membership, range validation, and extent order.

The plan and public `into_envelope` share `checked_response_len`. It includes the actual serialized success/metadata wrapper, 12 fixed header bytes, 20 fixed bytes per file, UTF-8 names and MIME, and each output payload. Field lengths must fit u32 and resource IDs must fit i64. The response limit remains 320 MiB inclusive. The 128 MiB input, 1 MiB request, 64 MiB wrapper metadata, and 4096-file limits retain their values.

Supported output keeps its original order, bytes, names, MIME, resource IDs, and metadata. Repeated IDs remain invalid. Each distinct ID counts in full, including aliases. Budget rejection keeps the existing `invalidRequest` code and `response budget exceeded` message. Default extraction uses the same plan and materializer, while retaining the existing bounded HEIF XMP assembly of at most 4 MiB.

## Reproducible evidence

Run the local and CI command from the repository root:

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
```

The extraction replay uses actual `process` and core extraction in scratch clones. Only a lowered response cap and a counter after completed resource-vector writes are injected. Baseline `b1c556314b0939a6370f68fcd8be63089f7b3883` copies 46768 bytes at an 8192-byte cap and returns success. The corrected path rejects with zero counted payload bytes. A separate supported selection emits exactly 4919 LLP1 bytes. It succeeds at 4919 and fails at 4918. Payload-only and wire-overhead cases fail before copying. Supported selected, default, and unavailable-XMP results compare byte for byte with the baseline.

The historical recipe retains the original assertions and verifies production Git blobs before red and green. Its four tests cover three earlier aggregate-budget fixes.

| Fix | Replayed behavior |
| --- | --- |
| `82abd717963f8c6e78ddce3babf22f49427c8048` | Aggregate scoring samples, copied payload, and descriptor count. |
| `0651b8020a6f4a41a353e9937932b902f1687966` | Overlapping AGTM sample, payload, and curve budgets. |
| `71818fbc4d724dea4e326ac642142a5d60e4fd57` | One frame-manifest budget shared across video tracks. |

Red must run the selected test and fail in an assertion with exit 101. Green must pass with exit 0. E3 uses only the test dependency from `cd2159d375d22498c9437b7df87c2546700a9288`; its original production prefix remains unchanged until green. Receipts record commands, exact revisions, source hashes, and log hashes. Scratch cleanup preserves these artifacts.

This proof does not measure peak memory, allocator activity, speed, decoding, or device acceptance. It does not allocate a production-size response or induce OOM. Interface tests separately check unknown resource order, JPEG and HEIF defaults, unavailable XMP, invalid later IDs, and a seven-byte item with two ordered extents.

The branch also simplifies one inherited MIME comparison with an equivalent De Morgan expression. Rust 1.96 Clippy reports the original expression as `nonminimal_bool` under the existing `-D warnings` gate. No MIME behavior or lint strictness changes.

The [verification skill and feature map](../.agents/skills/verify-liblivephoto/SKILL.md) describe launch, doctor, drive, and cleanup for future changes.
