#!/usr/bin/env python3
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

DRIVER = Path(__file__).with_name("verify_extraction.py")


class ExistingVerificationSession(unittest.TestCase):
    def check_mode(self, mode):
        with tempfile.TemporaryDirectory(prefix="llp-verify-") as directory, tempfile.TemporaryDirectory(prefix="llp-ownership-") as evidence_root:
            scratch = Path(directory).resolve()
            evidence = Path(evidence_root).resolve() / "existing"
            evidence.mkdir()
            marker = scratch / ".llp-verification-owner"
            marker.write_text(str(evidence))
            sentinel = scratch / "active-session"
            sentinel.write_bytes(b"preserve this active verification session\n")
            receipt = evidence / "receipt.json"
            receipt.write_text(json.dumps({"scratch": str(scratch), "commands": [], "verdict": "STARTED"}) + "\n")
            original = {path: path.read_bytes() for path in [marker, sentinel, receipt]}
            result = subprocess.run([sys.executable, str(DRIVER), mode, "--evidence", str(evidence)], capture_output=True, timeout=30)
            unchanged = all(path.exists() and path.read_bytes() == data for path, data in original.items())
            print(json.dumps({"mode": mode, "exit_code": result.returncode, "existing_session_unchanged": unchanged, "receipt_sha256": hashlib.sha256(receipt.read_bytes()).hexdigest()}))
            self.assertTrue(unchanged, "refusing an existing evidence directory must preserve its session")
            self.assertEqual(result.returncode, 2, result.stderr.decode(errors="replace"))

    def test_launch_preserves_existing_session(self):
        self.check_mode("launch")

    def test_run_preserves_existing_session(self):
        self.check_mode("run")


if __name__ == "__main__":
    unittest.main()
