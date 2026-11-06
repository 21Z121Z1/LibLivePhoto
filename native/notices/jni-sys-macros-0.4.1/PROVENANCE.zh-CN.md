
[English](PROVENANCE.md)

注册表包 `jni-sys-macros 0.4.1` 声明 MIT OR Apache-2.0，但遗漏了 workspace 许可证文件。包内 `.cargo_vcs_info.json` 固定到上游 `jni-rs/jni-sys` 提交 `64d77b7a5f119d7b55b4e2c169a4668067ff59e6`，路径为 `jni-sys-macros`。此处两份未经改写的许可证读取自该精确提交的根目录，而非后续默认分支：

- https://github.com/jni-rs/jni-sys/blob/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-APACHE
- https://github.com/jni-rs/jni-sys/blob/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-MIT

保留的是该依赖自身的许可选项，不把 Apache-2.0 统一套用于其他依赖。
