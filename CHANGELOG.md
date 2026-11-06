# Changelog

[简体中文](CHANGELOG.zh-CN.md)

## 0.1.0 — 2026-11-06

First ordinary release of the Rust core and `llp` CLI.
GitHub SDK/source releases and crates.io uploads have separate availability.

- Inspect, extract, structurally check and convert supported Apple Live Photo,
  Android Motion Photo and Samsung SEF layouts without transcoding media.
- Preserve exact resource bytes and rational photo time under explicit conversion
  and unmapped-resource policies; expose bounded vendor metadata inspection.
- Declare Rust 1.88 as the minimum supported toolchain.
- Provide English and Simplified Chinese documentation and release notes.
- Include the synthetic data required by the crate tests and examples, package
  READMEs and license texts, and verify consumers outside the source repository.
- Provide host SDK archives with dependency notices, source identity and hashes.
- Keep C/JNI, Android AAR, Swift, WASM and `xdremux-compat` experimental.

The maintainer reports device verification complete. The public record does not
yet itemize device/OS/operation combinations; see [Compatibility](COMPATIBILITY.md#device-verification).
Automated build receipts describe their own scope.

## Compatibility policy

For the supported Rust core and `llp`, patch releases in 0.1.x preserve public
source compatibility and existing CLI option/JSON field meanings. Breaking
changes require a new minor version. JSON consumers must accept additional
fields and diagnostics; human-readable messages are not stable identifiers.
The minimum Rust version remains 1.88 throughout 0.1.x. Experimental bindings and
`xdremux-compat` are outside this guarantee and will document incompatible changes.
