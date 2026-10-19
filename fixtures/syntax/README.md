# Owned round3 vectors

These files contain no private photographs or third-party source code.
The ICC files are independently authored envelope vectors with opaque private
tags, not complete semantic colour profiles.4.3 is intentionally outside the
pinned4.4 checker. The MOV contains repository-owned neutral video copied
without re-encoding plus newly encoded silence using AAC-LC/PCE48kHz/stereo and
the primary writer's QuickTime wave wrapper. The stz2 variant changes only its
sample-size indexes; real packet hashes and exact timelines are checked.

Reproduce into a new directory with
`python3 scripts/create_syntax_fixtures.py --out NEW_DIRECTORY`.
Recorded same-environment reproduction used system FFmpeg7.1.5-0+deb13u1 with
the native AAC encoder and bitexact flags. Other tool versions are not promised
to produce identical AAC bytes. FFmpeg is a GPL-enabled development tool here;
its executable/source is not redistributed or linked into the library. These
owned data files use the repository license. Actual commands, four hashes and
separate research-source licenses are in
[the source ledger](../../evidence/syntax-round3-sources.json).

`scripts/test_syntax_contracts.py` exercises the real CLI and independently uses
ffprobe for each packet's SHA256, exact DTS/PTS/duration and file index target.
This does not establish audio/colour rendering or device/Gallery acceptance.

The round4 acceptance manifest is [cases.json](cases.json):42 cases with exact
field expectations, classified errors/Unsupported reports, original media and
transport parity goldens. Every source/golden has a SHA256 in the manifest.
The added direct AAC file uses the same owned video plus newly encoded silence,
without `-aac_pce`. Its malformed derivatives change only descriptor syntax.
The compact-index input adds103actual unreferenced bytes to its single mdat;
the compact golden discards those bytes while preserving all17packets. Import,
removal and legacy stsz goldens are also independently demuxed by ffprobe.
The AGTM vector derives solely from the existing owned bindings fixture, with
compact sizes and relocated chunk offsets; its original raw bytes are retained.
The goldens alone establish transport parity; they are not an independent format
oracle. Eight output movies receive independent per-packet hash, exact DTS/PTS/
duration and mdat-index checks (198 output packets altogether).

Reproduce the added manifest and data into another new directory using
`python3 scripts/create_acceptance_fixtures.py --out NEW_DIRECTORY` after building
`liblivephoto-native`. It never overwrites an existing directory. The same
FFmpeg build is needed for byte-identical encoder output; the cross-platform CI
uses these committed, hashed owned inputs rather than regenerating them.
