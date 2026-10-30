# CLI publication

## Sub-features

The CLI builds a Motion Photo from still and movie parts, checks it, extracts the original movie, and refuses an existing output directory.

## How to get to it (user POV)

Use `llp build STILL --movie MOVIE --time 13/16 --target google-jpeg --out NEW_DIRECTORY`. Use `llp check` and `llp extract INPUT --video --out ANOTHER_NEW_DIRECTORY` on the built asset.

## Driving it with verify_extraction.py

Launch builds the real `llp` binary. Doctor checks its LLP1 version. Drive runs the existing `real_cli_build_check_extract_round_trip_and_refuse_overwrite` command test. It compares the extracted movie with the demo input, checks the expected output filename, and proves that a refused extraction leaves the existing report bytes unchanged.

## Gotchas

The command test creates and removes its own temporary files. The command log and receipt survive cleanup. The demo input is a local structural and byte-preservation check, not a device acceptance corpus.
