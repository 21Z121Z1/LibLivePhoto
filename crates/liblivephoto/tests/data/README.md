# Packaged test data

[简体中文](README.zh-CN.md)

These are byte-identical copies of the repository fixtures needed by this crate.
They allow the published source package to run its tests and examples without a
repository checkout. `scripts/verify_crates.py` checks every mirrored file before
packaging; update a copy from the same repository-relative source path.

Files under `web/` and `fixtures/bindings/` retain their original CC0 provenance
JSON. Those JSON files describe the complete original set; this directory carries
only the required subset. The syntax and vendor-selection vectors are project-owned
and use Apache-2.0. `fixtures/manifest.json` contains hashes of external originals;
those photos are not included. Existing ignored external-corpus tests still require
caller-supplied originals.
