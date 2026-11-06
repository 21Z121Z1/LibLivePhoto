# 样本政策

[English](README.md)

清单引用用户公开 XDRemux 仓库中的不可变样本；该仓库依据其样本验收约定公布源字节。本仓库不再分发这些照片及其个人元数据。测试从调用方目录或本地缓存读取精确字节，下载的相机文件和缓存被排除在 Git 与 CI 上传之外。源文件公开不等于通用的第三方媒体许可，不应把这些文件放入库发布包。Rust 测试内的合成向量是原创测试数据，使用本仓库许可证。

`bindings/` 只包含原创合成集成输入。`camera-album.jpg` 使用既有自有封面与独立构造的尾部，包含中性 ID 和不透明科学计数法 JSON。SHA256：`78c67e88f5cc0fbe35ca8ffa0892a41467951aa6ec4cd5f5e663638c3b8367d3`。它测试 C/JNI 传输和保留，不是原设备或相册互操作证据。`cargo run -p liblivephoto --example binding_fixtures -- NEW_DIRECTORY` 通过 CREATE_NEW 重建这些绑定向量。

`vivo-stream-motion.jpg` 将相同自有 JPEG/H.264 演示与独立构造的 UUID 结构、重复不透明键和中性 CameraAlbum 尾部组合。SHA256：`78c40253278409fa8818c3f6f56a81a356aa52d3c711101cca2f08d9cb8cce36`。它检验真实 C/JNI 元数据传输和字节一致性，不是原生 vivo 写入器或相册。生成器可重建六项输入，先前五项逐字节不变。公开原始样本测试独立验证两份 vivo 文件及各自全部 31 个流载荷；这些原件不提交或打包。

获取并验证公开样本集：

```sh
python3 scripts/fixtures.py --fetch
LIBLIVEPHOTO_FIXTURE_ROOT="$PWD/fixtures/cache" cargo test -p liblivephoto --test fixtures -- --ignored
```
