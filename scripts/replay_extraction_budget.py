#!/usr/bin/env python3
"""Replay the actual process path with a lowered response cap and copy counter."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "b1c556314b0939a6370f68fcd8be63089f7b3883"
INTERFACE = Path("crates/liblivephoto-interface/src/lib.rs")
COUNTERS = """pub static REPLAY_LIMIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(8192);
pub static REPLAY_COPIED_BYTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn replay_limit() -> usize { REPLAY_LIMIT.load(std::sync::atomic::Ordering::SeqCst) }
"""


def git(repo, *args):
    return subprocess.check_output(["git", *args], cwd=repo, stderr=subprocess.STDOUT)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def instrument(source, baseline):
    if baseline:
        old = "320 * 1024 * 1024"
        anchor = "    photo.extract(resource, &mut bytes)?;"
    else:
        old = "const MAX_RESPONSE: usize = 320 * 1024 * 1024;"
        anchor = """                    self.photo
                        .extract(resource, &mut bytes)
                        .map_err(|e| e.to_string())?;"""
    if source.count(old) != 1 or source.count(anchor) != 1:
        raise RuntimeError("extraction replay seam changed; inspect the real materializer")
    source = source.replace(old, "replay_limit()" if baseline else "")
    if not baseline:
        source = source.replace("total <= MAX_RESPONSE", "total <= replay_limit()")
    source = source.replace("pub const MAX_INPUT:", COUNTERS + "pub const MAX_INPUT:", 1)
    return source.replace(anchor, anchor + "\nREPLAY_COPIED_BYTES.fetch_add(bytes.len(), std::sync::atomic::Ordering::SeqCst);")


def run(repo, env, mode, log, output):
    command = ["cargo", "run", "--locked", "--quiet", "-p", "liblivephoto-interface", "--example", "extraction_budget", "--", mode, str(output)]
    with log.open("w") as stream:
        result = subprocess.run(command, cwd=repo, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=300)
    text = log.read_text()
    return {"command": command, "exit_code": result.returncode, "log": str(log), "log_sha256": digest(log.read_bytes()), "assertion": "panicked at" in text and "assertion" in text and "error[E" not in text}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=ROOT)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    source = args.source.resolve()
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    if subprocess.run(["git", "cat-file", "-e", f"{BASELINE}^{{commit}}"], cwd=source, capture_output=True).returncode:
        git(source, "fetch", "--no-tags", "origin", BASELINE)
    receipt = {"baseline": BASELINE, "head": git(source, "rev-parse", "HEAD").decode().strip(), "driver_sha256": digest(Path(__file__).read_bytes()), "probe_sha256": digest((ROOT / "scripts/probes/extraction_budget.rs").read_bytes()), "seam": "Only the response cap and a counter after actual Vec extraction are changed in scratch interface sources. Planning sink writes are excluded. Production limits and APIs remain unchanged.", "runs": []}
    try:
        with tempfile.TemporaryDirectory(prefix="llp-extraction-replay-") as directory:
            scratch = Path(directory)
            env = dict(os.environ, CARGO_TARGET_DIR=str(scratch / "target"))
            for variant in ["baseline", "candidate"]:
                repo = scratch / variant
                git(scratch, "clone", "--local", "--no-hardlinks", "--quiet", str(source), str(repo))
                if variant == "baseline":
                    git(repo, "checkout", "--quiet", "--detach", BASELINE)
                    original = git(source, "show", f"{BASELINE}:{INTERFACE.as_posix()}").decode()
                else:
                    original = (source / INTERFACE).read_text()
                modified = instrument(original, variant == "baseline")
                (repo / INTERFACE).write_text(modified)
                examples = repo / "crates/liblivephoto-interface/examples"
                examples.mkdir(exist_ok=True)
                (examples / "extraction_budget.rs").write_bytes((ROOT / "scripts/probes/extraction_budget.rs").read_bytes())
                record = {"variant": variant, "source_sha256": digest(original.encode()), "instrumented_sha256": digest(modified.encode()), "runs": []}
                receipt["runs"].append(record)
                (evidence / f"{variant}-source.rs").write_text(modified)
                for mode in ["compatibility", "proof"]:
                    run_record = run(repo, env, mode, evidence / f"{variant}-{mode}.log", evidence / f"{variant}-output.json")
                    expected = 101 if variant == "baseline" and mode == "proof" else 0
                    run_record["accepted"] = run_record["exit_code"] == expected and (expected == 0 or run_record["assertion"])
                    record["runs"].append(run_record)
                    if not run_record["accepted"]:
                        raise RuntimeError(f"{variant} {mode} failed its exit/assertion contract; see {run_record['log']}")
            baseline = (evidence / "baseline-output.json").read_bytes()
            candidate = (evidence / "candidate-output.json").read_bytes()
            if baseline != candidate:
                raise RuntimeError("supported output differs from the baseline")
            receipt["supported_output_sha256"] = digest(candidate)
            receipt["verdict"] = "PASS"
    except Exception as error:
        receipt["verdict"] = "FAIL"
        receipt["error"] = str(error)
        raise
    finally:
        (evidence / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(evidence / "receipt.json", flush=True)


if __name__ == "__main__":
    main()
