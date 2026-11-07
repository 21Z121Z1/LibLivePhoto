# 兼容性与转换

[English](COMPATIBILITY.md)

## 实机验证

维护者于 2026-11-06 确认已完成实机验证，但尚未公开逐设备、OS/相册及操作矩阵。因此不向未指定设备或每一表项自动赋予验证结果。

下表描述自动化证据，使用方字段反映已发布的逐格式记录，不要求重复维护者的实测。运行报告继续返回 `Unverified`，因为库无法从文件结构检查推断设备接收。掌握具体范围后可将已有实测结果补入此处，不从 CI 推导。

## 转换策略

读取、写入、验证及目标使用方状态是独立字段。运行报告对目标使用方使用 `Unverified`；解析或写入成功不证明相机、相册、照片、编辑或设备接收。

`TargetProfile` 选择保留源文件、Android JPEG、Android HEIF/AVIF、Apple 配对或有限的 Samsung JPEG SEF 约定。`WritePolicy` 选择配对身份和无法映射资源的处理方式。写入器不转码媒体；JPEG/HEIF 容器改变会报告 `TranscodeRequired` 并阻止执行。

默认策略拒绝没有证据支持原生映射的资源。`PreserveSidecars` 保留资源精确字节，并报告目标原生资源图不能表达它们；`DropExplicitly` 记录丢弃。保留侧车不代表相册能使用该辅助资源。

报告区分保留、改写、生成、无法映射、丢弃和需要转码的资源。保留证据说明实际比较的属性：完整源字节、视频容器字节、编码媒体载荷、精确照片时间或提取资源字节，不提升为全部厂商语义的保证。

Apple 转换要求明确配对标识符与已知精确时间。Android 转换在时间已知时要求精确的整数微秒时间戳，不静默舍入。Samsung SEF 写入器目前要求时间未知；已知时间的源应使用其他目标或复制源文件。

进行设备验证时，将原样本摘要、设备、OS、相机/相册版本、输出摘要及当前写入策略绑定。分别验证识别、播放、选定视频、封面时间、辅助资源使用、编辑、保存、导出与资源保留；文件名传输成功或 UI 徽标不足以证明这些属性。

缺少人像样本，限制了小米 `depthMotionPhoto`、深度载荷及子视频关系的映射。缺少原始厂商组件与设备时，不能据此宣称相应使用方接收。这些属于外部证据缺口，不等于解析成功或隐含支持。

## 跨格式执行约定

公开 `plan` 与 `execute` 接受所有已实现读取器返回的资源。宣称 OEM 接收前仍需考虑以下限制。

| 源 | 目标配置 | 自动化证据 | 原生使用方 |
| --- | --- | --- | --- |
| Android JPEG，包括有证据的 ColorOS 15/16、小米和 vivo 文件 | Apple 配对 | 原始样本、编码视频载荷与精确时间比较；无法映射资源使用明确侧车 | 未验证 |
| Android HEIF 和有证据的 Samsung HEIF | Apple 配对 | 原始样本、图像项目及编码视频载荷比较 | 未验证 |
| Apple JPEG 配对 | Android JPEG | 合成 Android → Apple → Android → Apple 往返；Android 阶段视频容器字节不变，比较精确时间与编码媒体 | 未验证 |
| Apple HEIF 配对 | Android HEIF | 现有 HEIF 写入器；只有可精确表示为整数微秒的时间才能执行 | 未验证 |
| 照片呈现时间未知的已解析 JPEG | Samsung JPEG SEF | 有界 SEF 直接记录写入器与约定测试 | 未验证 |
| 任意已实现读取器 | 原始源 | 比较完整源字节 | 不证明跨厂商识别 |
| Apple 或其他厂商 | ColorOS、HyperOS、vivo 私有格式 | 未实现专用私有写入器 | 未验证 |

例如 Apple 的 1/3 秒不能精确表示成 Android 整数微秒。即使允许侧车或明确丢弃，Android 计划也会阻断；这些策略不授权改变已知时间。静图容器变化也会阻断，直到调用方提供已转码输入。库不运行编解码后端、HDR 转换、隐式时间舍入或厂商字段推测。

Android 目录根据影片 `ftyp` 主品牌声明 MIME：`qt  ` 使用 `video/quicktime`，MP4 配置使用 `video/mp4`。JPEG/HEIF 写入器保留视频字节。合成 QuickTime 往返与 HEIF 静图样本测试检查实际 XMP 值、视频字节和精确时间。BMFF 解析通过不能证明编解码或相册兼容性；允许类型见官方 [Motion Photo 1.0 MIME 表](https://developer.android.com/media/platform/motion-photo-format#item-mime-type-values)。

Android 目标目前验证容器、资源和时间结构，不证明规范中的视频编解码或 AAC 声道/采样率要求。对 Google 相册 8.0.0.1011730683（versionCode 52642432）的静态阅读不是真机播放结果。该 APK 中，JPEG 动态检测读取 APP1 Adobe XMP；HEIC 检测要求主品牌 `heic`、一个 `meta/iinf` 的 `mime` 项且内容类型为 `application/rdf+xml`，以及末尾顶层 `mpvd`。该 HEIC 扫描返回第一个 `application/rdf+xml` `infe`，不跟随 `cdsc`。HEIF XMP 写入器把主图 XMP 的 `infe` 放在这些项的最前，并保留项目 ID 以及其他 XMP 项的 `iref` 链接。该 APK 的 HEIC 视频定位器只接受主品牌 `mp42` 和 `qt  `。已追到的本地扫描和播放路径没有 AVIF 动态照片分支。`google-avif` 遵守容器规则，不表示相册可以播放。默认构建保留视频字节。`relabel_video_major_brand` / `--relabel-video-brand mp42` 只就地修改主品牌，供 HEIC 定位器使用；它不把 `mp42` 加进兼容品牌，也不移除 PCM 或辅助轨道。主品牌不是 `heic` 的静图（例如 `mif1`）不会被改写。重写 Android XMP 时会去掉遗留的 `MicroVideo=1`，因为该 APK 会让旧偏移优先于目录。新建的 HEIF/AVIF XMP 项在项目 ID 能放进 16 位时使用 `infe` 版本 2。Primary 的 `Padding` 等于实际 `mpvd` 头长（普通为 8，64 位 size 时为 16）。sample-entry 扩展如果不是 box 列表，会报告为不支持的布局，而不是动态照片资源区间重叠。`google-*` 示例名选择这些目录写入器，不认证完整 Google 编解码符合性或 Google Photos 接收。调用方可以用 `plan_with_capabilities` 执行自行声明的 sample-entry、AAC 对象类型、声道和采样率约定；AAC 配置未知时会阻断该严格计划。默认 `plan` 保持容器/资源约定，两者都不证明解码可行性或设备配置。详见[集成](CORE_INTEGRATION.zh-CN.md)。

`cargo run -p liblivephoto --example convert -- PROFILE NEW_OUTPUT_DIR STILL [MOVIE] [--pair-id ID] [--sidecars|--allow-drop]` 是完整的独立消费示例，支持 `preserve`、`google-jpeg`、`google-heif`、`google-avif`、`apple`、`samsung-jpeg-sef`。执行前打印计划，通过库读取器验证输出，只向新目录输出，默认拒绝无法映射资源。侧车按资源 ID 单独保存，`report.txt` 描述处置。新的 Apple 配对需要调用方提供唯一标识符。该示例不是保证原子的生产输出器。

设备验收应记录源/输出 SHA-256、目标配置和策略、设备、OS、相机与相册版本，分别记录识别、播放、封面显示、封面选择、编辑、保存和导出，并对导出文件重复资源、时间及 HDR 检查。缺少相应运行记录的字段保持未验证；示例与 CI 只提供各自的结构证据。
