# llp

Command-line interface for LibLivePhoto. Rust 1.88 or later.

After this version is published to crates.io:

```sh
cargo install --locked liblivephoto-cli --version 0.1.0
llp version
llp parse input.jpg
llp extract input.jpg --video --out extracted
llp check input.jpg
```

Output directories must not already exist. `llp --help` lists build/conversion,
metadata and range options. Exit status 0 means success, 1 means the operation
reported failure, and 2 means an argument or I/O error. JSON output and error codes
are for automation; human-readable diagnostic wording may change.

Combined input is limited to 128 MiB. No transcoding runs. The 0.1.x CLI options
and existing JSON fields retain their meaning across patch releases. Consumers
must tolerate additional JSON fields and diagnostics.

[CLI reference](https://github.com/21Z121Z1/LibLivePhoto/blob/main/CLI.md).
Apache-2.0.
