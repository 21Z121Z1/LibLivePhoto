# Vendor declaration contracts

Thirteen transport cases use the repository's existing owned neutral
`web/public/demo/motion.mp4` and `fixtures/bindings/vivo-stream-motion.jpg`.
Samsung wrappers are independently authored from the documented SEF framing;
the tiny JPEG envelope has no decodable picture. vivo mutations change only a
synthetic identifier, an explicit XMP declaration, or a framing marker. No
original-device/private media, APK, decompiled source or native writer is included.

`cases.json` locks every fixture with SHA256 and asserts authored facts, explicit
parse rejection and byte-exact extraction. These are structural/preservation
contracts, not Gallery acceptance or a colour/codec renderer. Public original
coverage remains in the separate SHA-locked fixture corpus.

To reproduce without overwriting anything:

```sh
python scripts/create_vendor_fixtures.py --out /tmp/new-vendor-fixtures
```

Host C ABI, JNI and the extracted release CLI/compiled C/JAR consumers run all13
cases. Portable Rust adds selection, alias, conflict, bounds and budget tests.
Mobile/WASM existing suites retain their own count and scope; these13 new cases
are not claimed to execute on those runtimes.
