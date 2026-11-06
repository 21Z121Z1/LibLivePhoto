# liblivephoto-interface

[English](README.md)

为 `llp`、JNI、Swift 与 WASM 提供共用的解析、提取、构建、检查和元数据操作。需要 Rust 1.88 或更高版本。此适配层 crate 仍处于实验阶段。

请求使用 JSON，响应通过带版本的 LLP1 二进制封装保留原始字节。合计输入上限为 128 MiB，输出文件的发布由调用方负责。直接使用核心能力的 Rust 应用应依赖 `liblivephoto`。

详见[协议与 SDK 文档](https://github.com/21Z121Z1/LibLivePhoto/blob/main/NATIVE_SDK.zh-CN.md)。采用 Apache-2.0 许可证。
