# llp

[English](README.md)

LibLivePhoto 命令行接口，需要 Rust 1.88 或更高版本。

此版本上传 crates.io 后，可使用：

```sh
cargo install --locked liblivephoto-cli --version 0.1.0
llp version
llp parse input.jpg
llp extract input.jpg --video --out extracted
llp check input.jpg
```

输出目录必须尚不存在。`llp --help` 列出构建/转换、元数据和区间选项。`--target apple-pvt` 发布 `.pvt` 目录，并把 JSON 报告打印到标准输出。退出状态 0 表示成功，1 表示操作报告失败，2 表示参数或 I/O 错误。JSON 输出和错误代码用于自动化；面向人的诊断措辞可能改变。

合计输入上限为 128 MiB，不运行转码。0.1.x 的 CLI 选项和已有 JSON 字段在补丁版本间保持含义一致，调用方应容忍新增 JSON 字段与诊断。

详见 [CLI 参考](https://github.com/21Z121Z1/LibLivePhoto/blob/main/CLI.zh-CN.md)。采用 Apache-2.0 许可证。
