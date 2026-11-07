# CLI 使用说明

[English](CLI.md)

使用 `cargo build --release -p liblivephoto-cli` 或 `cargo install --path crates/liblivephoto-cli --locked` 构建或安装，二进制名为 `llp`。完整选项和各操作的输入约定见 `llp --help`。

需要只读选择 Samsung SEF 视频时，包括缺少或存在歧义的主视频声明，使用 `llp extract input.jpg --options-json '{"samsungRecordIndex":"0"}' --out new-selected`。索引是十进制字符串，指向已验证的 SEF 记录，不是 ResourceId 或播放角色。该操作只返回原视频，不修复或重新关联资产；不能与第二输入、resourceIds、静图/XMP 输出、时间或严格资产解析标志同时使用。提供者禁用控制仍有效，普通解析和默认提取仍拒绝歧义。详见厂商选择样本约定。

```sh
llp version
llp parse motion.jpg
llp check motion.jpg
llp metadata ordinary.heic
llp build still.jpg --movie video.mp4 --time 13/16 --target google-jpeg --out new-built
llp build still.heic --movie video.mp4 --time 13/16 --target google-heif --relabel-video-brand mp42 --out new-heic
llp extract motion.jpg --still --video --xmp --out new-extracted
llp agtm motion.jpg --time 13/16 --out new-agtm
llp agtm movie.mp4 --video-input --time 13/16 --out new-agtm-from-video
llp xmp metadata.xmp
llp optimize movie.mp4 --out new-optimized
llp import-tracks destination.mp4 --movie source.mp4 --tracks 1 --out new-imported
llp remove-tracks movie.mp4 --tracks 3 --out new-removed
llp conformance still.avif --options-json '{"profile":"heif-color-2025"}'
llp conformance movie.mp4 --tracks 1,2 --options-json '{"profile":"google-video-container-1.0"}'
```

Google JPEG、HEIC 和 AVIF 构建各输出一个容器，文件名是 `assetMP.jpg`、`assetMP.heic` 或 `assetMP.avif`。`MP` 后缀对应 Motion Photo 1.0 的文件名建议。`--relabel-video-brand mp42` 只就地改写内嵌视频 `ftyp` 的主品牌，不重新封装、不转码、不删除轨道。主品牌已是 `qt  ` 时保持不变。该选项只用于三个 Android 目标。

从独立静图和视频构建时，默认明确声明组成部分关联。使用 `parse` 解析两个文件时会验证 Apple 配对，除非设置 `--parts`。Apple 标识符不匹配时不会静默重新关联。精确时间使用整数分数；目标要求微秒表示时拒绝 1/3 秒。使用 `--dry-run` 在生成文件前检查构建计划。不隐式转码容器或编解码格式，不支持的目标、元数据或资源会返回诊断。

`apple-pvt` 把结果写成以 `.pvt` 结尾的新目录：

```sh
llp build IMG_0010.HEIC --movie IMG_0010.MOV --target apple-pvt --out IMG_0010.pvt
llp parse IMG_0010.pvt
llp build motion.jpg --pair-id 11111111-2222-4333-8444-555555555555 --target apple-pvt --out motion.pvt
```

目录内是 `{stem}.heic` 或 `{stem}.jpg`、`{stem}.mov` 和 `metadata.plist`。plist 中 `PFVideoComplementMetadataVersionKey` 的值是字符串 `"1"`。已有 Apple 配对按原字节复制，包括 MakerNote 标识、`com.apple.quicktime.content.identifier` 和 still-image-time。Motion Photo 或 `--parts` 输入走 Apple 配对写入器。`--pair-id` 把同一个标识写入静图和视频；本库不把 JPEG 静图转码成 HEIC。只有 `--allow-drop` 会省略无法映射的资源，这些资源不会进入包内。包内没有 `report.json`，JSON 报告在标准输出。`parse` 和 `check` 可以读取该目录。包结构有效不代表 Photos 可以导入。

JPEG/HEIC/AVIF、已有 Apple 支持、提供者过滤、明确的侧车/丢弃策略及支持的轨道操作，都与 Rust/JNI 共用库代码。CLI 提供 `--disable-3p`、`--enable-provider`、`--strict`、源 `--range OFFSET:LENGTH`、选定资源 ID 和 `--options-json`。区间针对所选快照，`inputRanges` 单独保留外层文件偏移。使用 `-` 读取有大小限制的标准输入。独立 AGTM 必须明确提供时间，不会伪造零值；其输出是元数据，不是渲染像素，也没有 Moments 构造参数。

`conformance` 是可选的只读配置查询，独立于默认保留/检查行为。视频轨道 ID 明确指定主角色与可选次角色。报告区分必要规则违反、不支持的事实及建议，只覆盖列出的规则。详见[配置边界](CORE_INTEGRATION.zh-CN.md)，包括 AAC 声明和公开文本中 44 kHz/44100 Hz 的歧义。通过报告不代表解码器、完整格式或原生相册认证。

报告通过标准输出返回 JSON。退出码 0 表示操作或检查通过，1 表示有详细信息的库/计划/检查失败，2 表示命令、I/O 或文件输出使用错误。生成和提取文件必须指定 `--out NEW_DIRECTORY`。命令使用 CREATE_NEW 创建新目录和各文件，不覆盖原件及已有输出。普通写入失败只回滚本次创建的文件，并只在目录为空时移除目录；清理失败会报告剩余路径。`report.json` 最后写入。这不保证其他进程观察到的原子可见性，也不保证断电或进程终止后的恢复；调用方应在成功退出后再读取报告。侧车文件与主结果一起输出，不代表原生相册会使用它们。

优化器和轨道导入器只接受已验证的独立布局，会报告未引用媒体、填充和源元数据侧车。不支持的引用、分片、`sidx` 或未知偏移约定会明确失败。支持查询通过不表示兼容任意 MP4。详见[验收清单](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/ACCEPTANCE_CHECKLIST.md)。

ICC 结构查询接受原始 ICC 数据，不接受外层图片：

```sh
llp conformance profile.icc --options-json '{"profile":"icc-structure-v4.4"}'
```

范围报告区分不支持的版本与已检查的 4.4 外层结构，未知和私有载荷不做语义解释。解析样本描述与可选一致性检查中包含 AAC-LC/PCE 和有边界的 QuickTime wave/esds 子集，不认证完整 ICC/AAC 渲染。非分片 `stz2` 索引的 4/8/16 位形式可用于有界优化、导入和删除；`sidx`、分片或外部引用组合仍会失败。详见[第 3 轮来源与执行](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/HANDOFF_ROUND3_20261019.md)。
