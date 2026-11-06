# 术语

[English](TERMINOLOGY.md)

| 术语 | 定义 |
| --- | --- |
| 资产（Asset） | 具有已声明或已观察到的身份、时间或语义关系的资源集合。 |
| 源（Source） | 一段不可变的输入字节序列，一个资产可以具有多个源。 |
| 资源（Resource） | 某个源的可提取视图，具有有序字节区间与角色。 |
| 区间（Extent） | 左闭右开的字节区间 `[lower_bound, upper_bound)`。 |
| 容器（Container） | 资源的 JPEG、HEIF 或 BMFF 物理结构。 |
| 轨道（Track） | 视频容器中的 BMFF 轨道，其身份与资源身份相互独立。 |
| 布局（Layout） | 资源在源中的物理排列方式。 |
| 格式（Format） | 标准规定的资源与元数据约定。 |
| 方言（Dialect） | 超出标准约定的厂商或历史解释。 |
| 信息来源（Provenance） | 一项事实的依据：声明、观察、恢复或推断。 |
| 照片呈现时间（Presentation time） | 与静图关联的精确媒体时间。 |
| 配对标识符（Pairing identifier） | 多个资源共享的声明身份；文件名不属于配对标识符。 |
| 次视频（Secondary video） | 区别于主视频的视频，该名称本身不定义用途。 |
| 静图替代视频（Substitution video） | 有证据支持的约定允许其替代静图的视频。 |
| 侧车文件（Sidecar） | 在目标原生资源图之外保留字节的输出。 |
| 兼容性（Compatibility） | 指定使用方及版本接受某项内容的情况，并附有证据。 |
| 保留证据（Preservation evidence） | 转换期间完成的字节或精确时间比较。 |

对有证据支持的属性使用 `preserved`。只知道媒体载荷相等时，不要把整个转换称为 `lossless`。即使没有通用角色映射，也必须保留未知的厂商角色名称。
