# LibLivePhoto

[English](README.md)

一个 Rust 库，加上命令 `llp`，用来读取、检查和生成 Motion Photo 与 Live Photo。

各家手机会把静图和短视频嵌进自己的容器。这个库负责把文件拆开，格式允许时交还原来的字节，并写到少数几种说得清的目标上。C ABI、Java JNI、Android AAR、Swift 包和 WASM 演示都是实验性外壳，底下仍是同一套操作。

需要 Rust 1.88 或更高版本（实测过，仓库里还没声明最低版本）。

## 快速开始

克隆仓库后，在根目录执行：

```sh
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

工作区里的其他 crate 用路径依赖这个库：

```toml
liblivephoto = { path = "crates/liblivephoto", version = "0.1.0" }
```

## 示例

`web/public/demo` 里是脚本画出来的几何图。`provenance.json` 标明 CC0，也不含私人拍摄的素材。

`llp` 会自己创建输出目录和里面的每个文件。目录已经在的话命令会失败，再跑一次之前先把旧目录删掉。

### 看一张 Motion Photo

```sh
cargo run --locked -p liblivephoto --example inspect -- web/public/demo/motion-photo.jpg
```

### 把视频抽出来

```sh
cargo run --locked -p liblivephoto-cli -- extract web/public/demo/motion-photo.jpg --video --out /tmp/llp-extract
```

### 做成一张 Motion Photo JPEG

这些演示文件记下的照片时间是 `13/16`。

```sh
cargo run --locked -p liblivephoto-cli -- build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out /tmp/llp-built
cargo run --locked -p liblivephoto-cli -- check /tmp/llp-built/asset-0.jpg
```

命令行上的目标名是 `preserve`、`google-jpeg`、`google-heif`、`google-avif`、`apple` 和 `samsung-jpeg-sef`。参数说明在 [CLI](CLI.md)。

## 支持哪些格式

相册肯不肯收，和文件上写着哪家厂商是两回事。完整契约见[格式支持](FORMAT_SUPPORT.md)。

| 契约 | 这棵树里有什么 | 「照片」或厂商相册 |
| --- | --- | --- |
| Android JPEG 目录 | 真实样本上能往返，测试覆盖畸形文件 | 未验证 |
| Android HEIF `mpvd` | 只追加元数据，并核对图像项和时间 | 未验证 |
| Android AVIF `mpvd` | 合成契约和一份生成文件上只追加元数据 | 未验证 |
| Apple JPEG 或 HEIF，再加 MOV | 核对配对、时间线和媒体，样本里有一对 iPhone 15 原片 | 未验证 |
| 旧版 Google Micro Video | 合成测试上可转成 Android，或保留源副本 | 未验证 |
| ColorOS 15 和 16 | 源副本和显式侧车；原片里有双路流 | 未验证 |
| OpCamera 和 JXRS | 源副本，或在说清未映射资源的策略后写入标准目标 | 未验证 |
| Samsung SEF 106 和 107 | 只有直接的 JPEG SEF 写入，而且照片时间必须未知 | 未验证 |
| 小米 Android 风格 Live Photo | 一份原片，写成 Android 目标或侧车 | 未验证 |
| vivo 单文件和 CameraAlbum 43 | 两份原片；Android 目标或侧车；没有原生页脚写入 | 未验证 |
| CameraAlbum 55 | 只有合成契约，没有原生写入 | 未验证 |
| vivo 媒体 UUID 流 | 影片字节原样留下，没有原生 UUID 写入 | 未验证 |
| 小米人像或深度，vivo 双路或人像 | 没有原片语料，也没有专用写入 | 未验证 |
| HarmonyOS Moving Photo | 本库不包含私有的磁盘契约 | 未验证 |
| 荣耀 `LIVE_` 页脚和反向 TLV | 合成契约上可留源副本或侧车，没有荣耀写入器 | 未验证 |

## 限制

字节在不在、结构过不过、系统相册收不收，要分开看。抽取可以交还原来的资源字节。`llp check` 和没有阻碍项的构建计划只说明结构。上表里，系统相册一列全部是未验证。

共享操作拒绝合计超过 128 MiB 的输入。Android 服务拒绝合计超过 32 MiB 的输入。

重封装只接受顶层盒子 `ftyp`、`moov`、`mdat`、`free`、`skip` 和 `wide`。别的顶层盒子，包括 `sidx`，会被拒绝。分片元数据样本和分片帧时间都不支持。

`TargetProfile` 能写 `PreserveSource`、Android JPEG / HEIF / AVIF、Samsung JPEG SEF 和 Apple 配对。没有 OPlus、小米、vivo 或荣耀的写入器。照片时间已知时，Samsung JPEG SEF 写入会拒绝。没有构造 Moments 的 API，写入器也不转码像素。

绑定说明分别在 [Java](java/README.md)、[Android](android/README.md)、[Apple](apple/README.md) 和 [WASM 演示](web/README.md)。命令输出和 CI 记录在[验证记录](docs/VERIFICATION.zh-CN.md)。

## 另见

转换上的限制见[兼容性](COMPATIBILITY.md)。打包见[原生 SDK](NATIVE_SDK.md)。厂商记录和还没定的问题索引在[研究笔记](docs/research.zh-CN.md)。

## 许可证

Apache-2.0。见 [LICENSE](LICENSE)。
