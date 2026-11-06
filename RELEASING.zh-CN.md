# LibLivePhoto 发布流程

[English](RELEASING.md)

0.1 版本正式支持 Rust 核心和 `llp`，各平台绑定仍为实验性。GitHub 版本与注册表上传是独立操作，以工作流摘要记录的实际结果为准。

## 安装

从 GitHub 源码安装不要求 crates.io 已存在 LibLivePhoto 的 crate：

```sh
cargo install --git https://github.com/21Z121Z1/LibLivePhoto.git --tag v0.1.0 --locked liblivephoto-cli
llp version
```

0.1.0 在 crates.io 可用后，应用可以使用：

```toml
[dependencies]
liblivephoto = "0.1.0"
```

```sh
cargo install --locked liblivephoto-cli --version 0.1.0
llp version
```

主机 SDK 归档附在对应 GitHub Release 下，使用前验证 SHA-256 文件。集成见[原生 SDK](NATIVE_SDK.zh-CN.md)。

## 验证候选版本

workspace 打包需要 Python 3.11+ 和 Cargo1.90+，消费方需要 Rust1.88+；Cargo1.90 用于多包发布。从干净检出运行：

```sh
cargo +1.88.0 test --locked --workspace --all-targets --all-features
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python3 scripts/check_markdown_links.py
python3 scripts/check_translations.py
python3 scripts/verify_crates.py
```

`verify_crates.py` 对 workspace 发布做 dry-run，解包全部六个归档，再运行测试/示例、doctest、rustdoc、外部 Rust 转换程序及 CLI 安装。在上传注册表前，依赖覆盖只指向已解包归档；上传后单独检查注册表安装。`--allow-dirty` 仅供本地开发。

`tests/data` 下所需合成副本与仓库源逐字节比较，原始私人照片继续排除。原始样本测试保留自行提供输入的要求。包内 README、CLI/SDK 指南及 release 文案提供英文和简体中文；可执行代理指令和第三方原始许可证保持原语言。

四组 main push 工作流必须对精确候选 SHA 通过：格式库验证、Apple/Web、Android 和文档。Linux 独立读取器安装限时五分钟；镜像下载失败时先检查失败日志，再决定是否重跑。维护者已完成实机验证，公开范围见[兼容性](COMPATIBILITY.zh-CN.md#实机验证)。

## 发布已确认的首个版本

一次性的 `Publish approved v0.1.0` 工作流在其工作流文件合入 main 时启动，也可从 main 手动触发。它等待精确 main 的 CI，通过归档/源码/版本哈希检查，整理已有测试产物，再发布 GitHub Release 和配套标签。正文同时包含 `release-notes/v0.1.0` 的中英版本。

版本提供六个 Rust 源码归档、三个主机 SDK 归档、实验性 iOS XCFramework 和 Android AAR，以及 SHA-256 文件和源码/产物清单。独立的 `Prepare release draft` 工作流继续用于手动准备草稿。

若仓库配置 `CARGO_REGISTRY_TOKEN` secret，发布作业会上传 workspace 至 crates.io，并验证全新的纯注册表 Rust 消费程序及 CLI 安装，不打印 token。若未配置，作业明确记录未尝试 crates.io 上传，仍可完成 GitHub 源码/SDK 发布。没有发生的注册表上传不能描述为已可从注册表安装。

已发布 crate 版本不能覆盖。上传部分成功时，应检查注册表状态，只从同一审阅源码发布剩余包；不会静默替换已有版本/标签。0.1.x 约定见[更新记录](CHANGELOG.zh-CN.md#兼容性约定)。
