# 厂商声明约定

[English](README.md)

13 个传输用例使用仓库已有的自有中性素材 `web/public/demo/motion.mp4` 和 `fixtures/bindings/vivo-stream-motion.jpg`。Samsung 外层结构依据 SEF 文档独立构造，微型 JPEG 外壳没有可解码图像。vivo 变体只改变合成标识符、明确 XMP 声明或结构标记，不包含原设备/私人媒体、APK、反编译源码或原生写入器。

`cases.json` 通过 SHA256 锁定每项素材，并断言所构造的事实、明确的解析拒绝及逐字节提取。这些是结构/保留约定，不是相册接收或色彩/编解码渲染器。公开原件覆盖仍由单独按 SHA 锁定的样本集提供。

重建到新目录，不覆盖已有内容：

```sh
python scripts/create_vendor_fixtures.py --out /tmp/new-vendor-fixtures
```

主机 C ABI、JNI、解包后的 release CLI/编译 C/JAR 消费程序运行全部 13 项。可移植 Rust 另外检查选择、别名、冲突、边界和预算。移动/WASM 原有套件保留自身数量和范围，不宣称这些新增 13 项已在其运行时执行。
