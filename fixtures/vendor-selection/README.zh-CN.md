# 明确记录选择与源区间约定

[English](README.md)

既有 13 个厂商用例和 42 个语法用例保持原样。独立的 20 项套件加入 9 项成功的原始字节选择、10 项明确拒绝，以及一项 vivo 尾部别名/源坐标检查。素材是自有中性媒体和独立构造的结构，不是原设备证据或原生 Samsung/vivo 写入器。

movieA 是仓库自有 H.264 演示，movieB 是既有 CC0 AGTM 测试容器。两者完整字节不同，可发现错误/默认流选择。两个指针记录有意引用同一物理影片；另一个歧义目录有两个不同主载荷。预期源区间区分位于不同位置的相等字节。vivo 用例保留精确原始 JSON 与源区间，不猜测时间单位或播放/流角色。

共享 LLP1 提取请求使用十进制字符串 `samsungRecordIndex`，例如 `{"action":"extract","samsungRecordIndex":"0"}`。它执行既有有界只读 Rust 清单/明确选择，复制已经验证的原视频，不修复或重新关联资产，不转码，也不改变原生语义。该选项与第二输入、resourceIds、静图/XMP 输出、时间或严格资产解析标志互斥。普通解析与默认提取仍拒绝歧义。Android 使用原有提取服务和白名单，新验收套件没有 JNI 回退。

`../contract-suites.json` 在所有真实入口锁定清单摘要与精确有序 ID，检查样本哈希、源坐标和返回视频的每个字节。重建时使用**新**目录：

```sh
python scripts/create_vendor_selection_fixtures.py --out /tmp/new-selection-fixtures
```
