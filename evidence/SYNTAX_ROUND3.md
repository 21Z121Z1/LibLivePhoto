# Round3 syntax contracts and invariants

Starting delivery53a3499961fedaf2467b347f1e98170595e1edd6 preserves tested
4d02be46804c13c19657bbd6d865342f6be816b7. This round adds independent contracts;
the previous211/210 tests, Android65 and vivo extraction are historical evidence,
not newly executed results for changed code.

| Contract | Primary authority fixed before implementation | Intended API / verification boundary |
| --- | --- | --- |
| ICC envelope | [ICC.1:2022 v4.4](https://archive.color.org/specification/ICC.1-2022-05.pdf), §§7.1–7.4 and generic tag headers | Bounded borrowed `inspect_icc_structure`; optional `validate_icc_structure` / conformance profile `icc-structure-v4.4`. Exact length/header, unique signatures, ordered-independent table, identical offset+size sharing, no partial overlap, contiguous aligned zero-padded data. Unknown/private tags remain raw. Other versions report unsupported. Required semantic tags, profile-ID hashes and rendered colour are not certified. |
| AAC-LC PCE | Fraunhofer FDK v2.0.3, commit716f4394641d53f0d79c9ddac3fa93b03a49f278, libMpegTPDec/src/tpdec_asc.cpp, CProgramConfig_Read and GASpecificConfig; its ISO13818-7 references distinguish fields/counting from renderer behavior | Parse channelConfiguration0 only for the expressly supported ASC/GA form; count SCE/CPE/LFE independently of associated-data/coupling elements. Keep declarations/raw bytes, reject conflicting rate/profile/tags and truncated/budget-exceeding structures. No loudspeaker-position/rendering inference. |
| QuickTime wave/esds | FFmpeg n7.1 commitb08d7969c550a804a59511c7b83f2dd8cc0499b8, libavformat/mov.c and movenc.c; primary reader/writer plus independently generated real output | Audio entry versions0/1; explicit unique esds, optional matching frma/mp4a marker and terminator. Unknown children/versions and direct+nested descriptor ambiguity are rejected rather than guessed. This is a tested wire subset, not full QTFF certification. |
| One remux index layout: stz2 | Official MPEG [conformance grammar](https://github.com/MPEGGroup/FileFormatConformance/blob/767a5bcb9fa4841d1b3bb0ca112a6e7d4c6bf067/data/standard_features/14496-12/boxes.json), ISO14496-12 edition7/2022, together with official reference SampleSizeAtom.c at689ac096a8e485385089341785c10271cf0684a8 | Version0/flags0, reserved24 zero, field widths4/8/16, exact count and packed extent. The grammar is a committee-maintained extraction, not a claim that the full ISO document was downloaded. No sidx/moof/external-media expansion. |

Before changing remux, the invariants are:

1. Exactly one sample-size table (stsz or stz2), exact packed length and a bounded
   sample count. The odd4-bit trailing nibble is not another sample; original
   packed bytes are retained. Mixed/duplicate declarations are ambiguous.
2. Each index resolves through the existing stsc and stco/co64 mapping to the
   same complete sample bytes inside a self-contained mdat. Relocation rewrites
   chunk targets; count, description association and sample boundaries persist.
3. Every sample retains exact DTS/duration/CTS and existing supported edits.
   No average-frame-rate conversion or inferred time domain is introduced.
4. Fast-start, compaction, selected removal and track import use the existing
   reference checks and disposition reports. Unknown offset-bearing boxes,
   fragmented/indexed top-level combinations and external references remain
   rejected. Byte retention is not permission to copy unknown reference syntax.
5. Tests must compare each payload, exact time/index target, preserved stz2 bytes
   and unknown-box/error behavior after remux, including malformed widths,
   lengths, counts and conflicting tables. Synthetic vectors are not playback
   or native Gallery acceptance.

Container materialization of ICC, Apple QTFF JSON, AOSP ASC, MP4RA and a GitHub
tree each returned proxy CONNECT403. Those downloads were stopped without
proxy/auth/access changes. Existing authorized cloud source-reading tools can
inspect public ICC and GitHub source facts; they do not establish successful
container downloads. Apple QTFF document bodies remain unread. No third-party
source is copied, translated, linked or executed as a production dependency.
Source licenses, identities and failed transfers are recorded in
[syntax-round3-sources.json](syntax-round3-sources.json). Actual execution remains
separate from this pre-implementation contract.
