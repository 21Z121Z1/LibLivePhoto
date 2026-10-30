# Verification

[中文](VERIFICATION.zh-CN.md)

Run commands from the repository root. Pass a new directory each time. `evidence/` is gitignored, so receipts stay out of the tree. CI uploads those directories as artifacts.

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
python3 scripts/test_verification_ownership.py
cargo test --locked --workspace --all-targets
```

`verify_extraction.py` builds `llp`, checks the checked-out head, runs the library, interface, and CLI tests, then `replay_extraction_budget.py` and `replay_historical_budgets.py`. The receipt and logs stay in the directory you named. `.github/workflows/verify.yml` runs that command and uploads `evidence/local/verify-extraction`, syntax receipts, and the fuzz logs from `scripts/fuzz.sh`. `.github/workflows/android.yml` and `.github/workflows/apple-web.yml` upload their own run logs.

Differential scripts, `scripts/test_syntax_contracts.py`, and `scripts/fuzz.sh` also write under `evidence/local`, or under a path you pass. They do not update tracked files.

Older research notes and SHA-named receipts are on branch [`archive/evidence-20261030`](https://github.com/21Z121Z1/LibLivePhoto/tree/archive/evidence-20261030/evidence) at `2fb54887aba91c0fbb607120038b93afc7dca8f0`. The index is [Research notes](research.md).

The merge to `main` at `042a188efc2f9c10638a571c3fec57b5032d5870` ran these workflows:

- [Verify independent format library](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330042)
- [Android native services](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330060)
- [Apple adapter and local web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330012)
