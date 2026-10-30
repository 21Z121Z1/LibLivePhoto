# LibLivePhoto

[English](README.md)

LibLivePhoto 用来读取、检查和生成 Motion Photo 与 Live Photo 文件。Rust 库和 `llp` 命令是使用这棵代码树的主要方式。C ABI、Java JNI、Android AAR、Swift 包和 WASM 演示都是实验性绑定，走的是同一套操作。

`Cargo.toml` 里的包版本是 0.1.0。`llp version` 报的也是这个版本。`git tag` 没有任何标签。向 crates.io API 查询 crate 名 `liblivephoto` 时返回 HTTP 404，所以 crates.io 上没有这个名字的可安装包。

## 安装库和命令行

`Cargo.toml` 没有设置 `rust-version`。在这台机器上，用 rustc 1.87.0 对 `liblivephoto`、`liblivephoto-cli` 和 `liblivephoto-native` 做锁定构建会失败，因为 `crates/liblivephoto-format/src/exif_raw.rs` 里的 `slice::as_chunks` 在那个版本仍是不稳定特性。同一套锁定构建在 rustc 1.88.0 和 rustc 1.99.1 上成功。这是构建记录，不是项目声明的最低 Rust 版本。

克隆仓库后，用一套能够编译这份锁定依赖的 Rust 工具链，运行下面的命令。

```sh
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

`llp version` 会打印 `"version": "0.1.0"` 和 `"success": true`。

本工作区里的 Rust crate 用路径依赖这个库。

```toml
liblivephoto = { path = "crates/liblivephoto", version = "0.1.0" }
```

## 其他绑定

这一节里的 Java 命令在这里实际跑过。Android、Swift 和 WASM 的命令没有在这里跑。它们的 CI 结果属于 `main` 上的提交 `042a188efc2f9c10638a571c3fec57b5032d5870`。

`python3 scripts/test_java.py` 会构建 `liblivephoto-native`，用 `javac --release 8` 编译 Java 源码，并对 `web/public/demo` 运行 `org.liblivephoto.NativeSmoke`。在 OpenJDK 21.0.12.1 和 rustc 1.99.1 下，脚本打印了 `"smoke": "passed"`。

`scripts/build_android.py` 会写出 `android/build/liblivephoto-android.aar`。`MotionPhotoService.MAX_INPUT_BYTES` 是 32 MiB，`MetadataParserService` 运行在隔离进程里。这台机器没有 Android SDK，所以脚本没有在这里执行。[Android native services](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330060) 的 `sdk-and-services` 作业在上述 `main` 提交上成功。该作业会构建 AAR，并运行一台自有的 API 35 x86_64 模拟器。模拟器结果不能当作厂商相册验收。

Swift 包在 `apple` 目录。`Package.swift` 声明 Swift tools 5.9、macOS 13、iOS 16，库名是 `LibLivePhotoApple`。这里没有安装 Swift。同一提交上的 [Apple adapter and local web](https://github.com/21Z121Z1/LibLivePhoto/actions/runs/41029330012) 作业 `apple` 成功。该作业会运行 `swift test --package-path apple`，并做 iOS 模拟器验收。模拟器结果不能当作「照片」应用的验收。

WASM 演示在 `web` 目录。同一工作流里的 `web` 作业成功。它使用 `wasm32-unknown-unknown` 目标、`wasm-bindgen-cli` 0.2.104 和 Node 24，然后运行 Node 测试和 Chrome 浏览器测试。这台机器的 Node 是 v22.23.4，那些 Web 命令没有在这里执行。

## 读取一张 Motion Photo

`web/public/demo` 里的文件是程序生成的几何图。`provenance.json` 标明它们使用 CC0，并且不含私人相机素材。下面的命令用的就是这些文件。

```sh
cargo run --locked -p liblivephoto --example inspect -- web/public/demo/motion-photo.jpg
```

在 rustc 1.99.1 下，示例会打印 `format=AndroidMotionPhoto`、`dialect=AndroidStandard`、4 个资源、1 条轨道，以及 `MediaTime { value: 812500, timescale: 1000000 }`。主视频资源是 19670 字节。

## 抽出视频字节

`llp` 用 `create_dir` 创建输出目录，用 `create_new` 创建每个输出文件。输出路径已经存在时会失败。再次运行前先删掉旧目录。

```sh
cargo run --locked -p liblivephoto-cli -- extract web/public/demo/motion-photo.jpg --video --out /tmp/llp-extract
cmp web/public/demo/motion.mp4 /tmp/llp-extract/resource-1.mp4 && echo match
```

抽取命令以退出码 0 结束。`resource-1.mp4` 为 19670 字节，与 `web/public/demo/motion.mp4` 逐字节相同。对这个样本来说，抽出的视频就是原来的视频字节。

## 生成一张 Motion Photo JPEG

```sh
cargo run --locked -p liblivephoto-cli -- build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out /tmp/llp-built
cargo run --locked -p liblivephoto-cli -- check /tmp/llp-built/asset-0.jpg
```

构建报告里有 `"canExecute": true`、`"target": "AndroidJpeg"` 和 `"consumerAcceptance": "Unverified"`。视频的处置是 `Preserved`。静图的处置是 `Rewritten`，因为写入器会替换容器元数据。随后 `llp check` 对 `ResourceRanges`、`StillContainer`、`MotionContainer` 和 `Tracks` 报告 `"valid": true`。

`13/16` 是这些演示文件在 `web/public/demo/provenance.json` 里记录的呈现时间。

## 支持范围

厂商名字不等于设备会接受这个文件。更完整的契约表在[格式支持](FORMAT_SUPPORT.md)。下表跟随那一页的说法。

| 契约 | 字节 | 结构 | 「照片」或厂商相册 |
| --- | --- | --- | --- |
| Android JPEG 目录 | 在真实样本上做往返比较 | 有测试，包括畸形文件 | 未验证 |
| Android HEIF `mpvd` | 只追加元数据，并比较图像项和时间 | 有测试，包括畸形文件 | 未验证 |
| Android AVIF `mpvd` | 合成契约和一份生成文件上的只追加元数据 | 参考实现重新解析，以及一次独立的静图解码 | 未验证 |
| Apple JPEG 或 HEIF，再加 MOV | 配对元数据、精确时间线，以及媒体比较。样本集里有一对原始 iPhone 15 文件 | 配对和支持的定时元数据 | 未验证 |
| 旧版 Google Micro Video | 合成测试上的 Android 转换和源副本 | 边界和目录检查 | 未验证 |
| ColorOS 15 与 16 扩展 | 源副本和显式侧车文件。原始样本包含两路流 | LPEX、拓扑和边界测试 | 未验证 |
| OpCamera 与 JXRS 布局 | 源副本，或在显式的未映射资源策略下写入标准目标。用例是合成的 | 帧结构和类型字段。校验和字节留在文件里，不视为已验证的校验和 | 未验证 |
| Samsung SEF 106 与 107 | 仅直接 JPEG SEF 写入器，且呈现时间必须未知 | 目录测试，包括畸形文件 | 未验证 |
| 小米 Android 风格 Live Photo | 一份原始样本，再写成 Android 目标或显式侧车文件 | 标准契约检查 | 未验证 |
| vivo 单文件与 CameraAlbum 43 | 两份原始样本。Android 目标或显式侧车文件。没有原生页脚写入器 | 页脚范围和声明 ID 检查 | 未验证 |
| CameraAlbum 55 | 没有原生写入器。只有合成契约 | 帧结构检查和畸形声明 | 未验证 |
| vivo 媒体 UUID 流 | 保留原始影片字节。没有原生 UUID 写入器 | 范围、大小和预算检查 | 未验证 |
| 小米人像或深度，以及 vivo 双路或人像 | 没有原始语料，也没有专用写入器 | 未知字节可以保留 | 未验证 |
| HarmonyOS Moving Photo | 本库没有私有的磁盘契约 | 不声称有磁盘契约 | 未验证 |
| 荣耀 `LIVE_` 页脚和反向 TLV | 合成契约上的源副本或显式侧车文件。没有荣耀写入器 | 有界记录和畸形用例 | 未验证 |

`crates/liblivephoto/src/conversion.rs` 里的 `TargetProfile` 包括 `PreserveSource`、`AndroidJpeg`、`AndroidHeif`、`AndroidAvif`、`SamsungJpegSef` 和 `ApplePair`。`liblivephoto-interface` 映射的 CLI 目标名是 `preserve`、`google-jpeg`、`google-heif`、`google-avif`、`apple` 和 `samsung-jpeg-sef`。

## 限度

这三件事是不同的事实。

字节被保留，指抽取结果或 `Preserved` 处置留下了原始资源字节。上面的抽取示例对演示视频就是这样。

结构检查通过，指 `llp check` 报告 `"valid": true`，或者构建计划没有阻碍项并且输出能再次解析。上面的构建示例就是这样。同一份构建报告把 `"consumerAcceptance"` 设为 `"Unverified"`。

[格式支持](FORMAT_SUPPORT.md) 的每一行都把原始消费者标成未验证。「照片」和厂商相册是否接受这些文件，目前没有验证。上面链接的 CI 作业跑的是模拟器和仿真器。

共享操作会拒绝合计超过 128 MiB 的输入。`crates/liblivephoto-interface/src/lib.rs` 里的 `MAX_INPUT` 是 `128 * 1024 * 1024`。Android 服务会拒绝合计超过 32 MiB 的输入。

重封装只接受顶层盒子 `ftyp`、`moov`、`mdat`、`free`、`skip` 和 `wide`。其他顶层盒子，包括 `sidx`，会从 `crates/liblivephoto/src/remux/compose.rs` 返回 `UnsupportedLayout`。`crates/liblivephoto/src/tracks.rs` 把分片元数据样本和分片帧时间标成不支持的布局。这次文档工作没有向 `llp` 提交过分片文件。

`TargetProfile` 没有 OPlus、小米、vivo 或荣耀写入器。Samsung JPEG SEF 写入器会拒绝已知的呈现时间。下面的命令以退出码 1 结束。

```sh
cargo run --locked -p liblivephoto --example convert -- samsung-jpeg-sef /tmp/llp-samsung web/public/demo/motion-photo.jpg
```

错误文本是 `SEF target has no supported exact presentation-time mapping; use Android or PreserveSource`。

本库没有构造 Moments 元数据的 API。写入器不会转码像素。

## 延伸阅读

命令细节见 [CLI](CLI.md)。转换限度见 [兼容性](COMPATIBILITY.md)。打包见 [原生 SDK](NATIVE_SDK.md)。绑定说明见 [Java](java/README.md)、[Android](android/README.md)、[Apple](apple/README.md) 和 [WASM 演示](web/README.md)。

厂商记录、交接说明和未决问题索引在[研究笔记](docs/research.zh-CN.md)。
