# 第 3 轮自有测试向量

[English](README.md)

这些文件不含私人照片或第三方源码。ICC 文件是独立构造的外层结构向量，含不透明私有 tag，不是完整语义色彩配置。固定的 4.4 检查器有意不支持 4.3。MOV 包含不重新编码的仓库自有中性视频，以及新编码的 AAC-LC/PCE 48 kHz 双声道静音，使用主写入器的 QuickTime wave 外壳。`stz2` 变体只改变样本大小索引，真实包哈希和精确时间线经过检查。

使用 `python3 scripts/create_syntax_fixtures.py --out NEW_DIRECTORY` 重建到新目录。同环境重建记录使用系统 FFmpeg7.1.5-0+deb13u1、原生 AAC 编码器和 bitexact 标志；不承诺其他版本产生相同 AAC 字节。FFmpeg 是此处的 GPL 开发工具，不再分发其可执行文件/源码，也不链接到库中。这些自有数据使用仓库许可证。实际命令、四项哈希及独立研究来源许可证见[来源记录](https://github.com/21Z121Z1/LibLivePhoto/blob/archive/evidence-20261030/evidence/syntax-round3-sources.json)。

`scripts/test_syntax_contracts.py` 使用真实 CLI，并独立调用 ffprobe 检查每个包的 SHA256、精确 DTS/PTS/时长和文件索引目标，不证明音频/色彩渲染或设备/相册接收。

第 4 轮验收清单为 [cases.json](cases.json)，含 42 项精确字段预期、分类错误/Unsupported 报告、原媒体及传输一致性黄金文件。各源/黄金文件都有 SHA256。新增直接 AAC 文件使用同一自有视频与新编码静音，不使用 `-aac_pce`；无效派生文件只改变描述符语法。紧凑索引输入在单个 `mdat` 中加入 103 个实际未引用字节，黄金输出移除这些字节，同时保留全部 17 个包。导入、删除和旧式 `stsz` 黄金文件也经 ffprobe 独立解复用。AGTM 向量只派生自既有自有绑定样本，采用紧凑大小与重定位 chunk 偏移，保留原始字节。黄金文件本身只证明传输一致性，不是独立的格式判定依据。八个输出影片接受逐包哈希、精确 DTS/PTS/时长与 `mdat` 索引检查，共计 198 个输出包。

构建 `liblivephoto-native` 后，使用 `python3 scripts/create_acceptance_fixtures.py --out NEW_DIRECTORY` 将新增清单和数据重建到另一个新目录。生成器拒绝覆盖已有目录。编码器逐字节一致的输出需要相同 FFmpeg 构建；跨平台 CI 使用已提交并带哈希的自有输入，不重新生成。
