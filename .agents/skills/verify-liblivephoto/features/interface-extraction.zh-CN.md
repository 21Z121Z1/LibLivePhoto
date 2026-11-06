# 共享操作响应

[English](interface-extraction.md)

## 子功能

明确指定 `resourceIds` 时保留顺序与未知 SharedData 字节。重复或缺失的 ID 会导致整个请求失败。默认提取返回静图、视频及可选 XMP。LLP1 计入元数据、名称、MIME、ID、封装开销和每项被选输出的载荷。

## 使用入口

对合适的输入调用 `liblivephoto_interface::process`，请求为 `{"action":"extract","resourceIds":[6,2,4]}`。适配层需要 LLP1 字节时调用 `into_envelope`。

## 通过 verify_extraction.py 验证

接口测试比较完整结果，覆盖 JPEG 与 HEIF 默认 XMP、缺少 XMP、未知资源、后续无效 ID 及有序多区间字节。缩小限额的重放会在复制前拒绝放大的选择，允许恰好 4919 字节，并拒绝 4918 字节限额和只计载荷的限额。支持的结果与基线 `b1c556314b0939a6370f68fcd8be63089f7b3883` 比较。历史重放对三个先前的合计预算修复要求出现四次断言失败，再得到四次通过。

## 注意事项

严格的复制前保证适用于明确选择。默认 HEIF XMP 继续使用原有的有界物化。小限额重放只在临时克隆中使用公开说明的源码插入点；生产响应限额仍为包含边界值的 320 MiB，不构造大型 OOM 用例。
