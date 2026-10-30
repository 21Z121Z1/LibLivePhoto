# 验证记录

[English](VERIFICATION.md)

这些内容原先写在中英文首页上。它们是某一次写文档时留下的实测，不是长期承诺。这次为了好读而改首页，没有把下面的命令再跑一遍。

当时的首页是提交 `af75e81`（`docs(readme): add a bilingual landing page for the preview`），经 `8842459` 合入 `main`。下面引用的 CI 属于 `main` 上的 `042a188efc2f9c10638a571c3fec57b5032d5870`。

## 包的身份

`Cargo.toml` 里的包版本是 0.1.0。`cargo build --locked -p liblivephoto-cli` 之后，`./target/debug/llp version` 打印了 `"version": "0.1.0"` 和 `"success": true`。`git tag` 没有任何标签。向 crates.io API 查询 crate 名 `liblivephoto` 得到 HTTP 404，那次查询没有找到同名的可安装包。

## 工具链

`Cargo.toml` 没有写 `rust-version`。那台机器上，rustc 1.87.0 编不过 `liblivephoto`、`liblivephoto-cli` 和 `liblivephoto-native` 的锁定构建，因为 `slice::as_chunks` 当时还不稳定。调用点在 `crates/liblivephoto-format/src/exif_raw.rs`。同一套锁定构建在 rustc 1.88.0 和 rustc 1.99.1 上通过。除了单独注明的地方，下面的命令用的是 rustc 1.99.1。这是构建日志，不是仓库声明的最低版本。

## 在演示文件上跑过的命令

输入是 `web/public/demo`。`provenance.json` 标明 CC0，并写明不含私人拍摄素材。文件里记下的照片时间是 `13/16`。

`cargo run --locked -p liblivephoto --example inspect -- web/public/demo/motion-photo.jpg` 打印了 `format=AndroidMotionPhoto`、`dialect=AndroidStandard`、4 个资源、1 条轨道，以及 `MediaTime { value: 812500, timescale: 1000000 }`。主视频资源是 19670 字节。

```sh
cargo run --locked -p liblivephoto-cli -- extract web/public/demo/motion-photo.jpg --video --out /tmp/llp-extract
cmp web/public/demo/motion.mp4 /tmp/llp-extract/resource-1.mp4 && echo match
```

抽取以退出码 0 结束。`resource-1.mp4` 为 19670 字节，与 `web/public/demo/motion.mp4` 逐字节相同。对这个样本，抽出的视频就是原来的视频字节。

```sh
cargo run --locked -p liblivephoto-cli -- build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out /tmp/llp-built
cargo run --locked -p liblivephoto-cli -- check /tmp/llp-built/asset-0.jpg
```

构建报告里有 `"canExecute": true`、`"target": "AndroidJpeg"` 和 `"consumerAcceptance": "Unverified"`。视频处置是 `Preserved`。静图处置是 `Rewritten`，因为写入器会换掉容器元数据。随后 `llp check` 对 `ResourceRanges`、`StillContainer`、`MotionContainer` 和 `Tracks` 报告 `"valid": true`。

Samsung SEF 转换示例以退出码 1 结束：

```sh
cargo run --locked -p liblivephoto --example convert -- samsung-jpeg-sef /tmp/llp-samsung web/public/demo/motion-photo.jpg
```

错误文本是 `SEF target has no supported exact presentation-time mapping; use Android or PreserveSource`。

那次没有向 `llp` 提交过分片文件。

## Java

`python3 scripts/test_java.py` 会构建 `liblivephoto-native`，用 `javac --release 8` 编译 Java 源码，并对 `web/public/demo` 运行 `org.liblivephoto.NativeSmoke`。OpenJDK 21.0.12.1 配 rustc 1.99.1 时，脚本打印了 `"smoke": "passed"`。

## Android、Swift 和 WASM

这些命令没有在那台机器上跑。CI 结果属于 `main` 的提交 `042a188efc2f9c10638a571c3fec57b5032d5870`。

`scripts/build_android.py` 会写出 `android/build/liblivephoto-android.aar`。`MotionPhotoService.MAX_INPUT_BYTES` 是 32 MiB，`MetadataParserService` 跑在隔离进程里。那台机器没有 Android SDK，所以脚本没在那里执行。[Android native services](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330060) 的作业 `sdk-and-services` 在该提交上成功。作业会构建 AAR，并跑一台自有的 API 35 x86_64 模拟器。模拟器结果不能当作厂商相册已经验收。

Swift 包在 `apple` 目录。`Package.swift` 声明 Swift tools 5.9、macOS 13、iOS 16，库名是 `LibLivePhotoApple`。那台机器没装 Swift。同一提交上，[Apple adapter and local web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330012) 的作业 `apple` 成功。作业会跑 `swift test --package-path apple`，以及 iOS 模拟器验收。模拟器结果不能当作「照片」已经验收。

WASM 演示在 `web`。同一工作流里的作业 `web` 成功。它使用 `wasm32-unknown-unknown`、`wasm-bindgen-cli` 0.2.104 和 Node 24，然后跑 Node 测试和 Chrome 浏览器测试。那台机器的 Node 是 v22.23.4，那些 Web 命令没有在那里执行。

## 重新生成收据

在仓库根目录运行，并传入一个新目录。`evidence/` 已被 Git 忽略，收据不会进入版本库。CI 会把该目录作为构件上传。

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
```

脚本会构建 `llp`，核对当前检出的提交，运行库、接口和 CLI 测试，然后运行 `replay_extraction_budget.py` 与 `replay_historical_budgets.py`。差分脚本、`scripts/test_syntax_contracts.py` 和 `scripts/fuzz.sh` 也写到 `evidence/local`。它们不修改已跟踪的文件。

较早的研究笔记和以提交 SHA 命名的收据在分支 [`archive/evidence-20261030`](https://github.com/21Z121Z1/LibLivePhoto/tree/archive/evidence-20261030/evidence)，提交 `2fb54887aba91c0fbb607120038b93afc7dca8f0`。索引见[研究笔记](research.zh-CN.md)。
