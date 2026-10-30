# 验证

[English](VERIFICATION.md)

在仓库根目录运行。每次传入一个新目录。`evidence/` 已被 Git 忽略，收据不会进入版本库。CI 把这些目录作为构件上传。

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
python3 scripts/test_verification_ownership.py
cargo test --locked --workspace --all-targets
```

`verify_extraction.py` 会构建 `llp`，核对当前检出的提交，运行库、接口和 CLI 测试，然后运行 `replay_extraction_budget.py` 与 `replay_historical_budgets.py`。收据和日志留在你指定的目录里。`.github/workflows/verify.yml` 运行同一条命令，并上传 `evidence/local/verify-extraction`、语法收据，以及 `scripts/fuzz.sh` 的日志。`.github/workflows/android.yml` 和 `.github/workflows/apple-web.yml` 上传各自的运行日志。

差分脚本、`scripts/test_syntax_contracts.py` 和 `scripts/fuzz.sh` 也写到 `evidence/local`，或写到你传入的路径。它们不修改已跟踪的文件。

较早的研究笔记和以提交 SHA 命名的收据在分支 [`archive/evidence-20261030`](https://github.com/21Z121Z1/LibLivePhoto/tree/archive/evidence-20261030/evidence)，提交 `2fb54887aba91c0fbb607120038b93afc7dca8f0`。索引见[研究笔记](research.zh-CN.md)。

合并进 `main` 的提交 `042a188efc2f9c10638a571c3fec57b5032d5870` 跑过这些工作流：

- [Verify independent format library](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330042)
- [Android native services](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330060)
- [Apple adapter and local web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330012)
