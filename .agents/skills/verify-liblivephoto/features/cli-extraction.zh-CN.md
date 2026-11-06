# CLI 输出

[English](cli-extraction.md)

## 子功能

CLI 使用静图和视频构建动态照片，检查生成资产，提取原视频，并拒绝已经存在的输出目录。

## 使用入口

使用 `llp build STILL --movie MOVIE --time 13/16 --target google-jpeg --out NEW_DIRECTORY`，再对生成资产使用 `llp check` 和 `llp extract INPUT --video --out ANOTHER_NEW_DIRECTORY`。

## 通过 verify_extraction.py 验证

Launch 构建真实 `llp` 二进制，Doctor 检查其 LLP1 版本。Drive 运行已有的 `real_cli_build_check_extract_round_trip_and_refuse_overwrite` 命令测试，将提取视频与演示输入比较，检查预期输出文件名，并确认拒绝提取后已有报告的字节不变。

## 注意事项

命令测试自行创建和清理临时文件，命令日志与收据在清理后保留。演示输入用于本地结构和字节保留检查，不属于设备接收验证样本集。
