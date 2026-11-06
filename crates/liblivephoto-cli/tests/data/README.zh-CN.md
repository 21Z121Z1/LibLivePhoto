# 随包测试数据

[English](README.md)

这些文件是本 crate 测试所需仓库素材的逐字节副本，保证已发布源码包能在没有仓库检出的环境中运行测试和示例。`scripts/verify_crates.py` 会在打包前检查每个镜像文件；更新时从仓库中相同相对路径复制。

`web/` 与 `fixtures/bindings/` 下的文件保留原始 CC0 来源 JSON。这些 JSON 描述原始完整集合，本目录只携带所需子集。语法与厂商选择向量由项目自行构造，使用 Apache-2.0 许可证。`fixtures/manifest.json` 只包含外部原始样本的哈希，不携带这些照片。原有被忽略的外部样本测试仍要求调用方提供原文件。
