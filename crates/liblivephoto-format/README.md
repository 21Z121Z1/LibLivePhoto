# liblivephoto-format

[简体中文](README.zh-CN.md)

Bounds-checked JPEG, EXIF/TIFF, HEIF and ISO BMFF primitives used by LibLivePhoto.
Rust 1.88 or later; no platform SDK or codec dependency.

Most applications should use the higher-level `liblivephoto` crate. These
primitives inspect container structure and do not decode image or audio pixels.
The 0.1.x API preserves source compatibility across patch releases; breaking
changes use a new minor version.

[Project documentation](https://github.com/21Z121Z1/LibLivePhoto). Apache-2.0.
