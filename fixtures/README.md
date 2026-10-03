# Fixture policy

The manifest references immutable fixtures in the user's public XDRemux repository.
That repository publishes the source bytes under its fixture acceptance contract.
This repository does not redistribute those photographs or their personal metadata.
Tests use exact bytes from a supplied directory or from a local cache.
Generated files and the cache are excluded from Git and CI uploads.
The source publication does not establish a general third-party media license.
Do not redistribute these files as library package contents.
Synthetic vectors in Rust tests are original test data under this repository's license.

Fetch and verify the public corpus:

```sh
python3 scripts/fixtures.py --fetch
LIBLIVEPHOTO_FIXTURE_ROOT="$PWD/fixtures/cache" cargo test -p liblivephoto --test fixtures -- --ignored
```
