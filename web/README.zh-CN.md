# 本地实况照片实验页

[English](README.md)

使用真实 Rust/WASM 解析、资源提取和精确样本时间。不上传，不使用远程 SDK、遥测、转码或部署；服务器只监听回环地址。

需要 stable Rust、Node 24（预期也支持 Node 22+）和精确版本 `wasm-bindgen-cli 0.2.104`。在仓库根目录运行：

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.104 --locked
npm ci --prefix web --ignore-scripts
npm --prefix web run build
npm --prefix web run dev
```

打开 `http://127.0.0.1:4173`，选择文件或内置合成样本。普通静图/视频组合必须勾选明确关联选项。开始、替换或取消输入会清除先前媒体和 object URL。迟到的读取、fetch 或 seek 不会替换当前结果。

测试：

```sh
npm --prefix web test
cd web
npx playwright install chromium
npm run test:browser
```

使用系统 Chromium 时，可设置 `BROWSER_EXECUTABLE=/usr/bin/chromium`。Playwright 结果和截图位于被 Git 忽略的 `web/test-results`。Web 构建成功不表示执行或验证过 Swift 适配层。Apple SDK 检查位于 `.github/workflows/apple-web.yml`，需单独查看实际 CI 结果。

`public/demo` 是项目自行生成、按 CC0-1.0 发布的几何图像，精确样本/帧来源与哈希记录在 `provenance.json`。默认样本不含私人相机素材或人物。重建使用 Pillow 和 ffmpeg，在 Cargo 位于 PATH 时从仓库根目录运行 `python3 scripts/generate_web_demo.py`；这些是生成工具，不是前端运行依赖。打开用户文件不会改写已有原件。

HEIC/HEVC 支持取决于浏览器和系统编解码器。页面会报告失败，并提供明确标记的视频帧或用户提供的 JPEG/PNG 封面，不把元数据保留等同于 HDR 渲染或原生照片支持。`src/timing.js` 的可选 LivePhotosKit 绑定要求调用方提供 Player；它为 HTMLVideoElement 明确传递 photoTime/frameTimes，并接受原始 MOV metadataVideoSrc。不捆绑或验证 Apple Player 运行时。

`public/acceptance` 包含额外的原创 CC0 测试媒体：同一编码 H.264 场景加双声道 440 Hz AAC、真实 x265 HEVC/HEIC，以及测试浏览器无法解码的 JPEG2000 MOV。生成器拒绝覆盖已有路径。七个浏览器测试覆盖合成器 `requestVideoFrameCallback` 在精确照片帧上的 mediaTime、真实 AAC 解码、真实静图/视频编解码错误、原始字节下载、URL 释放与取消/替换竞争。无头音频解码不证明真实扬声器有可听输出。

详见[架构与能力边界](../APPLE_API_ARCHITECTURE.zh-CN.md)。
