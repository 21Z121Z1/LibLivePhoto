# liblivephoto-bridge

[简体中文](README.zh-CN.md)

Experimental owned inspection and exact-time boundary for native and WASM
clients of LibLivePhoto. Rust 1.88 or later. It keeps parser, pairing and timing
rules in the Rust core and returns owned inspection manifests.

Most Rust applications should depend on `liblivephoto` directly. Platform
adapters and their manifests may change in a minor release. See the
[native SDK documentation](https://github.com/21Z121Z1/LibLivePhoto/blob/main/NATIVE_SDK.md).
Apache-2.0.
