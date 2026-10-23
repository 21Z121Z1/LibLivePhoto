#!/usr/bin/env python3
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

DRIVER = Path(__file__).with_name("verify_extraction.py")
CLAIM_BEFORE_MKDIR = """
import runpy
import sys
from pathlib import Path
from unittest.mock import patch

driver, mode, target, receipt = sys.argv[1:]
target = Path(target)
mkdir = Path.mkdir
claimed = False

def claim_then_mkdir(path, *args, **kwargs):
    global claimed
    if path == target and not claimed:
        claimed = True
        mkdir(path, parents=True, exist_ok=False)
        (path / "receipt.json").write_text(receipt)
    return mkdir(path, *args, **kwargs)

sys.argv = [driver, mode, "--evidence", str(target)]
with patch.object(Path, "mkdir", claim_then_mkdir):
    runpy.run_path(driver, run_name="__main__")
"""


class ExistingVerificationSession(unittest.TestCase):
    def check_mode(self, mode, interleaved=False):
        with tempfile.TemporaryDirectory(prefix="llp-verify-") as directory, tempfile.TemporaryDirectory(prefix="llp-ownership-") as evidence_root:
            scratch = Path(directory).resolve()
            evidence = Path(evidence_root).resolve() / "existing"
            marker = scratch / ".llp-verification-owner"
            marker.write_text(str(evidence))
            sentinel = scratch / "active-session"
            sentinel.write_bytes(b"preserve this active verification session\n")
            receipt = evidence / "receipt.json"
            receipt_text = json.dumps({"scratch": str(scratch), "commands": [], "verdict": "STARTED"}) + "\n"
            original = {marker: marker.read_bytes(), sentinel: sentinel.read_bytes(), receipt: receipt_text.encode()}
            if interleaved:
                command = [sys.executable, "-c", CLAIM_BEFORE_MKDIR, str(DRIVER), mode, str(evidence), receipt_text]
            else:
                evidence.mkdir()
                receipt.write_text(receipt_text)
                command = [sys.executable, str(DRIVER), mode, "--evidence", str(evidence)]
            result = subprocess.run(command, capture_output=True, timeout=30)
            unchanged = all(path.exists() and path.read_bytes() == data for path, data in original.items())
            print(json.dumps({"mode": mode, "claim_before_mkdir": interleaved, "exit_code": result.returncode, "existing_session_unchanged": unchanged, "receipt_sha256": hashlib.sha256(receipt.read_bytes()).hexdigest()}))
            self.assertTrue(unchanged, "refusing an existing evidence directory must preserve its session")
            self.assertEqual(result.returncode, 2, result.stderr.decode(errors="replace"))

    def test_launch_preserves_existing_session(self):
        self.check_mode("launch")

    def test_run_preserves_existing_session(self):
        self.check_mode("run")

    def test_launch_preserves_session_claimed_before_mkdir(self):
        self.check_mode("launch", interleaved=True)

    def test_run_preserves_session_claimed_before_mkdir(self):
        self.check_mode("run", interleaved=True)

    def test_failed_build_cleans_up_its_own_session(self):
        with tempfile.TemporaryDirectory(prefix="llp-build-failure-") as directory:
            root = Path(directory)
            evidence = root / "new-evidence"
            env = dict(os.environ, RUSTC=str(root / "missing-rustc"))
            result = subprocess.run([sys.executable, str(DRIVER), "launch", "--evidence", str(evidence)], capture_output=True, env=env, timeout=30)
            receipt = json.loads((evidence / "receipt.json").read_text())
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(receipt["verdict"], "FAIL")
            self.assertEqual(receipt["commands"][0]["name"], "launch")
            self.assertNotEqual(receipt["commands"][0]["exit_code"], 0)
            self.assertTrue(receipt["scratch_removed"])
            self.assertFalse(Path(receipt["scratch"]).exists())
            print(json.dumps({"mode": "launch", "forced_build_failure": True, "own_scratch_removed": receipt["scratch_removed"]}))


if __name__ == "__main__":
    unittest.main()
