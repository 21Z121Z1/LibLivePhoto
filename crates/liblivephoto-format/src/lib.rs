#![forbid(unsafe_code)]
//! Shared format primitives. This crate has no product or platform dependencies.
pub mod cursor;
pub mod error;
pub mod exif;
pub mod exif_raw;
pub mod fourcc;
pub mod isobmff;
pub mod jpeg_segments;

pub use cursor::{Cursor, Endian};
pub use error::{FormatError, Result};
pub use exif::Orientation;
pub use exif_raw::{
    exif_makernote, exif_user_comment, heif_exif_tiff, jpeg_exif_tiff, replace_exif_makernote,
};
pub use fourcc::FourCC;
pub use jpeg_segments::{jpeg_icc_profile, jpeg_image_end};
