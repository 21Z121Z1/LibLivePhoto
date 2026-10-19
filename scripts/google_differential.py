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
from fractions import Fraction

ROOT = Path(__file__).resolve().parent.parent
GOOGLE = "12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3"

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
        raw = run([reference, path, case])
        (case / "reference.log").write_text(raw)
        ref = json.JSONDecoder().raw_decode(raw.split("REFERENCE_JSON\t", 1)[1])[0]
        impl = json.loads(run([rust, path, case]))
        row = {"name": path.name, "sha256": digest(path), "reference": ref, "implementation": impl, "differences": []}
        assert impl["recognized"], row
        assert impl["build_error"] is None, row
        assert impl["agtm_error"] is None, row
        for mode in ["file", "memory", "range"]:
            r = ref[mode]
            if not r["recognized"]:
                row["differences"].append(mode + ":reference_not_recognized")
                continue
            for key in ["video_length", "time_us", "still_mime", "video_mime"]:
                if impl[key] != r[key]:
                    row["differences"].append(f"{mode}:{key}:{r[key]}->{impl[key]}")
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
                            assert prefix == "reference" and path.name.startswith("honor-"), (path, e.stderr)
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
        rebuilt_case = case / "rebuilt-reference"
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
        rows.append(row)
        print(path.name, "verified;", len(row["differences"]), "reference field differences", flush=True)
        return case
    for path in cases:
        compare(path)
    video = (output / "case-01/rust-video.bin").read_bytes()
    generated = output / "generated"
    generated.mkdir()
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
    ]:
        path = generated / name
        path.write_bytes(b"\xff\xd8\xff\xd9" + trailer)
        compare(path)
    avif = generated / "still.avif"
    run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=c=0x276880:s=16x16:d=0.04", "-frames:v", "1", "-c:v", "libaom-av1", "-still-picture", "1", "-n", avif])
    composed = generated / "avif-parts"
    composed.mkdir()
    built = json.loads(run([rust, avif, composed, output / "case-01/rust-video.bin"]))
    assert built["build_error"] is None, built
    compare(Path(built["rebuilt"]))
    run(["ffmpeg", "-v", "error", "-i", built["rebuilt"], "-frames:v", "1", "-f", "null", "-"])
    receipt = {"google_sha": GOOGLE, "library_sha": run(["git", "-C", ROOT, "rev-parse", "HEAD"]).strip(), "library_dirty": bool(run(["git", "-C", ROOT, "status", "--porcelain"]).strip()), "cases": rows, "avif_still_decode_passed": True}
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print("receipt:", output / "receipt.json")

if __name__ == "__main__":
    main()
