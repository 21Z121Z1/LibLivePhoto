# Vendor continuation: source facts and independent originals

This continues [the earlier matrix](VENDOR_RESEARCH_20261019.md), without
repeating its blocked downloads or treating synthetic outputs as native
interoperability. **O** means independent original-byte evidence, **S** static
source inference, **D** official documentation, **L** listing only. There is no
new native Gallery/Photos acceptance.

| Family | New checked evidence | Implemented subset / remaining boundary |
| --- | --- | --- |
| OPPO/OnePlus | L: the inspected OnePlus Photos15.30.30/150300300 listing explicitly names `com.oneplus.gallery`,268971840 bytes; this corrects the earlier namespace attribution. S: pinned public decompilation contains `com.oplus.gallery.olive_decoder` file/URI source adapters and an XMP-based reader. Package names, SDK namespaces and permission prefixes are distinct. | Existing OpCamera/JXRS and original ColorOS LPEX contracts remain. No authenticated APK hash/component version or new checksum/second-stream semantics were obtained. Listing access does not establish a binary identity or native writer. |
| Xiaomi | S: pinned `MiLivePhotoInfo` declares video offset, microsecond timestamp and a modern-versus-legacy flag; `MiLivePhotoTools` distinguishes header/EXIF checks and MediaStore paths. Some decompiled methods are incomplete. D: prior official SDK1.0.3 guide remains separate. | Existing Google-style original is covered. No private EXIF-only/portrait compatibility or native writer is inferred; the static source is not authenticated as the official1.0.3 SDK package. |
| vivo | O: both independent existing originals have one EStream UUID and one ExtInfo UUID,31 complete stream records, tags1/6/10 and exact data-to-directory consumption. S: nine vendor-namespace media-extension classes describe the same framing. | Added bounded UUID/ordered stream extraction and source-mapped metadata blocks. Six independent authored tests plus both originals and real C/JNI use the new API. Tags and keys remain opaque; no pairing/time/portrait/depth inference or native writer. |
| Samsung | O/S: existing original hashes and pinned SEF/ExifTool observations were reused. No new authenticated binary/component version was acquired in this continuation. | Direct JPEG writing remains limited. AutoPlay stream selection, exact native time mapping and wider firmware/native acceptance are unresolved; no support expansion based on a listing. |
| Honor | S: pinned public decompilation provides LivePhotoUtils, HnLivePhotoUtils and MotionPhotoUtils facades. It separates legacy LIVE data, a gated modern path and device-dependent playback; rotation can depend on platform APIs. | Existing synthetic Google-parity20/40/TLV contracts remain. The unverified legacy rotation/footer path and vendor platform operations are unsupported; no original Honor corpus, native writer or Gallery acceptance is claimed. |

The primary facts ledger is [vendor-round2-acquisition.json](vendor-round2-acquisition.json).
Sources are public Git snapshots, not authenticated original SDK/APK packages:

* `imwangwang/vivo-apps@6aff38cfa6133250fc58a2fdc5f6452c676440aa`, nine
  selected text files/130261 bytes, with no root license identified. Its
  VERSIONCODE2016 is a wire/source constant, not a verified app version. The
  actual independent originals declare2202; only matching observed framing is
  implemented. No APK, native library, manifest or package hash was available.
* `cxxsheng/TiktokSource@2c6fb7c918b71713c23c4464e7fbb51b752f54a2`, selected
  Oplus/Xiaomi/Honor text only, with canonical Git-blob and SHA256 checks. No
  root license or original APK/component version was identified. Decompiled
  source is static evidence with those limits, not a redistribution grant or
  executable dependency. No code was copied, translated, linked or executed.

## Independently measured vivo framing

The existing SHA-locked originals have EStream payloads516978/510208 bytes.
Each ends in31 fixed directory rows and a count marker. Exact stream data sums
are516066/509296 bytes; raw payload hashes/ranges are in
[vendor-vivo-stream-observations.json](vendor-vivo-stream-observations.json).
The parser validates every framing marker, checked size and enclosing UUID
boundary before exposing a result. Duplicate keys are retained in order rather
than selecting a last value. Raw payloads are borrowed; extraction copies only
when explicitly requested. Unknown encodings remain wire tags, not stream roles.

The ExtInfo UUID reuses the existing CameraAlbum parser, bounded to that box.
The source imageTime values31/12 do **not** establish exact video sample times:
the corresponding sample-index CTS values are116011/90000 and44972/90000,
whereas declared XMP photo times are1333944/1000000 and838055/1000000.
No average-FPS conversion, inferred unit or fabricated exact time is introduced.
Matching declaration IDs still do not prove unique capture pairing.

The C/JNI test vector is an owned neutral JPEG/H.264 demo plus independently
authored framing; five older binding vectors reproduce unchanged. It tests
transport and bytes, while the two separate original files supply O evidence.
Neither is actual Gallery interoperability. Proprietary/decompiled source and
original media remain private and outside Git; only manifests/hashes/facts and
owned synthetic test data are committed. New dependency count:0.

## Executed verification

The same core/JVM/fuzz/fixture objects passed [exact4d02be4 CI](platform-4d02be4.json):
Linux/macOS211 and Windows210 tests in each default/all-feature run, both
public-original tests, actual three-host C/JNI/packaged consumers and2114027
ASan executions. Android installed all65 existing service assertions and cleaned
up; iOS26.2 actual Swift/Rust and existing Apple/browser gates passed. The new
UUID extraction is directly exercised by Rust/original/C/JNI tests; the Android
65-assertion contract is not a new vendor Gallery/portrait acceptance test.
The [85-case Google comparison](differential-4d02be4.json) covers existing Google
contracts, not a nonexistent Google vivo provider. Reference99 includes an
explicit skipped HEVC encoder case; no reference rendering result is inferred.
