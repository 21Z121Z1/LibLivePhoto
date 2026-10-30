# LibLivePhoto

[English](README.md)

**LibLivePhoto** 是一个 Rust 库，也提供 `llp` 命令行工具，用于处理 Apple 实况照片、Android 动态照片和 Samsung SEF 文件。

这些格式都把一张照片和一段短视频组合在一起，保存方式却各不相同。LibLivePhoto 可以查看文件内容、提取原始媒体资源，也能生成已支持的目标格式。C/JNI、Android AAR、Swift 和 WASM 绑定仍处于实验阶段。

Rust 1.88+ (tested; not yet declared)

## 快速开始

克隆仓库并构建命令行工具：

```sh
git clone https://github.com/21Z121Z1/LibLivePhoto.git
cd LibLivePhoto
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

如果你的 Rust 项目与 `LibLivePhoto` 目录并列，可以在项目的 `Cargo.toml` 中加入：

```toml
[dependencies]
liblivephoto = { path = "../LibLivePhoto/crates/liblivephoto" }
```

## 示例

以下命令均从仓库根目录运行。[演示素材](web/public/demo/provenance.json)由脚本生成，采用 CC0 授权。

### 查看文件内容

```sh
./target/debug/llp parse web/public/demo/motion-photo.jpg
```

报告会列出文件格式、资源和照片呈现时间。

### 提取视频

```sh
./target/debug/llp extract web/public/demo/motion-photo.jpg --video --out llp-demo-extracted
```

视频保存为 `llp-demo-extracted/resource-1.mp4`。输出目录必须尚不存在，工具会自行创建。

### 生成动态照片并检查结构

```sh
./target/debug/llp build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out llp-demo-built
./target/debug/llp check llp-demo-built/asset-0.jpg
```

`13/16` 是演示素材的照片呈现时间。检查命令读取刚生成的 `asset-0.jpg`。

## 格式支持

下表仅列出已经实现、并有样本测试覆盖的操作。各格式的具体条件见[格式支持说明](FORMAT_SUPPORT.md)。

| 格式 | 读取 | 提取 | 生成 / 转换 |
| --- | --- | --- | --- |
| Android 动态照片 JPEG | 支持 | 支持 | `google-jpeg` |
| Android HEIF（`mpvd`） | 支持 | 支持 | `google-heif` |
| Android AVIF（`mpvd`） | 有限样本 | 有限样本 | `google-avif` |
| Apple 实况照片（JPEG/HEIF + MOV） | 支持 | 支持 | `apple` 配对文件 |
| Samsung SEF JPEG（106/107） | 支持 | 支持¹ | `samsung-jpeg-sef`² |
| 旧版 Google MicroVideo | 合成样本 | 合成样本 | 转为 Android 格式或保留原文件 |
| 部分 OPlus、小米、vivo、荣耀扩展 | 有限支持 | 有限支持 | 标准格式或侧车文件；无原生写入器 |

¹ Samsung 记录有歧义时，必须明确选择。² Samsung 写入要求照片时间未知。

**所有格式均未验证能否被 Apple「照片」或厂商相册实际接收。**

## 目前的限制

提取操作可以保留原始媒体字节。`llp check` 只检查已支持的文件结构。这两项结果都不能证明系统相册能识别生成的文件。

命令行和共享接口的合计输入上限为 **128 MiB**，Android 服务为 **32 MiB**。目前不支持流式解析，也无法重封装分片影片或含 `sidx` 等顶层盒子的文件。

写入器支持保留原文件、生成 Android JPEG/HEIF/AVIF、Apple 照片与 MOV 配对文件，以及 Samsung JPEG SEF。没有 OPlus、小米、vivo、荣耀的原生写入器，也不会重新编码媒体内容。

更多说明：[CLI](CLI.md) · [兼容性](COMPATIBILITY.md) · [原生 SDK](NATIVE_SDK.md) · [Java/JNI](java/README.md) · [Android](android/README.md) · [Swift](apple/README.md) · [WASM](web/README.md)。

构建与测试记录见[验证记录](docs/VERIFICATION.zh-CN.md)。项目采用 [Apache-2.0 许可证](LICENSE)。
