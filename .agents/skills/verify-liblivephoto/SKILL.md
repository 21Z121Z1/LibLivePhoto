---
name: verify-liblivephoto
description: Verify LibLivePhoto resource extraction through the Rust library, shared operation interface, and CLI, including response budgets and historical aggregate-budget regressions.
---

# Verify LibLivePhoto extraction

Run commands from the repository root. Use a new evidence directory for each run. Rust, Cargo, Git, and Python 3 are required. Cargo uses the checked-in lockfile. Historical replay fetches the exact listed commits from origin only when local objects are missing.

This is the canonical Codex skill at `.agents/skills/verify-liblivephoto`. It adapts the create-verification-skill workflow's Cursor path to native Codex discovery. Do not add a second copy under `.cursor`.

## Launch

For the complete local and CI command, run:

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
```

For a manual drive, first run:

```sh
python3 scripts/verify_extraction.py launch --evidence evidence/local/verify-extraction
```

Launch claims a new evidence directory with `mkdir` before error recovery can read a receipt. An existing directory exits with code 2 and keeps its session unchanged. Launch builds `llp` in a new temporary target directory. `receipt.json` identifies the committed head, binary hash, and owned scratch directory. A successful build makes the short-lived CLI ready. No server, device, credentials, or user data directory is needed.

## Doctor

```sh
python3 scripts/verify_extraction.py doctor --evidence evidence/local/verify-extraction
```

Doctor reads the build identity and runs `llp --version`. It requires the recorded head and binary hash and the `LLP1` protocol name. Start a new evidence directory after changing commits.

## Drive

```sh
python3 scripts/verify_extraction.py drive --evidence evidence/local/verify-extraction
```

The driver first checks that refused `launch` and `run` commands preserve an existing verification session. It then runs the existing core, interface, and CLI tests, followed by both budget replay scripts. The [feature map](features/README.md) identifies the actual journeys and observable results. The CLI command test builds, checks, extracts, compares output bytes, and verifies refusal to overwrite an existing directory. Each test owns its scratch paths, so concurrent runs do not share output directories.

## Evidence

Keep `receipt.json`, command logs, the extraction replay receipt and source variants, and the historical replay receipt and eight assertion logs. The driver checks their log hashes before reporting `PASS`.

The extraction replay uses the real `process`, parser, and core writer in isolated clones. Its disclosed seam lowers only the response cap and counts completed writes into resource output vectors. It compares supported outputs with the exact baseline. It requires a real assertion failure on the baseline and a green candidate. Compilation errors cannot satisfy red. The historical recipe transplants the original tests without changing their assertions.

These checks prove selected byte preservation, protocol behavior, CLI publication behavior, and the recorded small-budget cases. They do not measure allocator activity, peak memory, speed, decoding, or native Gallery or Photos acceptance. The selected baseline has no Python adapter, so Python adapter verification is outside this map.

## Cleanup

```sh
python3 scripts/verify_extraction.py cleanup --evidence evidence/local/verify-extraction
```

Cleanup removes only the temporary directory with this run's ownership marker. It preserves evidence. Confirm `scratch_removed` is true and every recorded log still exists. `run` performs cleanup on success or failure. For a failed manual step, run cleanup before starting again. Use `/maintain-verification-skill` when extraction behavior changes and the map needs an audit.

## Helpers

All helpers run with Python 3. For just one replay, use a new directory:

```sh
python3 scripts/replay_extraction_budget.py --evidence evidence/local/extraction-replay
python3 scripts/replay_historical_budgets.py --evidence evidence/local/history-replay
```
