# liblivephoto

[English](README.md)

用于检查、提取和转换 Apple 实况照片、Android 动态照片及已支持 Samsung SEF 布局的 Rust 库。需要 Rust 1.88 或更高版本。库保留编码媒体，不对图像或视频转码。

```no_run
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

Apple 配对使用 `Input::ApplePair { still, movie }`。精确照片时间使用整数值与时间尺度组成的 `MediaTime`。转换先调用 `plan` 再调用 `execute`；不支持的布局、无法精确表示的目标时间和无法映射的资源会明确报错。执行前应检查计划。`pvt::package_pair` 把已验证的 JPEG 或 HEIC 配对组装成内存中的 `.pvt` 目录，不转码，也不表示 Photos 可以导入。

Rust 默认输入预算为 256 MiB，独立的 CLI/共享接口使用 128 MiB。解析不是流式的。原生播放、编辑和相册接收取决于宿主、格式与设备。

0.1.x Rust API 在补丁版本间保持源码兼容，不兼容改动使用新的次版本号。可选 `xdremux-compat` 功能和各平台绑定继续处于实验阶段。

包内提供合成数据，可运行 `cargo test --all-targets --all-features`。被忽略的原始样本测试需要单独获取文件，不分发私人照片。格式限制、转换策略和平台 SDK 见[项目文档](https://github.com/21Z121Z1/LibLivePhoto)。采用 Apache-2.0 许可证。
