# 库资源字节

[English](library-resources.md)

## 子功能

库把不可变资源提取到调用方提供的 writer。有序区间、精确时间、明确的组成部分或 Apple 配对，以及源游标保留，继续遵守既有核心约定。

## 使用入口

调用 `MotionPhoto::parse`、`MotionPhoto::open` 或 `MotionPhoto::open_file`，再对属于该资产的资源调用 `extract`。

## 通过 verify_extraction.py 验证

Drive 阶段通过 Cargo 运行 `core_contract`、`video_metadata`、`agtm_complete` 和 bridge 库测试，检查读取器与当前合计预算回归。接口测试 `selected_heif_item_uses_ordered_extent_bytes_and_exact_length` 还会选择一个 HEIF 项目，其两个区间的顺序与源中的位置顺序不同，预期输出是明确指定的七个源字节。

## 注意事项

HEIF 项目使用合成编码数据。这些测试检查容器字节，不解码图片。接口计划没有替换核心预算或平台实现。
