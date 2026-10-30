# LibLivePhoto

[English](README.md)

用一个 Rust 库和 `llp` 命令行工具，读取、拆解、生成和检查 Apple 实况照片、Android 动态照片及 Samsung SEF 文件。

## 为什么做这个项目

实况照片本质上是一张静态照片和一段短视频，但不同厂商的封装和元数据并不相通。LibLivePhoto 可以查看文件里的资源、原样提取媒体数据，也能转换成已实现的目标格式，不会暗中重新编码画面。C/JNI、Android AAR、Swift 和 WASM 绑定目前仍是实验性功能。

Rust 1.88+ (tested; not yet declared)

## 快速开始

在仓库根目录执行：

```sh
cargo build --locked -p liblivephoto-cli
./target/debug/llp version
```

如果你的 Rust 项目和克隆下来的 `LibLivePhoto` 目录并列，在项目的 `Cargo.toml` 中加入：

```toml
[dependencies]
liblivephoto = { path = "../LibLivePhoto/crates/liblivephoto" }
```

## 示例

下面使用仓库自带的 [演示文件](web/public/demo/provenance.json)。素材由脚本生成，以 CC0 授权；命令均从仓库根目录运行。`--out` 指定的目录必须尚不存在，工具会自行创建。

### 查看动态照片的内容

```sh
./target/debug/llp parse web/public/demo/motion-photo.jpg
```

### 提取其中的视频

```sh
./target/debug/llp extract web/public/demo/motion-photo.jpg --video --out llp-demo-extracted
```

视频会保存为 `llp-demo-extracted/resource-1.mp4`。

### 生成动态照片并检查结构

```sh
./target/debug/llp build web/public/demo/cover.jpg --movie web/public/demo/motion.mp4 --time 13/16 --target google-jpeg --out llp-demo-built
./target/debug/llp check llp-demo-built/asset-0.jpg
```

`13/16` 是演示素材的照片呈现时间。其他命令和参数见 [CLI 文档](CLI.md)。

## 格式支持

下表只概括代码已实现、测试样本已覆盖的操作，不代表所有机型和文件变体都兼容。各格式的具体条件见[格式支持说明](FORMAT_SUPPORT.md)。

| 格式 | 读取 / 查看 | 提取 | 生成 / 转换 | 系统「照片」/ 厂商相册验收 |
| --- | --- | --- | --- | --- |
| Android 动态照片（JPEG） | 支持 | 支持 | `google-jpeg` | **未验证** |
| Android 动态照片（HEIF `mpvd`） | 支持 | 支持 | `google-heif` | **未验证** |
| Android 动态照片（AVIF `mpvd`） | 有限样本 | 有限样本 | `google-avif` | **未验证** |
| Apple 实况照片（JPEG/HEIF + MOV） | 支持 | 支持 | `apple` 配对文件 | **未验证** |
| Samsung SEF（JPEG，106/107） | 支持 | 支持；歧义记录需指定 | `samsung-jpeg-sef`，仅限照片时间未知 | **未验证** |
| 旧版 Google MicroVideo | 合成样本 | 合成样本 | 转为 Android 格式或保留原文件 | **未验证** |
| 部分 OPlus、小米、vivo、荣耀扩展 | 有限支持 | 有限支持 | 可转标准格式或输出侧车；无原生写入器 | **未验证** |

## 目前的限制

**原始字节得到保留、文件结构检查通过、系统相册能够导入，是三件不同的事。** 提取操作可以保留原始媒体字节；`llp check` 只检查已支持的结构规则，不等于 Apple「照片」或厂商相册已经验证可用。

命令行和共享接口的合计输入上限为 **128 MiB**，Android 服务为 **32 MiB**。目前不支持流式随机访问解析；分片影片及含 `sidx` 等不受支持顶层盒子的文件无法重封装。

写入目标包括原文件保留、Android JPEG/HEIF/AVIF、Apple 照片与 MOV 配对，以及照片时间未知时的 Samsung JPEG SEF。没有 OPlus、小米、vivo、荣耀的原生写入器，也不会转码图像像素。具体转换边界见[兼容性说明](COMPATIBILITY.md)。

实验性绑定说明：[Java/JNI](java/README.md) · [Android](android/README.md) · [Swift](apple/README.md) · [WASM](web/README.md)。构建与测试依据见[验证记录](docs/VERIFICATION.zh-CN.md)。项目采用 [Apache-2.0 许可证](LICENSE)。
