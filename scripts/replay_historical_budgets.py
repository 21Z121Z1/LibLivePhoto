#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

CASES = {
    "E1": {
        "parent": "5396a1be5cace7cf179721c6f91f1226ed1bbf64",
        "fix": "82abd717963f8c6e78ddce3babf22f49427c8048",
        "test_paths": ["crates/liblivephoto/tests/video_metadata.rs"],
        "production_paths": [
            "crates/liblivephoto/src/metadata_samples.rs",
            "crates/liblivephoto/src/video_metadata.rs",
        ],
        "tests": [
            ("scoring", "liblivephoto", "video_metadata", "scoring_extraction_bounds_total_samples_records_and_copied_payload"),
            ("nodes", "liblivephoto", "video_metadata", "tiny_unknown_descriptors_have_a_count_budget_as_well_as_a_byte_budget"),
        ],
        "fixture_review": "4097 two-byte samples; 7 samples with 10000 two-byte descriptors; 17 one-MiB samples. Largest movie payload is 17 MiB. Dense descriptor input is below 40000 bytes.",
    },
    "E2": {
        "parent": "82abd717963f8c6e78ddce3babf22f49427c8048",
        "fix": "0651b8020a6f4a41a353e9937932b902f1687966",
        "test_paths": [
            "crates/liblivephoto/tests/agtm_complete.rs",
            "crates/liblivephoto/tests/common/mod.rs",
        ],
        "production_paths": ["crates/liblivephoto/src/agtm.rs"],
        "tests": [
            ("agtm", "liblivephoto", "agtm_complete", "overlapping_agtm_samples_have_aggregate_count_payload_and_curve_budgets"),
        ],
        "fixture_review": "4096 allowed and 4097 rejected two-byte samples; 257 samples of 65536 bytes; 497 maximum-curve samples. Largest movie payload is 16842752 bytes.",
    },
    "E3": {
        "parent": "0651b8020a6f4a41a353e9937932b902f1687966",
        "fix": "71818fbc4d724dea4e326ac642142a5d60e4fd57",
        "prerequisite": "cd2159d375d22498c9437b7df87c2546700a9288",
        "test_paths": ["crates/liblivephoto/tests/common/mod.rs"],
        "production_paths": ["crates/liblivephoto-bridge/src/lib.rs"],
        "tests": [
            ("bridge", "liblivephoto-bridge", None, "tests::frame_manifest_budget_is_shared_across_video_tracks"),
        ],
        "fixture_review": "Two tracks reference one 60000-byte mdat. The red path constructs 120000 frame records. The green path constructs 60000 and reports the second track unavailable. No large media file or OOM input is generated.",
    },
}
MARKER = b"#[cfg(test)]\n"


def git(repo, *args, data=None):
    return subprocess.check_output(["git", *args], cwd=repo, input=data, stderr=subprocess.STDOUT)


def blob(source, commit, path):
    return git(source, "show", f"{commit}:{path}")


def apply_diff(source, repo, parent, fix, paths):
    patch = git(source, "diff", parent, fix, "--", *paths)
    git(repo, "apply", "-", data=patch)


def fingerprint(data):
    return hashlib.sha256(data).hexdigest()


def production_bytes(case_id, data):
    return data.split(MARKER, 1)[0] if case_id == "E3" else data


def verify_production(source, repo, case_id, case, commit):
    hashes = {}
    for path in case["production_paths"]:
        actual = production_bytes(case_id, (repo / path).read_bytes())
        expected = production_bytes(case_id, blob(source, commit, path))
        if actual != expected:
            raise RuntimeError(f"{case_id} production differs from {commit} at {path}")
        hashes[path] = fingerprint(actual)
    return hashes


def run_test(repo, env, prefix, case_id, case, test, stage):
    label, package, target, name = test
    command = ["cargo", "test", "--locked", "-p", package]
    command.extend(["--test", target] if target else ["--lib"])
    command.extend([name, "--", "--exact"])
    path = Path(f"{prefix}-{case_id}-{label}-{stage}.log")
    with path.open("w") as out:
        out.write(f"parent={case['parent']}\nfix={case['fix']}\ncwd={repo}\ncommand={shlex.join(command)}\n")
        out.flush()
        result = subprocess.run(command, cwd=repo, env=env, stdout=out, stderr=subprocess.STDOUT, timeout=180)
        out.write(f"\nexit_code={result.returncode}\n")
    output = path.read_text()
    panics = [line for line in output.splitlines() if "panicked at" in line or "assertion" in line or line.strip().startswith(("left:", "right:"))]
    if stage == "red":
        accepted = (
            result.returncode == 101
            and "running 1 test" in output
            and f"test {name} ... FAILED" in output
            and "panicked at" in output
            and "assertion" in output
            and "test result: FAILED. 0 passed; 1 failed;" in output
            and "error[E" not in output
        )
    else:
        accepted = result.returncode == 0 and f"test {name} ... ok" in output and "test result: ok. 1 passed; 0 failed;" in output
    print(f"{case_id} {label} {stage}: exit {result.returncode}, evidence {'accepted' if accepted else 'rejected'}, {path}", flush=True)
    return {"stage": stage, "name": name, "command": command, "cwd": str(repo), "exit_code": result.returncode, "accepted": accepted, "assertion_lines": panics, "log": str(path), "log_sha256": fingerprint(path.read_bytes())}


def replay(source, scratch, prefix, case_id, case):
    repo = scratch / case_id
    git(scratch, "-c", "advice.detachedHead=false", "clone", "--local", "--no-hardlinks", "--quiet", str(source), str(repo))
    git(repo, "-c", "advice.detachedHead=false", "checkout", "--quiet", "--detach", case["parent"])
    apply_diff(source, repo, case["parent"], case["fix"], case["test_paths"])
    if case_id == "E3":
        apply_diff(source, repo, case["fix"], case["prerequisite"], ["Cargo.lock", "crates/liblivephoto-bridge/Cargo.toml"])
        path = case["production_paths"][0]
        parent = blob(source, case["parent"], path)
        fixed = blob(source, case["fix"], path)
        (repo / path).write_bytes(parent.split(MARKER, 1)[0] + MARKER + fixed.split(MARKER, 1)[1])
    record = {"id": case_id, **case, "repo": str(repo), "runs": []}
    record["red_production_sha256"] = verify_production(source, repo, case_id, case, case["parent"])
    record["test_sha256"] = {path: fingerprint((repo / path).read_bytes()) for path in case["test_paths"]}
    if case_id == "E3":
        record["test_suffix_sha256"] = fingerprint(MARKER + (repo / case["production_paths"][0]).read_bytes().split(MARKER, 1)[1])
    env = dict(os.environ, CARGO_TARGET_DIR=str(scratch.parent / "target"))
    for test in case["tests"]:
        record["runs"].append(run_test(repo, env, prefix, case_id, case, test, "red"))
    if not all(run["accepted"] for run in record["runs"]):
        record["verdict"] = "INVALID_RED"
        return record
    if case_id == "E3":
        path = case["production_paths"][0]
        current = (repo / path).read_bytes()
        fixed = blob(source, case["fix"], path)
        (repo / path).write_bytes(fixed.split(MARKER, 1)[0] + MARKER + current.split(MARKER, 1)[1])
    else:
        apply_diff(source, repo, case["parent"], case["fix"], case["production_paths"])
    record["green_production_sha256"] = verify_production(source, repo, case_id, case, case["fix"])
    for path, expected in record["test_sha256"].items():
        if fingerprint((repo / path).read_bytes()) != expected:
            raise RuntimeError(f"{case_id} test changed between red and green")
    if case_id == "E3":
        if fingerprint(MARKER + (repo / case["production_paths"][0]).read_bytes().split(MARKER, 1)[1]) != record["test_suffix_sha256"]:
            raise RuntimeError("E3 test suffix changed between red and green")
    for test in case["tests"]:
        record["runs"].append(run_test(repo, env, prefix, case_id, case, test, "green"))
    record["final_diff_stat"] = git(repo, "diff", "--stat").decode()
    record["verdict"] = "PASS" if all(run["accepted"] for run in record["runs"]) else "FAIL"
    return record


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, default=ROOT)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--case", choices=list(CASES), action="append")
    args = parser.parse_args()
    args.source = args.source.resolve()
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    prefix = evidence / "history"
    selected = args.case or list(CASES)
    for case_id in selected:
        case = CASES[case_id]
        for commit in [case["parent"], case["fix"], *([case["prerequisite"]] if "prerequisite" in case else [])]:
            if subprocess.run(["git", "cat-file", "-e", f"{commit}^{{commit}}"], cwd=args.source, capture_output=True).returncode:
                git(args.source, "fetch", "--no-tags", "origin", commit)
    receipt = {"source": str(args.source), "source_head": git(args.source, "rev-parse", "HEAD").decode().strip(), "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(), "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(), "dependency_mode": "locked", "recipe_sha256": fingerprint(Path(__file__).read_bytes()), "cases": []}
    receipt_path = evidence / "receipt.json"
    try:
        with tempfile.TemporaryDirectory(prefix="llp-history-replay-") as directory:
            scratch = Path(directory) / "repos"
            scratch.mkdir()
            for case_id in selected:
                receipt["cases"].append(replay(args.source, scratch, prefix, case_id, CASES[case_id]))
        receipt["scratch_removed"] = not scratch.exists()
        receipt["verdict"] = "PASS" if all(case["verdict"] == "PASS" for case in receipt["cases"]) else "FAIL"
    except Exception as error:
        receipt["verdict"] = "ERROR"
        receipt["error"] = repr(error)
        raise
    finally:
        receipt_path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n")
        print(f"receipt={receipt_path}", flush=True)
    return 0 if receipt["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
