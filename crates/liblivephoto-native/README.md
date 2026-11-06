# liblivephoto-native

[简体中文](README.zh-CN.md)

Experimental C and JNI boundary for the LibLivePhoto operation interface.
Rust 1.88 or later. It builds `livephoto_native` as a shared/static library;
the portable parsers remain in safe Rust.

Use the packaged SDK for the C header, Java jar, platform libraries and dependency
notices. Response handles are owned by Rust and require the matching free call.
Foreign pointers must be valid for the declared lengths.

[SDK packaging and integration](https://github.com/21Z121Z1/LibLivePhoto/blob/main/NATIVE_SDK.md).
Apache-2.0.
