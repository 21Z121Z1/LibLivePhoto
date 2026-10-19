# Fixture policy

The manifest references immutable fixtures in the user's public XDRemux repository.
That repository publishes the source bytes under its fixture acceptance contract.
This repository does not redistribute those photographs or their personal metadata.
Tests use exact bytes from a supplied directory or from a local cache.
Downloaded camera files and the cache are excluded from Git and CI uploads.
The source publication does not establish a general third-party media license.
Do not redistribute these files as library package contents.
Synthetic vectors in Rust tests are original test data under this repository's license.

`bindings/` contains only original synthetic integration inputs. The new
`camera-album.jpg` is the existing owned test cover plus an independently
constructed footer, with a neutral ID and opaque scientific-notation JSON.
SHA256:`78c67e88f5cc0fbe35ca8ffa0892a41467951aa6ec4cd5f5e663638c3b8367d3`.
It tests C/JNI transport and preservation; it is not original-device or Gallery
interoperability evidence. `cargo run -p liblivephoto --example binding_fixtures
-- NEW_DIRECTORY` reproduces the binding vectors using CREATE_NEW.

Fetch and verify the public corpus:

```sh
python3 scripts/fixtures.py --fetch
LIBLIVEPHOTO_FIXTURE_ROOT="$PWD/fixtures/cache" cargo test -p liblivephoto --test fixtures -- --ignored
```
