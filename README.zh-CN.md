# LibLivePhoto

[English](README.md)

**LibLivePhoto** 是一个 Rust 库，也提供 `llp` 命令行工具，用于处理 Apple 实况照片、Android 动态照片和 Samsung SEF 文件。

这些格式都把一张照片和一段短视频组合在一起，保存方式却各不相同。LibLivePhoto 可以查看文件内容、提取原始媒体资源，也能生成已支持的目标格式。C/JNI、Android AAR、Swift 和 WASM 绑定仍处于实验阶段。

Rust 1.88+。0.1 版本正式支持 Rust 核心和 `llp`；`xdremux-compat`
功能和各平台绑定保留实验标记。

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

```rust,no_run
use liblivephoto::{Input, MotionPhoto, ParseOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("input.jpg")?;
    let photo = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::compatible())?;
    photo.validate()?;
    let mut video = std::fs::File::create_new("video.mp4")?;
    photo.extract(photo.motion_video(), &mut video)?;
    Ok(())
}
```

版本化源码和 SDK 可从 [GitHub Releases](https://github.com/21Z121Z1/LibLivePhoto/releases) 获取。注册表安装命令与实际可用状态单独记录，见[发布说明](RELEASING.zh-CN.md)。上面的源码构建不依赖 LibLivePhoto 已上传注册表。

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
./target/debug/llp check llp-demo-built/assetMP.jpg
```

`13/16` 是演示素材的照片呈现时间。检查命令读取刚生成的 `assetMP.jpg`。Google JPEG、HEIC 和 AVIF 构建使用这个 `MP` 后缀。这不证明 Google 相册可以播放。

## 格式支持

下表仅列出已经实现、并有样本测试覆盖的操作。各格式的具体条件见[格式支持说明](FORMAT_SUPPORT.zh-CN.md)。

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

维护者已完成实机验证。此表描述已实现的布局和自动化覆盖；具体设备、系统和操作组合
尚未逐项录入公开的[兼容性记录](COMPATIBILITY.zh-CN.md#实机验证)。

## 目前的限制

提取操作可以保留原始媒体字节。`llp check` 只检查已支持的文件结构。这两项结果都不能证明系统相册能识别生成的文件。

命令行和共享接口的合计输入上限为 **128 MiB**，Android 服务为 **32 MiB**。目前不支持流式解析，也无法重封装分片影片或含 `sidx` 等顶层盒子的文件。

写入器支持保留原文件、生成 Android JPEG/HEIF/AVIF、Apple 照片与 MOV 配对文件，以及 Samsung JPEG SEF。没有 OPlus、小米、vivo、荣耀的原生写入器，也不会重新编码媒体内容。

更多说明：[CLI](CLI.zh-CN.md) · [兼容性](COMPATIBILITY.zh-CN.md) · [原生 SDK](NATIVE_SDK.zh-CN.md) · [Java/JNI](java/README.zh-CN.md) · [Android](android/README.zh-CN.md) · [Swift](apple/README.zh-CN.md) · [WASM](web/README.zh-CN.md)。

构建与测试记录见[验证记录](docs/VERIFICATION.zh-CN.md)。0.1.x 兼容性约定和发布检查
见[更新记录](CHANGELOG.zh-CN.md)与[发布说明](RELEASING.zh-CN.md)。项目采用 [Apache-2.0 许可证](LICENSE)。
