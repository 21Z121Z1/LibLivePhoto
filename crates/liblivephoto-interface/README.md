# liblivephoto-interface

[简体中文](README.zh-CN.md)

Shared parse, extract, build, check and metadata operations behind `llp`, JNI,
Swift and WASM. Rust 1.88 or later. This adapter crate is experimental.

Requests are JSON; responses retain original bytes in the versioned LLP1 binary
envelope. Combined input is limited to 128 MiB. Publication of output files
belongs to the caller. Core Rust applications should use `liblivephoto` directly.

[Protocol and SDK documentation](https://github.com/21Z121Z1/LibLivePhoto/blob/main/NATIVE_SDK.md).
Apache-2.0.
