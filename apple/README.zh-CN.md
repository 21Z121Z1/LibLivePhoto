# Apple 轻量适配层

[English](README.md)

Swift Package 支持 macOS 13+ / iOS 16+，使用公开 PhotoKit、PhotosUI 和 AVFoundation。在 macOS 上运行 `swift test --package-path apple`；iOS 可在 Xcode 编译，或执行 `.github/workflows/apple-web.yml` 的模拟器 SDK 构建。Linux 无法验证 Apple SDK 分支，开发容器没有 Swift/Xcode。

宿主通过 Rust 核心验证文件，保留原始清单字节和文件，并可用 `InspectionManifest.decode` 解码 v1 清单。样本来自 `liblivephoto-bridge` 对内置合成 Apple 配对的真实输出，不是手写元数据。包中也包含下述真实原生 RustCoreBridge；宿主集成保留带版本的清单及本地资源 URL。Web 仍直接使用 Rust 的 WASM 绑定。

`PhotoKitAdapter.load` 加载本地配对资源，`resources(for:)` 保留完整原生资源清单，`export` 导出明确选定的资源；`importPair` 以关闭移动的方式提交 `.photo` 与 `.pairedVideo`。调用方获取适当的照片图库权限；导入包本身不会自动调用 API。编译或无效输入测试不会访问用户图库，也不能证明有效配对导入成功。

NativePlayback 封装 iOS/macOS 系统视图、状态和静音。公开系统播放手势访问器在 iOS 上存在，macOS 会报告该访问器不可用；原生对象测试不证明真实设备播放/HDR 接收。NativeEditingSession 转发系统帧处理器和音量，并管理预览/输出取消。适配层对每个编辑上下文的操作串行化；操作期间应保留会话和封装对象。NativeOperation 回调可能在原生队列上运行，UI 更新需自行切换到 UI actor。提交前的取消是终态；不可取消的提交返回 false。导出有字节预算，拒绝覆盖已有目标；I/O 失败可能留下新输出的一部分。导入复制原文件，不移动它们。

CaptureContract 只验证宿主会话和设置，不是相机系统。可移植 API 明确不提供拍摄和照片私有效果。能力限制与证据见[公开 API 映射](../APPLE_API_ARCHITECTURE.zh-CN.md)。

`RustCoreBridge` 调用调用方提供的本地原生库中的真实独立 C ABI。它在释放 Rust 句柄前复制 LLP1 响应，保留权威 JSON 字节，并解码精确检查约定。异步封装抑制已取消或过期完成结果，不宣称中断了同步原生工作。构建 `cargo build -p liblivephoto-native`，并把 `LIBLIVEPHOTO_NATIVE_LIBRARY` 指向 dylib，以运行真实 macOS bridge 测试。iOS 宿主必须在应用中嵌入正确构建/签名的原生库；只编译模拟器 SDK 不验证设备链接或运行策略。不从远程 URL 下载或加载库代码。

原生测试会尝试加载项目生成的有效本地配对，并逐字节比较导出资源，独立于照片图库导入权限。是否通过以具体 CI/测试收据为准。导入、编辑、真实播放/HDR 接收仍需有适当权限的资产与 Apple 设备；测试不绕过隐私权限。

`PhotoKitAdapter.editingInput` 请求已获授权的资产，明确指定网络、进度、取消策略和调整数据处理。使用真实宿主调整数据创建输出，通过 `NativeEditingSession.save` 渲染，再对同一资产调用 `PhotoKitAdapter.commitEdit`。导入与提交要求事先取得宿主权限，不自动申请。音量必须是 [0,1] 内的有限值；授权图库与渲染实际接收另行验收。

静态 XCFramework 使用 `RustCoreBridge(linkedCore: true)`，并保留、导出 C 符号。打包、链接参数及真实 iOS 模拟器 Swift/Rust 验收命令见[原生 SDK 集成](../NATIVE_SDK.zh-CN.md)。

运行验收工具只创建和删除自有模拟器 UUID，记录启动进度、所有权状态、限定范围的 CoreSimulator 日志及失败收据。在两次 iPhone17e/iOS26.5 启动监测超时后，优先从已装 SDK 选择兼容 SE 设备，也可用 `--device-type IDENTIFIER` 指定兼容设备。若 `bootstatus` 超时但自有设备已处于 `Booted`，工具可在 120 秒限时内尝试原样执行真实 Swift/Rust 进程。全部原始原生断言和运行身份输出仍必须满足，断言失败不会通过重试改记成功。此处证明 C ABI 执行，不证明 SpringBoard、照片图库或真实设备 UI 就绪；清理失败也会导致验收失败。`python3 scripts/test_apple_harness.py` 的七项测试只做故障注入，不是模拟器验收。

工具选择已安装且不低于 iOS16 部署目标的 iOS 运行时，优先最低可用版本；`--runtime IDENTIFIER` 可选择其他可用兼容版本。它验证查询到的 iPhoneSimulator SDK，在子进程中隔离继承的 SDKROOT，并记录所有可用运行时及所选 SDK。XCFramework 按目标选择正确 SDK，明确设置 iOS16 部署目标。不修改保存的环境设置，不安装运行时，不禁用迁移，也不授予权限。
