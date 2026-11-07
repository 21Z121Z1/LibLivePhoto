# 格式支持

[English](FORMAT_SUPPORT.md)

状态对应下列约定，厂商名称本身不证明设备支持。维护者已完成实机验证，表格仍只描述自动化证据。完成情况与逐格式公开覆盖的区别见[设备记录](COMPATIBILITY.zh-CN.md#实机验证)。

| 约定 | 读取 | 写入 | 结构验证 | 原始使用方 |
| --- | --- | --- | --- | --- |
| Android 1.0 JPEG 目录 | 真实样本 | 往返、媒体与时间比较 | 测试及格式错误用例 | 未验证 |
| Android HEIF `mpvd` | 真实样本 | 只追加元数据；项目和时间比较 | 测试及格式错误用例 | 未验证 |
| Android AVIF `mpvd` | 合成约定及一份生成 AVIF/参考比较 | 只追加元数据；编码项目、视频及时间不变 | 参考重新解析与独立静图解码 | 未验证 |
| Apple JPEG/HEIF + MOV 配对 | 原始 iPhone15 配对与生成样本 | 配对元数据、精确时间线、媒体比较 | 配对及已支持定时元数据 | 未验证 |
| 旧 Google MicroVideo | 合成约定测试 | Android 转换及源复制 | 边界与目录检查 | 未验证 |
| OPlus ColorOS15/16 扩展 | 原始样本，含双流 | 源复制；明确侧车转换 | LPEX、拓扑及边界测试 | 未验证 |
| Oplus OpCamera/JXRS 提供者布局 | 合成同输入参考用例；区分 HEIC 声明和视频 | 源复制，或在明确无法映射资源策略下写标准目标 | 普通/短/校验和/JSON 目录结构及类型字段；保留校验和字节但不宣称验证其语义 | 未验证；原始 LPEX 样本不能证明这些变体 |
| Samsung SEF106/107 | 原始直接及指针记录，含无版本 HEIF | 时间未知的 JPEG 直接 SEF；明确无法映射策略 | 目录及无效输入测试 | 未验证 |
| 小米 Android 风格实况照片 | 一份原始样本 | Android 目标及明确侧车 | 标准约定检查 | 未验证 |
| vivo Android 风格单文件/CameraAlbum43 尾部 | 两份原始样本；有界尾部/JSON 检查 | Android 目标及明确侧车，无原生尾部写入器 | 标准约定、精确尾部区间/字节及声明 ID 一致性 | 未验证；imageTime 单位与拍摄配对唯一性未确定 |
| CameraAlbum55 尾部 | 只有合成/静态约定 | 无原生写入器 | 精确结构、限额、原 JSON 和无效声明 | 未验证；无原设备样本 |
| vivoMediaEStream / vivoMediaExtInfo UUID | 两份独立原件，各 31 个不透明流；ftyp 在首的连续快照，有界有序提取及尾部结构 | 保留原影片字节，无原生 UUID 写入器 | 流/目录/结构区间、大小、重复键报告及错误/预算检查 | 未验证；键/tag 不代表已解码流角色或原生相册证据 |
| 小米人像/深度、vivo 双流/人像 | 无原始样本集 | 无专用写入器 | 可保留未知字节 | 未验证 |
| HarmonyOS MovingPhoto | 已研究平台 API | 调用方负责平台集成 | 不宣称私有磁盘格式约定 | 未验证 |
| 荣耀 JPEG `LIVE_` 20/40 字节尾部与反向 TLV | 合成约定及同输入参考比较 | 源复制或明确侧车，无荣耀原生写入器 | 有界记录、时间、歧义与无效输入 | 未验证；无荣耀原始样本 |
| 其他华为/荣耀私有方言 | 无匹配原始样本 | 无专用写入器 | 不作声明 | 未验证 |

Google 相册播放不是本表的一行。写入器按 Motion Photo 1.0 的容器规则处理 JPEG、HEIC 和 AVIF。对一份相册 APK 的静态阅读显示更窄的 HEIC 定位器，且没有追到 AVIF 动态路径。见[兼容性](COMPATIBILITY.zh-CN.md)。

样本不能覆盖全部固件、模式、编解码器或元数据编码。严格模式要求满足已实现标准约定，或明确的 Apple 配对；兼容模式可在 Android 目录不可用时使用有效 SEF 目录；取证模式报告恢复来源，不捏造时间戳。

Apple 时间读取器支持已有证据的 `mebx` still-time 编辑列表形式。其他定时元数据编码、分片影片重封装与不支持的编辑列表仍有限制。大文件需提高输入预算，未实现流式随机访问解析。

Google 的 Oplus JXRS 校验和/JSON 目录用例及纯 OpCamera XMP 有独立于原始 ColorOS LPEX 样本的有界合成/参考覆盖。校验和语义与原生相册兼容性仍未确定。不新增 Oplus 原生写入器或 stream2 语义角色。见[源码/测试映射](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/SOURCE_TEST_MAP.md)和[五厂商证据矩阵](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/VENDOR_RESEARCH_20261019.md)。

仓库包含合成测试向量及外部原件哈希，不再分发外部照片或专有组件。

第 3 轮加入可选 ICC4.4 外层检查、AAC-LC PCE 声道声明和 entry-version0/1 wave/esds 检查，以及非分片索引替代 `stz2` 的 4/8/16 位形式。这些是有边界的语法/保留约定，不是完整 ICC/AAC 语义、色彩/音频渲染或原生相册支持。见[范围及执行](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_ROUND3_20261019.md)。

第 4 轮已验证源码 **bd6f85b4a75201e6ed06154dbe6b70b5dd0a38f7** 通过三组工作流、八个作业，包括在所有适用原生、移动和真实浏览器入口执行的 42 项新增语法/字节/时间/错误向量。详见[交接](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_ROUND4_20261019.md)、[终态收据](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/platform-bd6f85b.json)和[剩余边界](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/REMAINING_WORK.md)。该历史记录对原生相册/照片和完整 ICC/AAC/渲染/设备兼容性仍标为未验证或部分验证。

当前维护者确认的实机状态见[兼容性记录](COMPATIBILITY.zh-CN.md#实机验证)。
