# Dependencies added by the Apple/local-web stage

No native Google/libheif/codec dependency is bundled by this stage. Existing
license analysis in GOOGLE_CORE_RUN.md remains applicable to the reference build.

| Component | Role | License verified from source/package |
| --- | --- | --- |
| wasm-bindgen 0.2.104 and matching macro/backend/shared crates | Rust → WASM binding and build tool | MIT OR Apache-2.0; checked crate LICENSE-MIT/LICENSE-APACHE and Cargo metadata |
| New Rust transitive crates (bumpalo, cfg-if, log, once_cell, rustversion, syn 2) | compile/runtime support | individually recorded in apple-web-dependencies.json from locked downloaded Cargo package metadata; not a blanket Apache claim |
| @playwright/test, playwright, playwright-core 1.56.1 | browser test tooling only | Apache-2.0; package LICENSE and lock metadata checked |
| fsevents 2.3.2, optional macOS Playwright dependency | test tooling only | MIT per locked package metadata; not installed on Linux |
| Apple system frameworks | native OS dependency, not redistributed | Apple platform/SDK terms, not this repository's Apache-2.0 |
| Chromium | browser test runtime, not bundled | its own multi-component licenses; not relicensed as Apache-2.0 |
| Pillow / ffmpeg | optional synthetic-demo generation tools | their own licenses (Pillow HPND; FFmpeg configured build can be LGPL/GPL and codec terms); not bundled in frontend or reclassified |
| `web/public/demo` images/video | project-generated geometry, no private media | CC0-1.0; provenance.json records hashes and generation inputs |

The web production build copies only project JS/CSS/HTML, generated WASM binding,
and synthetic assets. Node/Playwright/Swift SDK binaries are not copied to it.

The native interface stage adds pinned `jni 0.21.1`, `jni-sys`, `cesu8`, `combine`,
`bytes`, `thiserror` and platform support crates. They are not assumed to have a
single license; native-dependencies.json records each locked downloaded package's
own Cargo license/source/checksum. The C ABI and Java protocol codec are project
code; no Google Java/protobuf/JNI implementation was copied. OpenJDK is an external
build/test runtime under its own GPL + Classpath Exception terms, not bundled or
relicensed by this library. Android/Apple SDK terms remain separate.
