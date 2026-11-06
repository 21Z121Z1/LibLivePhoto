# 复制资源前检查完整响应

[English](bounded-extraction.md)

共享接口在复制任何选定载荷到输出向量前，先拒绝超出预算的 `resourceIds` 选择。即使 CLI 不序列化 LLP1，也会通过 `process` 收到同样的拒绝。

旧路径逐个复制资源，直到 `into_envelope` 才检查响应。不同 SharedData ID 可以引用同一区间，因此有限源能放大成很多独立输出。私有 `CheckedExtraction` 借用完整选择，通过 `io::sink()` 向核心读取器查询每项精确提取长度，在消费式物化器复制前检查整个响应。成员关系、区间验证和区间顺序仍由核心负责。

计划与公开 `into_envelope` 共用 `checked_response_len`，计入真实序列化 success/metadata 外壳、12 个固定头字节、每文件 20 个固定字节、UTF-8 名称/MIME 和各输出载荷。字段长度必须能用 u32 表示，资源 ID 必须能用 i64 表示。响应上限仍为含边界值的 320 MiB；128 MiB 输入、1 MiB 请求、64 MiB 外壳元数据及 4096 文件上限不变。

受支持输出保持原顺序、字节、名称、MIME、资源 ID 和元数据。重复 ID 仍无效，每个不同 ID 均按完整大小计入，包括别名。预算拒绝保留 `invalidRequest` 代码与 `response budget exceeded` 消息。默认提取使用相同计划/物化器，同时保留已有最多 4 MiB 的有界 HEIF XMP 组装。

## 可复现证据

从仓库根目录运行本地与 CI 共用命令：

```sh
python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction
```

提取重放在临时克隆中运行真实 `process` 与核心提取，仅注入降低的响应上限和资源向量完成写入后的计数器。基线 `b1c556314b0939a6370f68fcd8be63089f7b3883` 在 8192 字节上限下复制 46768 字节并返回成功；修正路径会拒绝，计数载荷字节为零。另一受支持选择产生恰好 4919 个 LLP1 字节，在 4919 上限成功、4918 失败。只计载荷和传输开销用例均在复制前失败。受支持选择、默认结果及缺少 XMP 的结果与基线逐字节相同。

历史重放保留原断言，在失败/通过前检查生产 Git blob，以四项测试覆盖三个旧合计预算修复。

| 修复 | 重放行为 |
| --- | --- |
| `82abd717963f8c6e78ddce3babf22f49427c8048` | 合计评分样本、复制载荷及描述符数量。 |
| `0651b8020a6f4a41a353e9937932b902f1687966` | 重叠 AGTM 样本、载荷和曲线预算。 |
| `71818fbc4d724dea4e326ac642142a5d60e4fd57` | 视频轨道共用一个帧清单预算。 |

失败阶段必须实际运行指定测试，以断言失败并退出 101；通过阶段必须退出 0。E3 只使用 `cd2159d375d22498c9437b7df87c2546700a9288` 的测试依赖，原生产代码前缀在通过阶段之前不改变。收据记录命令、精确修订、源哈希及日志哈希，清理临时目录后仍保留。

这些证据不测量内存峰值、分配器活动、速度、解码或设备接收，不分配生产大小响应或诱发 OOM。接口测试另行检查未知资源顺序、JPEG/HEIF 默认行为、缺少 XMP、后续无效 ID，以及由两个有序区间组成的七字节项目。

该分支还用等价德摩根表达式简化一处继承的 MIME 比较。Rust1.96 Clippy 在原 `-D warnings` 规则下将旧表达式报告为 `nonminimal_bool`，MIME 行为和 lint 严格度不变。

[验证说明及功能索引](../.agents/skills/verify-liblivephoto/SKILL.md)描述后续变更的 launch、doctor、drive 与 cleanup。
