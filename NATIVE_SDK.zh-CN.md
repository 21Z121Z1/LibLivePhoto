# 原生 SDK 集成

[English](NATIVE_SDK.md)

CLI、C ABI、Java/JNI、Swift 与 Android 服务共用操作实现。JSON 请求使用 `liblivephoto-interface`，LLP1 响应持有精确 JSON 和原始输出字节。适配层不静默输出文件、上传、转码或创建 Moments 元数据；整数时间是权威值。

可选 `conformance` 请求检查单个不可变 HEIF 图像或影片。使用 `profile:"heif-color-2025"`，或 `profile:"google-video-container-1.0"` 加有序 `trackIds:[primary,secondary]`，其中次轨道可省略。CLI `--tracks` 和 Java `new Request("conformance").option("profile",...).tracks(...)` 使用同一结构。报告列出检查规则及 Violation/Unsupported/Recommendation，成功只覆盖列出的必要规则。详见[范围与示例](CORE_INTEGRATION.zh-CN.md)。主机 C/JNI 调用已有测试，Android 服务白名单不开放这一额外操作。完整码流、渲染和原生相册接收均独立于这两种配置检查。

## 主机归档

```sh
python3 scripts/package_native.py
python3 scripts/test_packaged_sdk.py dist/liblivephoto-<os>-<arch>-sdk.zip
```

需要 Python 3.11+、完整 JDK 21 和 Rust。归档包含 release CLI、动态/静态库、C 头文件、Java 8 jar、文档、源身份、文件哈希和每项锁定依赖的声明。CI 在 Linux x86_64、macOS arm64 和 Windows x86_64 上测试解包后的 SDK。外部 Java 消费程序只使用打包 jar 编译，并加载解包后的 release 库。脚本只构建产物；[发布工作流](RELEASING.zh-CN.md)另行整理并发布版本化产物。

C 调用方使用 SDK 中的 `include/liblivephoto.h`，或源码检出中的 `native/include/liblivephoto.h`。指南中相对仓库的构建命令和源码路径针对带标签的源码检出；SDK 二进制位于 `bin/` 和 `lib/`。响应句柄由 Rust 持有：释放前复制数据，并只调用一次配套释放函数。调用方负责指针有效性，边界层检查空指针、长度和预算。核心、格式和共享接口继续禁止 unsafe 代码；指针操作局限于原生边界。

Java 调用方使用 jar，并设置 `-Dliblivephoto.native.path=/absolute/library`，详见 [Java](java/README.zh-CN.md)。输入字节、路径和借用 `FileChannel` 区间共用操作结构。定位读取保留句柄所有权与游标。取消会停止排队或读取阶段的工作，并丢弃迟到结果；同步 Rust 阶段内部不可中断。

## Apple 静态核心

在 Xcode 环境安装 Rust 目标 `aarch64-apple-ios`、`aarch64-apple-ios-sim` 和 `x86_64-apple-ios` 后：

```sh
python3 scripts/package_apple.py
python3 scripts/test_apple_simulator.py
```

XCFramework 包含 iOS arm64 与 arm64/x86_64 模拟器静态库，以及头文件和依赖声明。将相应归档链接到宿主，保留并导出 C 入口，再使用 `RustCoreBridge(linkedCore: true)`。提供的模拟器验收使用 `-force_load` 和 `-export_dynamic`；启用 dead stripping 的宿主也必须保留并导出 bridge 解析的 `llp_*` 符号。支持的原生宿主还可使用本地 dylib 初始化器。宿主签署最终应用，适配层不下载库。

模拟器验收会针对 iOS SDK 编译真实 Swift、链接 Rust、创建自有兼容模拟器，并执行 parse/extract/AGTM/错误调用。只关闭和删除本次创建的模拟器。SDK 构建、模拟器运行和真实设备/照片图库接收具有独立收据。详见 [Apple](apple/README.zh-CN.md)。

## Android 与元数据

AAR、四个服务、隔离元数据进程、FD/区间、管道输出、预算和已安装 APK 测试见 [Android](android/README.zh-CN.md)。脚本不自动接受 SDK 许可；生成的测试密钥继续被忽略，不上传。

`metadata` 返回区间、有界原始视图、声明及独立的源验证。`extract-metadata` 在合计 128 MiB 预算内复制完整选定块，使用 `blockIds` 或 CLI `--blocks`。块可能含编码 HEIF 项目或专有视频尾部，响应会明确标注。这扩展了 Google 只返回首个匹配动态块的便捷接口，不推测像素、深度或 HDR 语义。`classify-metadata` 总会清除已保存的源验证。缺少可选 XMP 时列入 `unavailable`；无效检查返回 `success=false`。

使用结果和诊断判断情况，不把签名或生成文件当作相册/照片接收证据。未知的带引用 MP4 布局继续被拒绝。有证据支持的重封装保留样本包和精确时间，并报告主动压缩或丢弃。

第 4 轮验证的源码 **bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7** 通过了三组工作流、八个作业，包括在所有适用原生、移动和真实浏览器入口执行的 42 个新增语法、字节、时间与错误向量。详见[交接记录](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_ROUND4_20261019.md)、[收据](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/platform-bd6f85b.json)及[边界](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/REMAINING_WORK.md)。该历史记录对原生相册/照片、完整 ICC/AAC/渲染/设备兼容性仍标为未验证或部分验证。

共享操作协议提供最小 WASM 导出：浏览器构建输出到 `web/dist/operation-pkg`，Node 构建输出到 `operation-node`。`process_operation(requestJSON, primaryBytes, movieBytes)` 返回自持有 LLP1 字节，调用相同 Rust 实现，不输出文件。已有检查/UI 模块保持独立。42 个用例检查原始 ICC 配置、AAC 声明、紧凑样本表、字节保留和精确时间。Android 服务白名单不变：conformance/remux 测试使用真实的应用内 JNI，已有 parse/build/extract/AGTM 测试使用 Binder/FD 区间。构建出 arm64 Android/iOS 库不等于真实设备运行验收。

当前维护者确认的实机状态见[兼容性记录](COMPATIBILITY.zh-CN.md#实机验证)。
