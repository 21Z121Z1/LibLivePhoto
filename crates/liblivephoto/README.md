# liblivephoto

[简体中文](README.zh-CN.md)

Rust library for inspecting, extracting and converting Apple Live Photos,
Android Motion Photos and supported Samsung SEF layouts. Rust 1.88 or later.
The library preserves encoded media; it does not transcode images or video.

```no_run
use liblivephoto::{Input, MotionPhoto, ParseOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("input.jpg")?;
    let photo = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::compatible())?;
    photo.validate()?;
    let mut video = std::fs::File::create_new("video.mp4")?;
    photo.extract(photo.motion_video(), &mut video)?;
    Ok(())
}
```

Apple pairs use `Input::ApplePair { still, movie }`. Exact photo time uses
`MediaTime`, an integer value and timescale. Conversion uses `plan` followed by
`execute`; unsupported layouts, inexact target times and unmapped resources are
explicit errors. Inspect the plan before executing it. `pvt::package_pair`
assembles an in-memory `.pvt` directory from a validated JPEG or HEIC pair and
does not transcode or claim Photos import.

The default Rust input budget is 256 MiB. The separate CLI/shared interface uses
128 MiB. Parsing is not streaming. Native playback, editing and Gallery acceptance
depend on the host, format and device.

The 0.1.x Rust API preserves source compatibility across patch releases. Breaking
changes use a new minor version. The opt-in `xdremux-compat` feature and platform
bindings remain experimental.

The package includes synthetic data for `cargo test --all-targets --all-features`.
Ignored original-corpus tests require separately obtained files; private photos are
not distributed. See the [project documentation](https://github.com/21Z121Z1/LibLivePhoto)
for format limits, conversion policies and platform SDKs. Apache-2.0.
