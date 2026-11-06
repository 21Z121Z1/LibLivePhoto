# 提取功能索引

[English](README.md)

此索引覆盖有边界限制的资源提取变更涉及的库、共享接口和 CLI 行为。

| 功能 | 证据 |
| --- | --- |
| [库资源字节](library-resources.zh-CN.md) | 核心读取器测试，以及共享接口的有序多区间提取。 |
| [共享操作响应](interface-extraction.zh-CN.md) | 未知资源、请求顺序、默认行为、错误、LLP1 长度及缩小限额的公开路径重放。 |
| [CLI 输出](cli-extraction.zh-CN.md) | 通过真实二进制完成构建、检查、逐字节提取和拒绝覆盖。 |

统一命令是 `python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction`。所有条目均在其 drive 阶段运行。
