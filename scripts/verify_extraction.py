#!/usr/bin/env python3
"""Run the maintained library, interface, CLI, and budget replay checks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def head():
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def save(evidence, state):
    (evidence / "receipt.json").write_text(json.dumps(state, indent=2) + "\n")


def command(evidence, state, name, args):
    log = evidence / f"{name}.log"
    env = dict(os.environ, CARGO_TARGET_DIR=str(Path(state["scratch"]) / "target"))
    with log.open("x") as stream:
        stream.write(json.dumps(args) + "\n")
        stream.flush()
        result = subprocess.run(args, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=900)
    state["commands"].append({"name": name, "args": args, "exit_code": result.returncode, "log": str(log), "sha256": digest(log)})
    save(evidence, state)
    if result.returncode:
        raise RuntimeError(f"{name} exited {result.returncode}; see {log}")
    return log.read_text()


def launch(evidence):
    scratch = Path(tempfile.mkdtemp(prefix="llp-verify-")).resolve()
    (scratch / ".llp-verification-owner").write_text(str(evidence))
    state = {"head": head(), "scratch": str(scratch), "commands": [], "driver_sha256": digest(Path(__file__)), "verdict": "STARTED"}
    save(evidence, state)
    command(evidence, state, "launch", ["cargo", "build", "--locked", "-p", "liblivephoto-cli", "--bin", "llp"])
    binary = scratch / "target/debug" / ("llp.exe" if os.name == "nt" else "llp")
    state.update(binary=str(binary), binary_sha256=digest(binary), launched=True)
    save(evidence, state)
    return state


def doctor(evidence, state):
    if state["head"] != head() or digest(Path(state["binary"])) != state["binary_sha256"]:
        raise RuntimeError("build identity changed; launch a new verification directory")
    output = command(evidence, state, "doctor", [state["binary"], "--version"])
    if "LLP1" not in output:
        raise RuntimeError("CLI did not report the LLP1 protocol")
    state["doctor"] = True
    save(evidence, state)


def drive(evidence, state):
    if not state.get("doctor") or state["head"] != head():
        raise RuntimeError("run doctor on the current committed head before drive")
    command(evidence, state, "session-ownership", [sys.executable, str(ROOT / "scripts/test_verification_ownership.py")])
    command(evidence, state, "library", ["cargo", "test", "--locked", "-p", "liblivephoto", "--test", "core_contract", "--test", "video_metadata", "--test", "agtm_complete"])
    command(evidence, state, "bridge", ["cargo", "test", "--locked", "-p", "liblivephoto-bridge", "--lib"])
    command(evidence, state, "interface-cli", ["cargo", "test", "--locked", "-p", "liblivephoto-interface", "-p", "liblivephoto-cli", "--all-targets"])
    for script, name in [("replay_extraction_budget.py", "extraction"), ("replay_historical_budgets.py", "history")]:
        output = evidence / name
        command(evidence, state, name, [sys.executable, str(ROOT / "scripts" / script), "--evidence", str(output)])
        receipt = json.loads((output / "receipt.json").read_text())
        if receipt["verdict"] != "PASS":
            raise RuntimeError(f"{name} replay did not pass")
        records = receipt.get("cases", receipt.get("runs", []))
        for record in records:
            for run in record["runs"]:
                if digest(Path(run["log"])) != run["log_sha256"]:
                    raise RuntimeError(f"{name} replay log hash differs")
    state.update(driven=True, verdict="PASS")
    save(evidence, state)


def cleanup(evidence, state):
    scratch = Path(state["scratch"])
    if scratch.exists():
        marker = scratch / ".llp-verification-owner"
        if scratch.parent != Path(tempfile.gettempdir()).resolve() or not scratch.name.startswith("llp-verify-") or marker.read_text() != str(evidence):
            raise RuntimeError("cleanup refused an unowned scratch directory")
        shutil.rmtree(scratch)
    state["scratch_removed"] = not scratch.exists()
    save(evidence, state)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["launch", "doctor", "drive", "cleanup", "run"])
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    if args.mode in ["launch", "run"]:
        try:
            evidence.mkdir(parents=True, exist_ok=False)
        except FileExistsError:
            parser.error("evidence directory already exists; choose a new directory")
    state = None
    try:
        if args.mode in ["launch", "run"]:
            state = launch(evidence)
        else:
            state = json.loads((evidence / "receipt.json").read_text())
        if args.mode in ["doctor", "run"]:
            doctor(evidence, state)
        if args.mode in ["drive", "run"]:
            drive(evidence, state)
        if args.mode == "cleanup":
            cleanup(evidence, state)
    except Exception as error:
        if state is None and (evidence / "receipt.json").exists():
            state = json.loads((evidence / "receipt.json").read_text())
        if state is not None:
            state.update(verdict="FAIL", error=str(error))
            save(evidence, state)
            cleanup(evidence, state)
        raise
    finally:
        if args.mode == "run" and state is not None:
            cleanup(evidence, state)
    print(evidence / "receipt.json")


if __name__ == "__main__":
    main()
