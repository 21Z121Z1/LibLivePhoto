#!/usr/bin/env python3
"""Linux local reference comparison. Never downloads or modifies upstream source.

Requires an unmodified checkout and an existing Google motion_photo_tests build.
All inputs, extracted bytes and logs stay under an ignored, new evidence directory.
The JSON receipt records differences; Google output is not an absolute oracle.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import tempfile
import traceback
from fractions import Fraction

ROOT = Path(__file__).resolve().parent.parent
GOOGLE = "12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3"
PHOTO_TIME_REJECTION = "conversion is blocked: " + "; ".join(
    f"track {track} metadata sample 0 conflicts with the source photo time or score contract: [PhotoTimeMismatch]; remove that track explicitly before composition"
    for track in [4, 5])
# Measured identical refusals from the preserved bd6f85b SDK and round5 source.
# No filename/prefix/error-substring wildcard may turn another error into a pass.
EXPECTED_REJECTIONS = {
    "449750b550710e3a2311549154dbd28118dc72a95cb1c82306249f5d7e44f558": "honor-footer20.jpg",
    "c20dae73b1d2a1dfe1643418d993d43757fc5a1dfed8dee317ad8c52795a226b": "honor-footer40.jpg",
    "ef7910aee7a9c17da0961fb2ccf286b5db56676b0984fdbd72b83b1d904bb93e": "honor-tlv.jpg",
}
POSITIVE_HONOR_SHA = "a791dc8b57a874df21dadb7e4542c447173af335beb49a2e42d5dddca8e35684"
REFERENCE_MOVIE_SHA = "310e216584649672746c32a2830bf4cca4c570b4e6c3b8036c68f8c59085b432"

def expected_build_rejection(name, sha256, error, rebuilt):
    if EXPECTED_REJECTIONS.get(sha256) == name:
        assert error == PHOTO_TIME_REJECTION and rebuilt is None, (name, sha256, error, rebuilt)
        return True
    assert error is None and rebuilt is not None, (name, sha256, error, rebuilt)
    return False

def expected_field_difference(row, mode, key, observed, actual):
    known = (row["name"] in ["motion_photo_agtm.MP.jpg", "motion_photo_no_agtm.MP.jpg"]
             and key == "video_mime" and observed == "image/jpeg" and actual == "video/mp4")
    known |= (row["sha256"] in ["c20dae73b1d2a1dfe1643418d993d43757fc5a1dfed8dee317ad8c52795a226b", POSITIVE_HONOR_SHA]
              and key == "time_us" and observed == -1 and actual == (500000 if row["sha256"] == POSITIVE_HONOR_SHA else 450000))
    assert known, ("unexpected reference field difference", row["name"], mode, key, observed, actual)
    return f"{mode}:{key}:{observed}->{actual}"

def run(args, **kwargs):
    return subprocess.run(list(map(str, args)), check=True, text=True, errors="backslashreplace", capture_output=True, **kwargs).stdout

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def packets(path):
    info = json.loads(run(["ffprobe", "-v", "error", "-show_streams", "-show_packets", "-show_data_hash", "sha256", "-of", "json", path]))
    result = []
    for stream in info["streams"]:
        if stream["codec_type"] not in ["video", "audio"]:
            continue
        scale = Fraction(stream["time_base"])
        values = []
        for packet in info["packets"]:
            if packet["stream_index"] == stream["index"]:
                values.append([packet.get("data_hash"), *[str(int(packet[key]) * scale) if key in packet else None for key in ["pts", "dts", "duration"]]])
        result.append([stream["codec_type"], stream["codec_tag_string"], values])
    return result

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference-source", type=Path, required=True)
    parser.add_argument("--reference-build", type=Path, required=True)
    args = parser.parse_args()
    source = args.reference_source.resolve()
    assert run(["git", "-C", source, "rev-parse", "HEAD"]).strip() == GOOGLE
    run(["git", "-C", source, "diff", "--exit-code"])
    parent = ROOT / "evidence/local"
    parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="google-diff-", dir=parent))
    build = args.reference_build.resolve() / "lib/core"
    link = shlex.split((build / "CMakeFiles/motion_photo_tests.dir/link.txt").read_text())
    libraries = link[link.index("-o") + 2:]
    reference = output / "reference-probe"
    run(["c++", "-std=c++17", "-I" + str(source), ROOT / "scripts/google_reference_probe.cc", "-o", reference, *libraries], cwd=build)
    run(["cargo", "build", "--locked", "-p", "liblivephoto", "--example", "core_probe"], cwd=ROOT)
    rust = ROOT / "target/debug/examples/core_probe"
    fixtures = source / "lib/core/motion_photo/testdata"
    cases = [fixtures / name for name in [
        "motion_photo.MP.jpg", "motion_photo_single_video_track.MP.jpg",
        "motion_photo_single_video_track.MP.heic", "motion_photo_agtm.MP.jpg",
        "motion_photo_no_agtm.MP.jpg",
    ]]
    rows = []
    def compare(path):
        case = output / f"case-{len(rows):02}"
        case.mkdir()
        row = {"name": path.name, "sha256": digest(path), "execution_status": "running", "differences": []}
        rows.append(row)
        raw = run([reference, path, case])
        (case / "reference.log").write_text(raw)
        ref = json.JSONDecoder().raw_decode(raw.split("REFERENCE_JSON\t", 1)[1])[0]
        impl = json.loads(run([rust, path, case]))
        row.update(reference=ref, implementation=impl)
        assert impl["recognized"], row
        rejected = expected_build_rejection(path.name, row["sha256"], impl["build_error"], impl["rebuilt"])
        row["build_status"] = "expected_rejection" if rejected else "built"
        assert impl["path_matches_memory"], row
        assert impl["agtm_error"] is None, row
        for mode in ["file", "memory", "range"]:
            r = ref[mode]
            assert r["recognized"], (mode, row)
            for key in ["video_length", "time_us", "still_mime", "video_mime"]:
                if impl[key] != r[key]:
                    row["differences"].append(expected_field_difference(row, mode, key, r[key], impl[key]))
        row["rust_extracted_video_sha256"] = digest(case / "rust-video.bin")
        if row["sha256"] in EXPECTED_REJECTIONS or row["sha256"] == POSITIVE_HONOR_SHA:
            assert (case / "rust-video.bin").read_bytes() == video, row
            assert (case / "rust-still.bin").read_bytes() == b"\xff\xd8\xff\xd9", row
            row["full_original_movie_including_metadata_retained"] = True
        row["parse_and_extraction_executed"] = {
            "rust_file_and_memory_parse":impl["path_matches_memory"],
            "rust_still_and_movie":all((case/name).is_file() for name in ["rust-still.bin","rust-video.bin"]),
            "reference_file_and_memory_extract":ref["file_extracted"] and ref["extracted"],
        }
        assert all(row["parse_and_extraction_executed"].values()), row
        for key in ["still", "video"]:
            rp = case / f"reference-{key}.bin"
            lp = case / f"rust-{key}.bin"
            if rp.exists():
                row[f"{key}_bytes_equal"] = digest(rp) == digest(lp)
                if key == "video":
                    for prefix in ["reference", "rust"]:
                        path_video = case / f"{prefix}-video.bin"
                        try:
                            info = json.loads(run(["ffprobe", "-v", "error", "-show_entries", "stream=codec_type,codec_tag_string", "-of", "json", path_video]))
                            row[prefix + "_tracks"] = info["streams"]
                        except subprocess.CalledProcessError as e:
                            assert prefix == "reference" and (row["sha256"] in EXPECTED_REJECTIONS or row["sha256"] == POSITIVE_HONOR_SHA), (path, e.stderr)
                            assert path_video.read_bytes() == path.read_bytes()[-impl["video_length"]:], row
                            row["reference_video_ffprobe_error"] = e.stderr.strip()
                            row["differences"].append("memory_extraction:reference_tail_offset_includes_vendor_footer")
                    if "reference_video_ffprobe_error" not in row:
                        assert packets(case / "reference-video.bin") == packets(case / "rust-video.bin"), row
                        row["encoded_av_packets_and_exact_times_equal"] = True
                    file_video = case / "reference-file-video.bin"
                    if file_video.exists() and ref["file_extracted"]:
                        assert packets(file_video) == packets(case / "rust-video.bin"), row
                        row["file_extraction_encoded_av_packets_and_exact_times_equal"] = True
        if ref["agtm"] is not None:
            assert impl["agtm"], row
            assert impl["agtm"][0]["hdrReferenceWhite"] == ref["agtm"]["hdrReferenceWhite"], row
            assert impl["agtm"][0]["hasAdaptiveToneMap"] == ref["agtm"]["hasAdaptiveToneMap"], row
        if rejected:
            row["execution_status"] = "passed"
            row["rebuild_validation"] = "not executed: exact fixed-input expected rejection; parse/extraction checks executed above"
            print(path.name, "EXECUTED_PASS; EXPECTED_REJECTION;", len(row["differences"]), "expected reference differences", flush=True)
            return case
        rebuilt_case = case / "rebuilt-reference"
        if row["sha256"] == POSITIVE_HONOR_SHA:
            assert impl["time_us"] == 500000 and Path(impl["rebuilt"]).read_bytes().endswith(video), row
            row["rebuilt_full_original_movie_including_metadata_retained"] = True
        rebuilt_case.mkdir()
        rebuilt_ref = run([reference, impl["rebuilt"], rebuilt_case])
        (rebuilt_case / "reference.log").write_text(rebuilt_ref)
        rebuilt_ref = json.JSONDecoder().raw_decode(rebuilt_ref.split("REFERENCE_JSON\t", 1)[1])[0]
        row["rebuilt_reference"] = rebuilt_ref
        assert rebuilt_ref["memory"]["recognized"], row
        assert rebuilt_ref["memory"]["video_length"] == impl["video_length"], row
        if impl["time_us"] >= 0:
            assert rebuilt_ref["memory"]["time_us"] == impl["time_us"], row
        assert packets(case / "rust-video.bin") == packets(rebuilt_case / "reference-video.bin"), row
        row["rebuilt_encoded_av_packets_and_exact_times_equal"] = True
        row["execution_status"] = "passed"
        print(path.name, "EXECUTED_PASS; BUILT;", len(row["differences"]), "expected reference differences", flush=True)
        return case
    def checked_compare(path):
        before = len(rows)
        try:
            return compare(path)
        except Exception as error:
            if len(rows) == before:
                rows.append({"name": path.name, "sha256": digest(path)})
            rows[-1].update(execution_status="failed", error=str(error), traceback=traceback.format_exc())
            print(path.name, "REAL_FAILURE:", type(error).__name__, str(error)[:600], flush=True)
            return None
    for path in cases:
        checked_compare(path)
    # A fixed input segment, not the outcome of a preceding tested operation.
    # Thus a genuine earlier failure cannot prevent later independent cases.
    video = cases[1].read_bytes()[-123086:]
    assert hashlib.sha256(video).hexdigest()==REFERENCE_MOVIE_SHA
    generated = output / "generated"
    generated.mkdir()
    (generated/'reference-movie.mp4').write_bytes(video)
    def tag20(s):
        data = s.encode()
        assert len(data) <= 20
        return data.ljust(20, b"\0")
    def tlv(data, tag):
        return data + len(data).to_bytes(4, "little") + tag20(tag)
    for name, trailer in [
        ("honor-footer20.jpg", video + tag20(f"LIVE_{len(video)}")),
        ("honor-footer40.jpg", video + tag20("450") + tag20(f"LIVE_{len(video)}")),
        ("honor-tlv.jpg", tlv(video, "HiHonor_1000000") + tlv(b"unknown", "HiHonor_CIID")),
        ("honor-time-consistent.jpg", video + tag20("500") + tag20(f"LIVE_{len(video)}")),
    ]:
        path = generated / name
        path.write_bytes(b"\xff\xd8\xff\xd9" + trailer)
        checked_compare(path)
    avif_decoded=False;before_avif=len(rows)
    try:
        avif = generated / "still.avif"
        run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=c=0x276880:s=16x16:d=0.04", "-frames:v", "1", "-c:v", "libaom-av1", "-still-picture", "1", "-n", avif])
        composed = generated / "avif-parts"
        composed.mkdir()
        built = json.loads(run([rust, avif, composed, generated/'reference-movie.mp4']))
        assert built["build_error"] is None, built
        checked_compare(Path(built["rebuilt"]))
        run(["ffmpeg", "-v", "error", "-i", built["rebuilt"], "-frames:v", "1", "-f", "null", "-"])
        avif_decoded=True
    except Exception as error:
        if len(rows)==before_avif:rows.append({"name":"avif-parts-build"})
        rows[-1].update(execution_status="failed",error=str(error),traceback=traceback.format_exc())
        print("AVIF REAL_FAILURE:",str(error)[:600],flush=True)
    failures = [row["name"] for row in rows if row["execution_status"] != "passed"]
    receipt = {"google_sha": GOOGLE, "library_sha": run(["git", "-C", ROOT, "rev-parse", "HEAD"]).strip(), "library_dirty": bool(run(["git", "-C", ROOT, "status", "--porcelain"]).strip()), "cases": rows, "avif_still_decode_passed": avif_decoded,
               "planned_cases":10,"executed_cases":len(rows),"executed_passes":len(rows)-len(failures),"successful_builds":sum(row.get("build_status")=="built" and row["execution_status"]=="passed" for row in rows),"expected_build_rejections":sum(row.get("build_status")=="expected_rejection" for row in rows),"expected_reference_differences":sum(len(row.get("differences",[])) for row in rows),"real_failures":failures,"complete":len(rows)==10,"no_conflicting_metadata_removed":True}
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print("receipt:", output / "receipt.json")
    if failures or len(rows)!=10:raise SystemExit(1)

if __name__ == "__main__":
    main()
