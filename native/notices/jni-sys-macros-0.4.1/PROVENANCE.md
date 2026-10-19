The registry package `jni-sys-macros 0.4.1` declares MIT OR Apache-2.0 but omits
the workspace license files. Its included `.cargo_vcs_info.json` pins upstream
`jni-rs/jni-sys` commit `64d77b7a5f119d7b55b4e2c169a4668067ff59e6`, path
`jni-sys-macros`. The two unmodified licenses here were read from that exact
commit's root, not from a later default branch:

- https://github.com/jni-rs/jni-sys/blob/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-APACHE
- https://github.com/jni-rs/jni-sys/blob/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-MIT

These are retained as this dependency's own license options. No blanket
Apache-2.0 designation is applied to other dependencies.
