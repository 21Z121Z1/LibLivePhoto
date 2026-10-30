#!/usr/bin/env python3
"""Compare raw sample selection against pinned Google; assert explicit differences."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SHA = "12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3"

def run(args):
    return subprocess.check_output([str(a) for a in args], cwd=ROOT, text=True)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference-source", type=Path, required=True)
    source = parser.parse_args().reference_source.resolve()
    assert run(["git", "-C", source, "rev-parse", "HEAD"]).strip() == SHA
    output = Path(tempfile.mkdtemp(prefix="timed-metadata-diff-", dir=ROOT / "evidence/local"))
    reference = output / "reference"
    run(["c++", "-std=c++17", "-I" + str(source / "lib/core/motion_photo/includes"),
         ROOT / "scripts/timed_metadata_reference_probe.cc",
         source / "lib/core/motion_photo/src/motion_photo_mp4/mp4_box_reader.cc", "-o", reference])
    subprocess.run(["cargo", "build", "--locked", "-p", "liblivephoto", "--example", "timed_metadata_probe"], cwd=ROOT, check=True)
    rust = ROOT / "target/debug/examples/timed_metadata_probe"
    fixtures = output / "fixtures"
    run([rust, "fixtures", fixtures])
    # Independently enumerated half-open intervals: (source index, start, duration).
    expected = {
        "plain": [(0, 0, 1000), (0, 0, 1000), (1, 1000, 1000), (1, 1000, 1000), None],
        "composition": [None, (0, 500, 1000), (0, 500, 1000), (1, 1500, 1000), None],
        "edits": [None, (0, 500, 500), (1, 1000, 500), None, None],
        "edited-composition": [None, (0, 500, 1000), (0, 500, 1000), None, None],
    }
    payloads = [[0, 0], [0, 0x80, 3, 0xf7]]
    rows = []
    for name, intervals in expected.items():
        path = fixtures / (name + ".mp4")
        for time, interval in zip([0, 500000, 1000000, 1500000, 2500000], intervals):
            google = json.loads(run([reference, path, time]))
            actual = json.loads(run([rust, path, time]))
            assert google["index"] == min(time // 1000000, 1), (name, time, google)
            assert google["bytes"] == payloads[google["index"]]
            assert actual["success"], (name, time, actual)
            if interval is None:
                assert not actual["samples"], (name, time, actual)
            else:
                index, start, duration = interval
                assert actual["samples"] == [{"index": index, "time": start, "scale": 1000, "duration": duration, "bytes": payloads[index]}], (name, time, actual)
            rows.append({"fixture": name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                         "timestamp_us": time, "google": google, "rust": actual,
                         "same_source_sample": interval is not None and interval[0] == google["index"],
                         "expected_half_open_interval_verified": True})
    receipt = {"reference_sha": SHA, "implementation_sha": run(["git", "rev-parse", "HEAD"]).strip(),
               "tracked_diff_sha256": hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD"], cwd=ROOT)).hexdigest(),
               "cases": rows, "scope": "Google Mp4BoxReader primitive actually used by MetadataEngine, not all facade/device behavior",
               "differences": ["Pinned Google selects by stts, omits ctts/edit mapping and clamps after the last sample.",
                               "Rust honors composition and exact unit-rate edits, returns no sample outside half-open intervals, and preserves raw packet bytes."]}
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"{len(rows)} timed sample-selection cases passed; receipt: {output / 'receipt.json'}")

if __name__ == "__main__":
    main()
