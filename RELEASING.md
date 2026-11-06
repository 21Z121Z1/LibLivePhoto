# Releasing LibLivePhoto

[简体中文](RELEASING.zh-CN.md)

Rust core and `llp` are the supported 0.1 release surface. Platform bindings remain experimental. GitHub releases and registry uploads are separate operations; inspect the workflow summary for their actual outcomes.

## Install

GitHub source builds do not require the LibLivePhoto crates to exist on crates.io:

```sh
cargo install --git https://github.com/21Z121Z1/LibLivePhoto.git --tag v0.1.0 --locked liblivephoto-cli
llp version
```

After 0.1.0 is available on crates.io, applications can use:

```toml
[dependencies]
liblivephoto = "0.1.0"
```

```sh
cargo install --locked liblivephoto-cli --version 0.1.0
llp version
```

Host SDK archives are attached to the versioned GitHub Release. Verify their SHA-256 files. Integration: [Native SDK](NATIVE_SDK.md).

## Verify the candidate

Use Python 3.11+ and Cargo 1.90+ for workspace packaging; consumers need Rust 1.88+. Cargo 1.90 is required for multi-package publishing. From a clean checkout:

```sh
cargo +1.88.0 test --locked --workspace --all-targets --all-features
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python3 scripts/check_markdown_links.py
python3 scripts/check_translations.py
python3 scripts/verify_crates.py
```

`verify_crates.py` dry-runs workspace publishing, extracts all six archives and runs tests/examples, doctests, rustdoc, an external Rust converter and a CLI installation. Until registry upload, dependency patches point only to extracted archives. Registry installation is checked separately after upload. `--allow-dirty` is for local development.

Required synthetic copies under `tests/data` are compared byte for byte with the repository sources. Original private photos remain excluded. The original-corpus tests retain their opt-in inputs. Package READMEs, CLI/SDK guides and release notes are supplied in English and Simplified Chinese; executable agent instructions and original third-party license texts keep their original language.

All four main push workflows must succeed on the exact candidate SHA: format-library verification, Apple/web, Android and docs. Linux independent-reader setup has a five-minute deadline. Inspect failed mirror downloads before rerunning a job. The maintainer has completed device verification; published scope is recorded in [Compatibility](COMPATIBILITY.md#device-verification).

## Publish the approved first version

The one-time `Publish approved v0.1.0` workflow starts when its workflow file is merged into main, or by a main-branch manual dispatch. It waits for exact-main CI, verifies archive/source/version hashes, prepares the existing tested artifacts, then publishes the GitHub Release and matching tag. Its release body contains both language versions of `release-notes/v0.1.0`.

The release includes six Rust source archives, three host SDK archives, the experimental iOS XCFramework and Android AAR, SHA-256 files and a source/asset manifest. The separate `Prepare release draft` workflow remains available for manual draft preparation.

If the repository has a `CARGO_REGISTRY_TOKEN` secret, the publication job uploads the workspace to crates.io and verifies a new registry-only Rust consumer and installed CLI. No token is printed. If the secret is absent, the job explicitly records that crates.io upload was not attempted; GitHub source/SDK publication can still complete. Do not describe an absent registry upload as an installed registry release.

Published crate versions cannot be overwritten. If a registry upload partly succeeds, inspect its versions and publish only the remaining packages from the same reviewed source. Existing releases/tags are not silently replaced. The 0.1.x policy is in [Changelog](CHANGELOG.md#compatibility-policy).
