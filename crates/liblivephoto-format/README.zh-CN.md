# liblivephoto-format

[English](README.md)

LibLivePhoto 使用的 JPEG、EXIF/TIFF、HEIF 和 ISO BMFF 基础结构，包含边界检查。需要 Rust 1.88 或更高版本，不依赖平台 SDK 或编解码器。

大多数应用应使用上层 `liblivephoto` crate。这些基础结构检查容器，不解码图像或音频数据。0.1.x 的补丁版本保持源码兼容；不兼容改动使用新的次版本号。

详见[项目文档](https://github.com/21Z121Z1/LibLivePhoto)。采用 Apache-2.0 许可证。
