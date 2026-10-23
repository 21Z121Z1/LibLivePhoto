# Library resource bytes

## Sub-features

The library extracts immutable resources to a caller-owned writer. Ordered extents, exact time, explicit parts or Apple pairing, and source cursor preservation retain their existing core contracts.

## How to get to it (user POV)

Call `MotionPhoto::parse`, `MotionPhoto::open`, or `MotionPhoto::open_file`, then call `extract` on an asset-owned resource.

## Driving it with verify_extraction.py

The drive stage runs `core_contract`, `video_metadata`, and `agtm_complete` through Cargo, plus the bridge library tests. This checks the reader and the current aggregate-budget regressions. The interface test `selected_heif_item_uses_ordered_extent_bytes_and_exact_length` additionally selects a HEIF item whose two extents are out of source order. Its expected output is seven explicit source bytes.

## Gotchas

The HEIF item is synthetic encoded data. These tests check container bytes without decoding a picture. No core budget or platform implementation is replaced by the interface plan.
