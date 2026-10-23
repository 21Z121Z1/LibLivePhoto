# Shared operation response

## Sub-features

Explicit `resourceIds` retain order and unknown SharedData bytes. Duplicate or missing IDs fail the whole request. Default extraction returns still, movie, and optional XMP. LLP1 accounts for metadata, names, MIME, IDs, framing, and every selected output payload.

## How to get to it (user POV)

Call `liblivephoto_interface::process` with `{"action":"extract","resourceIds":[6,2,4]}` and an appropriate input. Call `into_envelope` when a wrapper needs LLP1 bytes.

## Driving it with verify_extraction.py

The interface tests compare complete results and exercise JPEG and HEIF default XMP, unavailable XMP, unknown resources, invalid later IDs, and ordered multi-extent bytes. The scaled replay rejects an amplified selection before copying, accepts exactly 4919 bytes, and rejects 4918 bytes and payload-only budgets. It compares supported results against baseline `b1c556314b0939a6370f68fcd8be63089f7b3883`. The historical replay requires four assertion failures followed by four passes for three prior aggregate-budget fixes.

## Gotchas

The strict pre-copy guarantee applies to explicit selection. Default HEIF XMP retains its existing bounded materialization. Small cap replay is a disclosed source seam in scratch clones. Production keeps the 320 MiB inclusive response limit. No large OOM case is generated.
