# Java/JNI 接口

[English](README.md)

使用 `cargo build -p liblivephoto-native` 构建独立原生边界，再用完整 JDK 21 运行 `python3 scripts/test_java.py`，按 `--release 8` 编译。兼容 Java 8 的源码只使用公开 API；Android 的文件、路径和通道接口目前要求 API 26+。Android 实际构建和执行独立于主机 JVM 结果。

```java
NativeEngine engine = new NativeEngine();
NativeResult parsed = engine.parse(Input.path(path));
NativeResult built = engine.build(Input.bytes(still), Input.bytes(movie),
    new ExactTime(13, 16), "google-jpeg");
```

设置 `-Dliblivephoto.native.path=/absolute/path/to/the/library`，或为 `livephoto_native` 配置正常的平台库搜索路径。方法提供真实的 parse/check/extract/build/metadata/AGTM 能力；`extractMetadata` 使用合计预算复制完整源块，并标记原始/编码内容。`Request` 与 `execute` 提供提供者过滤、明确独立组成部分、Apple 配对 ID、侧车/丢弃策略、选定资源、轨道导入/删除和优化。请求 JSON 遵循共享 `liblivephoto-interface` 协议，不承诺与 Google Java/protobuf ABI 兼容，没有 Moments 构造方法。

`Input.range(FileChannel, offset, length)` 是文件描述符/区间约定：使用属于公开 FileDescriptor/FileInputStream 或 Android ParcelFileDescriptor 适配器的通道。读取具有定位和边界，不关闭调用方通道。路径输入自行打开和关闭通道。在操作期间保留原描述符，并同步外部修改；不反射私有 FileDescriptor 字段，也不猜测原始描述符。

输出是自持有字节缓冲区，带名称、MIME、资源身份和精确 JSON 报告。`data()` 只读，`copyBytes()` 返回新数组。`writeNew` 拒绝覆盖已有路径。成功包含有效的检查结果，调用方仍应读取语义结果，包括 `valid`、`samples`、计划阻断原因和资源处置。方法不上传、发布、转码或隐式删除轨道；原输入经过复制/快照并保持不变。

过大的内存输入在保留副本前被拒绝。封装 JSON 读取器限制为 64 MiB 头部、128 层嵌套、1048576 个值和最长 1024 字符的数字字面量；重复键或无效 UTF-8 会失败。精确 BigDecimal 数值与十进制字符串媒体时间得到保留。检查适配层最多跨轨道投影 100000 个帧时间；额外时间信息会明确报告超限，原影片仍可获取。

异步 `submit` 使用调用方 Executor 和明确的进度/完成通知。读取进度报告真实字节，原生解析进度不确定。取消是终态，迟到的原生结果会被丢弃，但同步原生代码不能在阶段内部中断。原生调用不输出文件。完成回调在完成或取消操作的调用方/执行器线程上运行。

核心、格式、重封装、共享接口和 WASM bridge 继续禁止 unsafe Rust。只有 `liblivephoto-native` 负责文档规定的外部指针 ABI 和 JNI 导出。C 调用方使用 `native/include/liblivephoto.h`：提供有效不可变字节范围，接收 Rust 持有的不透明响应，并准确释放一次。LLP1 保留原始文件载荷与十进制字符串表示的 64 位时间/偏移。原生指针和 JNI 本地引用不跨调用逸出；panic 会转为明确失败。

主机冒烟测试真实加载动态库，构建动态资产，解析内存、路径及描述符区间输入，检查精确时间，提取相同视频字节，拒绝不精确时间，并测试取消和响应生命周期。原生相册、设备编解码器及 Android 服务接收另行验收。
