# 核心集成

[English](CORE_INTEGRATION.md)

本阶段扩展 PR #1 提交 `794ab552596db0f66095600d23cf7937a631b9ec` 的实现，不替换已有资源模型、Apple 写入器或 plan/execute 边界。[初始覆盖矩阵](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/GOOGLE_CORE_COVERAGE.md)是历史检查点；[当前清单](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/GOOGLE_COMPLETE_INVENTORY.md)与[源码/测试映射](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/SOURCE_TEST_MAP.md)记录已实现子集、各操作约定和剩余验收要求。

## 输入和输出

厂商检查的[第 5 轮约定矩阵](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/VENDOR_PRESERVATION_ROUND5.md)区分已有资产级资源提取、新增只读 Samsung 清单及 vivo 声明审计。明确流选择复制选中原始容器字节，不证明相册播放角色或原生写入能力，两种审计都不猜测专有时间单位。

`parse(Input, options)` 借用内存，`open(PathInput, options)` 持有有界快照。`PathInput::Parts` 从路径明确组合静图/视频，由调用方提供精确时间，不从文件名推断配对。

`open_file(FileInput, options)` 为已打开的普通文件截取精确区间快照。`FileInput` 支持单文件资产、Apple 配对和调用方声明的组成部分。`FileRange { file, offset, length }` 必须非空且位于文件内。资源图偏移相对快照，合计输入预算在分配前应用到两个部分。读取中输入缩小会导致 I/O 错误，调用方必须保持内容稳定。

Unix 的 `open_fd(BorrowedFd, offset, length, options)` 复制借用描述符，使用定位读取，调用方保留所有权与游标。Windows 的 `open_file` 在 `seek_read` 后恢复游标；调用方必须同步对该文件和克隆句柄的其他操作。Windows/macOS 测试及打包消费程序在基线 `4848e5f` 已实际通过，见[执行收据](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/stable-baseline-4848e5f.json)。其他平台报告不支持定位输入。零长度不表示文件剩余部分。

所有输入形式进入相同解析器与资源提取器。`extract` 写入任意 Rust `Write`，包括调用方打开的文件、基于描述符的 `File` 或向量。`plan(...).execute()` 返回 `ComposedAsset` 缓冲区及报告，没有直接流式 build-to-fd API、输出区间覆盖或隐式发布；调用方负责发布和原子性。`still_bytes()` 返回含嵌入辅助载荷的静图容器，不是解码像素。

`xmp_bytes()` 返回原始主 XMP 包，不重新序列化。JPEG 借用不含 APP1 标识符的包；HEIF/AVIF 组装有界 XMP 项目区间。`None` 表示没有包；格式错误或有歧义的元数据返回错误。容器元数据资源仍可独立提取。

## 目标选择与媒体要求

`AndroidAvif` 要求 AVIF 静图，`AndroidHeif` 保留 HEIC 约定，两者改写元数据并追加 `mpvd`，都不解码或重新编码像素。`ConversionPlan::relabel_video_major_brand` 可以把 Android 内嵌视频的 `ftyp` 主品牌就地改成 `mp42`。它不移动样本偏移、不改兼容品牌、不转码、不删除轨道。`qt  ` 保持不变。默认计划保留视频字节。AVIF/HEIC/JPEG 不匹配时需要调用方控制的转码，Apple 写入器不增加 AVIF 目标。Android 时间仍要求精确整数微秒，侧车/丢弃策略也不能允许 1/3 秒。

`sample_descriptions()` 提供轨道 ID、每个 sample-entry FourCC、原始编解码及 HDR 属性、精确 tkhd 矩阵、编码 sample-entry 尺寸，以及音频版本/样本大小声明。sample-entry 扩展如果不是 box 列表，返回 `UnsupportedLayout`（"sample entry extensions are not a box list"）。这不是动态照片资源区间重叠。`inspect_video_sample_descriptions` 在合计预算内检查独立 MP4/MOV。AAC AudioSpecificConfig 提供对象类型、采样率和声道数。

声道配置为零时，只解释有界 AAC-LC PCE 子集：front/side/back SCE/CPE 与 LFE 声明参与声道计数，data/coupling 元素不参与。原始 ASC/注释字节和声明 tag 被保留。核心编码器依赖、GA 扩展、非 LC/含 SBR 的 PCE 及未知尾部语法不受支持；识别明确的无 SBR 同步后缀。

QuickTime mp4a 条目版本 0/1 可以使用直接 esds，或一个有界 wave，其中 esds 唯一、frma 匹配且最后有空标记，可选全零 mp4a 标记。冲突/混合描述符、未知子项或 esds 版本会被拒绝解释。QuickTime v2、编解码级 VUI/SEI 与完整解码器符合性不在范围内。原始 sample-entry 音频字段和 AAC 声明是不同事实，库不会协调它们之间所有可能的分歧。

调用方可向 `plan_with_capabilities` 提供 `MediaCapabilities`，例如允许 `avc1`/`hvc1`、AAC 对象类型 2、最多双声道及 44.1/48 kHz。这些是示例调用方选择，不是推断设备限制。每个样本描述都必须满足约定。已知不兼容编解码器或音频参数产生 `TranscodeRequired` 并阻止执行。未知 AAC 事实产生无法映射/阻断原因，不武断宣称需要某种不支持的转码。默认 `plan` 只承诺容器/资源保留，库不选择编码器或转码。

可选 `validate_google_video_container` / `google_video_conformance` 检查明确列出的 Google1.0 声明、数量与顺序规则。调用方指定主/次轨道 ID，未知角色不会被视为通过。`Violation`、`Unsupported` 和 `Recommendation` 分开表示。次视频精确逐帧对应属于建议，使用支持编辑后的有理数 CTS，不选择最近帧。样本尺寸是观察事实，来源没有固定分辨率要求。

AAC 48/96 kHz 和字面值 44000 Hz 按原值检查。公开页面写 44 kHz，因此 44100 Hz 报 Unsupported，不静默重新解释；调用方能力约定仍可明确允许 44100。支持的 LC/PCE 与 wave 声明参与可选配置；QuickTime v2、扩展/未知语法和不支持的编解码声明仍是明确限制。报告不证明码流、像素或原生设备兼容性。

CLI/C/JNI 对单个不可变输入提供只读 `conformance`：

```sh
llp conformance STILL.avif --options-json '{"profile":"heif-color-2025"}'
llp conformance VIDEO.mp4 --tracks 1,2 --options-json '{"profile":"google-video-container-1.0"}'
```

视频配置将第一个 ID 视为主轨道，可选第二个视为次轨道，接受独立影片。动态照片调用方可先提取原视频，或调用 Rust 方法。必要规则失败或 Unsupported 返回 `success=false` 和规则诊断；建议仍然显示。默认 `check`、解析/构建及字节保留各自独立。Android 远程服务保留原角色白名单，不宣称新增动作是已安装服务功能。

## 显示和色彩事实

`still_metadata()` 提供原始 EXIF 方向，以及八种 EXIF 方向和 HEIF `irot`/`imir` 共用的正交变换表示。按明确提供的 ISO23008-12:2025 §6.5.12 文本和 libavif 公开声明，`imir` axis0 交换上下，axis1 交换左右。矩阵使用 x 向右、y 向下坐标。API 分别保留 EXIF 与 HEIF 声明，不推测它们的组合顺序、不应用裁切，也不归一化任意影片矩阵。缺少方向与零等未知厂商值不同，两者都不会变成已观察到的单位变换。

ICC 配置保留为不透明字节。CICP 的原色、传递函数、矩阵和范围是明确字段；传递代码 16/18 表示已声明 PQ/HLG，缺少或未解释色彩元数据不等于 SDR。主 HEIF 属性顺序、未知属性、编解码配置和原始 HDR 属性被保留。

可选 `inspect_icc_structure(profile)` 借用原始 ICC 配置和 tag 区间，偏移从该配置字节 0 计算。传入提取或重组后的 ICC 字节，而不是外层 JPEG/HEIF。`validate_icc_structure` 和 `conformance` 配置 `icc-structure-v4.4` 检查选定的 ICC.1:2022 头部、精确长度及有界 tag 拓扑，包括完整共享区间和填充。未知/私有 tag 保留原始数据，其他版本报告 Unsupported。日期、类别、PCS 语义、必需 tag、profile-ID 哈希、LUT 解释及色彩渲染不在认证范围。默认保留不将此检查作为写入修复，也不归一化原 ICC；没有 gain map/HDR 渲染。

[规范/来源/运行映射](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/NORMATIVE_CONFORMANCE.md)记录支持的条款子集。支持 HEIF 自包含 construction0/1，明确拒绝 construction2 和外部项目源。旧 meta 载荷内的绝对项目数据在只追加元数据替换后仍保留，construction1 区间仍位于对应 idat。有界元数据组装器在复制前检查 4096 个区间和 16 MiB 被引用字节。

`validate_heif_color_information(still)` 与 `MotionPhoto::heif_color_conformance()` 可选检查 ISO23008-12:2025 §6.5.5 的数量、ICC/nclx 配对及配对原色/transfer=2 声明，合并所有 ipma 中受支持的图像项目关联。类型化报告区分 `Violation` 与 `Unsupported`，`checked_rules_satisfied()` 要求两类问题都不存在。独立报告只覆盖列出的规则，不代表完整 HEIF/BMFF、ICC 有效性或渲染色彩。未知项目、色彩类型/布局及重复关联不会被猜测。调用方可要求规划/发布前检查报告，默认保留不会为通过检查静默改写矛盾声明。

查询明确限制：项目、属性、关联行及问题各 4096，关联总数 65536，被引用 colr 字节 16 MiB。这是实现预算，不是规范最大图像大小。事实提取最多接受 4096 个主属性关联及 16 MiB 载荷，避免重复引用无界放大分配。转换计划在写入前报告检查失败。

写入器在改写后比较静图显示/色彩事实，以及已有编码媒体和精确时间。报告中的 `StillDisplayMetadataEqual`、`StillColorMetadataEqual` 表示实际比较，不认证全部色彩、裁切、gain map 或厂商显示语义。

## 定时 AGTM 与明确重封装

`inspect_camera_album_footer` 读取两份独立 vivo 原件中观察到的精确 CameraAlbum 末尾结构。在明确大小、嵌套及重复键限制内，提供原 JSON/区间、整数声明，以及外层结构和 JSON ID 是否一致。`MetadataEngine` 通过内存/路径/fd/区间 API 和 `metadata` 请求提供 `camera_album` 提供者，以及可完整提取的 `camera-album.footer` 块，受 `disableThirdParty`/启用列表控制。未知 JSON 在 `rawJson` 中保持原文和数值拼写，不把未知流称为深度，不以 ID 相同证明拍摄配对，也不通过平均 FPS 换算原始 `imageTime`。独立的 55 字节/length2=2 变体只有静态/合成证据，不新增原生厂商写入器或相册播放/编辑声明。详见[厂商证据与缺口](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/VENDOR_RESEARCH_20261019.md)。

`inspect_vivo_media_extensions(movie)` 与 `MotionPhoto::vivo_media_extensions()` 从完整、连续且首盒为 `ftyp` 的 BMFF 快照读取顶层 `vivoMediaEStream`/`vivoMediaExtInfo` UUID。前置其他盒或不连续布局不在这一附加检查约定内。两份独立 vivo 原件各有 31 个流记录。API 借用精确载荷，报告有序数值键、不透明编码 tag、目录/结构区间和已有 CameraAlbum 尾部。重复键保留为独立记录，未知 tag 不赋予深度、人像、陀螺仪或渲染语义；尾部时间单位和配对唯一性仍未验证。

方法区间相对原影片，`vivo_media` 提供者将可提取的 `vivo.stream-payload`、目录及 footer-box 块映射到外层源快照。路径/fd/区间元数据调用与 C/JNI 共用适配层。独立容器检查不等于影片验证或原生相册接收。格式错误的可选厂商元数据产生诊断，原容器尾部仍可提取。多个已知 UUID 需要明确关联，否则失败。限额为 256 MiB 输入、65536 个顶层盒、4096 个流、每种已知 UUID 各一个、合计流载荷 16 MiB；元数据集合也保留 4096 块/16 MiB 合计限制。不新增原生写入器。详见[独立来源和原字节证据](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/VENDOR_RESEARCH_ROUND2.md)。

`agtm_at(MediaTime)` 返回原始元数据样本字节、轨道/样本 ID、呈现区间及解码后的版本 0 基础字段。它读取已验证样本表并遵守 `ctts`，明确的 `it35`/T.35 信令建立关联。缺少样本返回空向量。单位速率编辑列表把空段、裁切段和重复段映射到影片时间，在编辑边界裁剪样本区间，不改变载荷或源样本索引。编辑时长与查询必须能精确换算成轨道刻度；非单位速率、溢出、分片、不支持版本、截断基础字段或非整数时间换算会明确失败。不扫描任意影片字节、不用第一个样本替代缺失样本，也不为缺失/截断内容伪造基础数据。

`decode_agtm` 解析固定 Google 解码器读取的全部版本 0 字段：所有替代图规则、公共/独立曲线和混合、x/y 坐标、显式/PCHIP 斜率选择、角度与切线斜率、模式选择器和自定义色度。截断会失败，完整原始输入及未知尾随位保留。Google 忽略六个编码混合系数，其 mix 结构保持零值；本 API 提供原始整数，不推测权重缩放，也将未知预设保留为选择器。`reference_json()` 提供 Google 首规则投影，`details_json()` 提供所有规则。`extract_agtm_from_video` 支持独立 MP4/MOV。输出是元数据，不是渲染像素。`decode_video_metadata` / `extract_video_metadata` 解码已有 Google 评分和防抖描述符，保留原始/未知位、精确时间及语义问题。九项同输入解码向量与固定参考实现匹配，明确排除 Moments 构造。

`metadata_packets` 报告编辑前轨道媒体时间线上的原始解码顺序包，包括合成偏移；不支持的编辑速率也能查看这些原始包。`extract_video_metadata` 把载荷内时间戳按声明的影片时间验证，包括与 XMP 直接比较，绝不再加元数据轨道的 ctts 或编辑偏移。

`project_scoring_media_times` 是针对调用方明确声明的编辑前视频媒体 CTS 的替代方案。调用方分别标识主/高分辨率视频轨道；它精确匹配含 ctts 的源 CTS，再按单位速率编辑映射样本索引。评分引用保留原始位，排序后的影片时间显示重复，明确区分已裁切和未指定状态。缺少帧匹配、不支持速率、溢出、分片、非整数微秒或超出合计输出限制均失败。不生成替代载荷，不自行选择某次重复。定时 AGTM 扫描样本前先选定一个顺序编辑，避免样本数 × 编辑数放大工作量/分配，现有合计提取预算仍有效。

`remux::remove_metadata_tracks(movie, ids)` 是明确选择的删除操作，只接受唯一元数据轨道 ID。拒绝分片/含段索引的影片、轨道引用及不支持的影片级盒。保留原媒体数据，追加有效 `moov`，前后均验证轨道，报告列出删除和保留 ID。未引用样本仍留在 `mdat`，不会擦除其内容，也不压缩文件、不从其他影片导入轨道、不实现 fast-start 优化。默认提取/构建不删除元数据轨道，包括 AGTM。

其他 `remux::import_tracks`、`remove_tracks`、`optimize`、`optimization_support`、`track_summary` 提供有界通用重封装：改写/升级 chunk 偏移，保留编码样本与精确时间，给导入轨道分配唯一 ID，并报告元数据侧车/丢弃字节。只有明确允许压缩才移除未引用媒体。额外非分片索引布局为 `stz2`（版本 0，4/8/16 位），与 `stsz` 互斥。保留已有 stsc/stco/co64、精确时间和自包含 mdat 检查；改写保留紧凑 stz2 字节，同时升级/重定位 chunk 偏移。段索引 `sidx`、分片、外部引用及未知带偏移约定仍被拒绝。固定 MP4CopyTrack/优化器/删除差分覆盖真实 H.264 B 帧、AAC、已有 Apple 元数据和 hvc1，见 `scripts/mp4_differential.py`。

`MetadataEngine` 读取普通 JPEG/HEIC/AVIF 的块/头部区间、原始元数据、提供者投影与处理控制。已保存集合的分类会清除源验证。完整选定块可在单独预算内从原源提取。对固定提供者/拆分器未提供的等价命名空间前缀和无 mpvd 的普通 HEIF XMP，也会在支持时提供。原始元数据提取不解码像素。

## 消费方迁移

已有公开资源/轨道字段、精确时间规则和 `xdremux-compat` 导出保留。`PathInput` 新增 `Parts`，下游穷尽匹配必须处理。新目标配置和诊断使用已有非穷尽枚举。公开转换示例新增 `google-avif` 目录写入器，不是设备认证配置。不稳定兼容谓词 `is_heif_mime` 现包括 `image/avif`；只接受 HEIC 的下游产品路径必须明确区分 AVIF。

对 XDRemux PR #58 的 `0688ea7d6270df52cc5968e2c36dd016f13bd31b` 只做了读取，其仍固定旧库 SHA，不改消费者文件或依赖固定值。可用 `scripts/consumer_replay.py` 对本库重放未改的公开互操作测试；外部依赖别名不是完整 XDRemux 产品构建。普通外部消费程序也执行 Android → Apple → Android → Apple 路径。原生照片/相册接收、私有厂商写入器，以及应用编解码/HDR 策略各自独立。

第 4 轮源码 **bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7** 通过三组工作流、八个作业，包括所有适用原生、移动及真实浏览器入口的 42 项新增语法/字节/时间/错误向量。详见[交接](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_ROUND4_20261019.md)、[终态收据](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/platform-bd6f85b.json)与[剩余边界](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/REMAINING_WORK.md)。该历史记录对原生相册/照片及完整 ICC/AAC/渲染/设备兼容性仍标为未验证或部分验证。

第 3 轮给 `AacConfiguration` 新增公开 `channel_configuration` 与 `program_config`。原有方法/读取字段及带 `..` 的解构仍可使用；原先完整结构字面量会实际报 E0063，需要新增观察字段。这是 Rust 源码兼容变化，不是 C/JNI ABI 变化。严格能力计划将无法检查的 esds 标记为每资源 Unmapped；默认保留计划仍保留完整源影片。非 PCE 后续原始 AAC 语法保留，但不做完整 GA/扩展验证。

使用 `scripts/consumer_replay.py --xdremux-repository PATH --check-consumer-sources`，可在独立测试环境中通过明确本地依赖覆盖编译原消费方 facade、示例与测试源码。实际 XDRemux 仓库/依赖固定值不变，这不是完整产品构建。

当前维护者确认的实机状态见[兼容性记录](COMPATIBILITY.zh-CN.md#实机验证)。
