# Releasing LibLivePhoto

0.1.0 is in preparation. No registry upload or public release is performed by PR
checks. Rust core and `llp` are the supported release surface. The platform
bindings remain experimental and are included as SDK assets.

## Install a published version

These commands become available after 0.1.0 appears on crates.io:

```toml
[dependencies]
liblivephoto = "0.1.0"
```

```sh
cargo install --locked liblivephoto-cli --version 0.1.0
llp version
```

Before that, use the source build in the [README](README.md). Host SDK archives
are attached to the versioned GitHub Release; verify the accompanying SHA-256
file before use. Swift/Android integrations follow [Native SDK](NATIVE_SDK.md).

## Verify the candidate

Use Python 3.11+ and Cargo 1.90+ for workspace packaging. Consumers only need Rust
1.88+. Cargo 1.90 is needed for multi-package publishing, not for the library API.
Run these commands from a clean checkout of the intended commit:

```sh
cargo +1.88.0 test --locked --workspace --all-targets --all-features
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python3 scripts/check_markdown_links.py
python3 scripts/verify_crates.py
```

`verify_crates.py` runs `cargo publish --workspace --dry-run --locked`, then
extracts all six `.crate` archives into a temporary workspace. It runs all their
tests/examples, doctests, rustdoc with warnings denied, an external Rust conversion
consumer and a CLI installation. Before the first upload, patches resolve local
dependencies to those extracted archives. They never point to repository sources.
This proves archive contents and consumption; the final registry upload/download
is a separate check. `--allow-dirty` is available for local development only.

Required synthetic fixture copies live under each crate's `tests/data`, preserving
the original repository-relative layout and provenance. The verifier compares
them with their original files byte for byte. Update the copies whenever an
original changes. Private originals are not included; existing external-corpus
tests keep their original opt-in requirements.

All four CI workflows must pass on the exact candidate SHA: format-library
verification (including Rust 1.88 and crate contents), Apple/web, Android, and docs.
The maintainer has completed device verification; retain the actual coverage in
the [compatibility record](COMPATIBILITY.md#device-verification).

## Prepare a GitHub Release draft

After the change is merged, dispatch **Prepare release draft** on `main` with the
full SHA of the current `main` head and `version: 0.1.0`. It checks versions and
the four successful push workflows for that SHA, downloads their existing
artifacts, verifies their source receipts, and creates a draft containing:

- all six Rust source packages;
- the three host SDK archives and SHA-256 files;
- the iOS XCFramework archive and Android AAR, marked experimental;
- a release manifest with the source SHA and asset hashes.

The workflow creates a draft only. It does not publish to crates.io or publish
the GitHub Release. It refuses an existing release/tag rather than replacing it.
Review the draft, compatibility scope and notices before publication.

## Publish and check installation

With publishing credentials configured by the maintainer, run from the reviewed
commit using Cargo 1.90+:

```sh
cargo publish --workspace --locked
```

Cargo orders the format, core, bridge, interface, CLI and native crates by their
dependencies. Published versions cannot be overwritten. If an earlier attempt
partly uploaded, inspect the registry and publish only the remaining packages;
do not change a version's content and retry it.

After upload, create a new Rust project outside the repository with the registry
dependency shown above. Run its parse/extract example on a known fixture, and run
the CLI installation command without `--path`, patches or source overrides.
Confirm `llp version` reports 0.1.0. This verifies registry distribution separately
from archive testing. Then publish the reviewed GitHub Release draft, creating
the matching `v0.1.0` tag, and replace the preparation wording/date in the docs.

0.1.x compatibility commitments are in [Changelog](CHANGELOG.md#compatibility-policy).
