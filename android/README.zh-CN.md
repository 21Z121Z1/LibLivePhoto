# Android 绑定服务（API 26+）

[English](README.md)

AAR 包含 Java API、Rust JNI 库（`arm64-v8a` 与 `x86_64`）、AIDL 和四个不导出的服务。`MetadataParserService` 运行在隔离进程中，构建器、检查器和提取器分别运行在应用进程中。不申请媒体或 Internet 权限，不上传或输出文件，不包含 Moments 构造。

使用**已经安装且许可已接受**的 Android SDK/NDK、完整 JDK 21，以及 Rust 目标 `aarch64-linux-android,x86_64-linux-android` 构建：

```sh
python3 scripts/build_android.py
python3 scripts/test_android.py # only on your authorized test device/emulator
```

脚本不接受 SDK 许可，会报告缺失路径。生成的 `android/build/liblivephoto-android.aar` 可作为宿主应用的本地 AAR，包含服务 manifest 和 JNI 混淆保留规则。单独签名的验收 APK 只使用项目合成媒体；生成测试密钥被 Git 忽略，不上传。构建脚本不发布版本。

```java
MotionPhotoClient client = new MotionPhotoClient(context, MetadataParserService.class);
client.ready().thenAccept(connected -> {
    // Keep this descriptor open until execute() returns; the host owns it.
    try (ParcelFileDescriptor fd = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)) {
        connected.execute(new Request("metadata"), MotionPhotoClient.Source.whole(fd), null,
            (phase, done, total) -> { /* dispatch UI updates yourself */ })
            .result().thenAccept(result -> { /* owned metadata and original file bytes */ });
    } catch (Exception error) { /* explicit connection/request/I/O error */ }
});
// Close when the host no longer needs this binding. Active operations cancel.
```

有界描述符使用 `MotionPhotoClient.Source.range(fd, offset, length)`。定位读取保留调用方游标，不关闭其描述符。宿主负责打开路径；内存输入可直接使用 `NativeEngine`，或转换为宿主持有的临时 FD。服务合计输入上限为 **32 MiB**，请求 JSON 为 64 KiB，每个服务允许一个运行作业和两个排队作业。这些是明确的适配层限制，区别于核心/JVM 限制。管道及不可 seek 输入在区间验证时被拒绝。响应通过可靠输出管道传送 LLP1，客户端响应预算为 160 MiB；Binder 只携带小型请求、进度和状态。宿主必须完整解码结果后，才可明确写出新文件。服务拒绝普通输出文件，避免输出描述符与输入别名时覆盖原文件。

构建使用 `new Request("build").inputKind("parts").target("google-jpeg")`，提供两个源和 `ExactTime`；检查使用 `new Request("check")`，提取使用 `new Request("extract").option("video",true)...`。元数据解析器也支持 `parse`、`metadata`、`classify-metadata`、`xmp`、`agtm` 和提供者过滤，请求结构与 Java API/CLI 相同。服务拒绝超出自身角色的动作。`NativeResult.success` 与结构化元数据表示核心失败；连接、IPC 或截断响应会使 future 异常完成。取消停止读取/排队工作，丢弃迟到结果；同步 Rust 阶段内部不可中断。回调进程死亡会取消工作。

参见[官方 AIDL 线程和所有权说明](https://developer.android.com/develop/background-work/services/aidl)及 [ParcelFileDescriptor](https://developer.android.com/reference/android/os/ParcelFileDescriptor)。构建收据与已安装 APK 的运行证据分开记录。模拟器服务成功不验证厂商相册、相机拍摄或真实设备 HDR 渲染，不使用设备访问或权限绕过。

运行验收执行六个独立 instrumentation：`core`、`metadata`、`lifecycle`、`concurrency`、`syntax`、`vendor`，六者均必须通过。各自保留 240 秒整例限时、40 秒操作限时和 20 秒 ready-future 等待，服务连接按顺序建立，避免软件模拟器上的八次完整解析/生命周期迭代共用一个会过期的总限时。生命周期用例仍执行全部八次解析；并发用例另外检查三个排队管道读取器，以及两个调用方竞争取消同一操作。保留每例日志、失败阶段/堆栈与 logcat。SDK/AVD 配置成功或只执行部分流程，都不能生成运行成功收据。

启动后最多等待 120 秒/40 次，要求 package/activity 服务可用，且 `pm list packages` 成功并返回干净结果。退出 224、超时、格式错误或错误输出都不能表示“包不存在”。不重启 ADB，也不修改系统权限。每次运行写入独立 `android/build/acceptance-*/`，保存命令标准输出/错误/退出状态、有界 logcat、范围日志和收据，也包括安装前失败。成功门槛为全部 156 项断言通过，并移除已确认属于本次运行的测试 APK。`python3 scripts/test_android_harness.py` 在无设备环境注入工具失败，其通过不属于 Android 运行验收。

失败 ADB 命令及失败后有边界的服务/logcat 快照也会出现在 CI 标准输出中，避免产物传输失败遮蔽安装错误。主机故障注入抑制自身合成的服务成功标记；只有实际安装设备阶段与运行收据能证明服务验收。instrumentation 阶段也通过状态 bundle 传递，因此进程在返回失败 bundle 前死亡时，仍能看到最后完成阶段。

工具在清理前保留独立且有界的 crash-buffer 快照，以及本包的 `dumpsys activity exit-info`。不可用的诊断命令会明确保留，不推定通过；不修改日志缓冲区大小或设备权限。[Android logcat](https://developer.android.com/tools/logcat) 描述独立 crash buffer；[API35 ActivityManager 实现](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android15-release/services/core/java/com/android/server/am/ActivityManagerService.java)支持按包查询退出历史。退出描述属于诊断文本，不是稳定机器协议或已推断的根因。最终运行收据也打印到 CI 标准输出，包括完成范围、精确断言总数及自有包清理确认。

[有界绑定调查](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/ANDROID_BIND_ROUND2.md)加入可选客户端事件轨迹、可调试宿主生命周期/原生工作轨迹，以及每范围绑定服务、系统和退出快照，分别定位绑定接受、回调缺失和握手延迟，不改变现有就绪/操作限时，也不证明根因已修复。旧收据字段 `bind_deadline_seconds:20` 指同步 `bindService` 返回后的 `ready().get(20)`；240 秒范围计时包含同步调用。连接成功状态还保留既有 PID 断言取得的服务 PID 和实测 future 等待时长。
