# liblivephoto-native

[English](README.md)

LibLivePhoto 操作接口的实验性 C 与 JNI 边界。需要 Rust 1.88 或更高版本，将 `livephoto_native` 构建为动态库或静态库；可移植解析器继续使用安全 Rust。

使用打包后的 SDK 获取 C 头文件、Java jar、平台库和依赖声明。响应句柄由 Rust 持有，必须使用配套释放函数。外部指针必须在所声明的长度范围内有效。

详见[SDK 打包与集成](https://github.com/21Z121Z1/LibLivePhoto/blob/main/NATIVE_SDK.zh-CN.md)。采用 Apache-2.0 许可证。
