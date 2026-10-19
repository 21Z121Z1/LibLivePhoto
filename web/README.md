# Local Live Photo lab

Real Rust/WASM parsing, resource extraction and exact sample timing. No uploads,
remote SDK, telemetry, transcode or deployment. The server is loopback-only.

Prerequisites: stable Rust, Node 24 (Node 22+ also expected), and the exact
`wasm-bindgen-cli 0.2.104`. In the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.104 --locked
npm ci --prefix web --ignore-scripts
npm --prefix web run build
npm --prefix web run dev
```

Open `http://127.0.0.1:4173`. Choose your file(s) or the included synthetic sample.
Ordinary still/video composition requires the explicit association checkbox.
A failed operation leaves the prior preview visible; status describes the failure.

Tests:

```sh
npm --prefix web test
cd web
npx playwright install chromium
npm run test:browser
```

On a machine with system Chromium, use `BROWSER_EXECUTABLE=/usr/bin/chromium`.
Playwright results/screenshots are under `web/test-results` (ignored). A successful
web build does not run or validate the Swift adapter. Apple SDK checks are in
`.github/workflows/apple-web.yml`; actual CI results must be read separately.

`public/demo` is original project-generated geometric artwork released under
CC0-1.0, with precise sample/frame provenance and hashes in `provenance.json`.
The default has no private camera media or people. Regeneration uses Pillow and
ffmpeg (`python3 scripts/generate_web_demo.py` from the repo root with Cargo on
PATH). These are tools, not runtime frontend dependencies. Existing originals
are never rewritten when opening user files.

HEIC/HEVC support depends on your browser and system codecs. The page reports
failure and offers a labelled video-frame or user-provided JPEG/PNG cover. It
does not turn metadata preservation into HDR rendering or native Photos support.
The optional LivePhotosKit binding in `src/timing.js` requires a caller-provided
Player; it passes photoTime/frameTimes explicitly for HTMLVideoElement and accepts
original MOV metadataVideoSrc. Apple Player runtime is not bundled or verified.

See [architecture and capability boundaries](../APPLE_API_ARCHITECTURE.md).
