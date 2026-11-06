# liblivephoto-bridge

[English](README.md)

为 LibLivePhoto 的原生与 WASM 客户端提供实验性的自持有检查和精确时间边界。需要 Rust 1.88 或更高版本。解析、配对和时间规则保留在 Rust 核心中，返回自持有的检查清单。

大多数 Rust 应用应直接依赖 `liblivephoto`。平台适配层及其清单可能在次版本更新时改变，详见[原生 SDK 文档](https://github.com/21Z121Z1/LibLivePhoto/blob/main/NATIVE_SDK.zh-CN.md)。采用 Apache-2.0 许可证。
