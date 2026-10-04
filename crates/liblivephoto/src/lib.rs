#![forbid(unsafe_code)]
//! Parse, inspect, validate and remux related still and motion resources.
mod android;
pub mod apple;
mod asset;
mod conversion;
mod error;
mod facade;
mod heif;
mod heif_io;
mod jpeg_write;
mod live_photo_mov;
mod live_photo_still;
#[path = "vendor/oplus/lpex.rs"]
mod lpex;
mod model;
mod motion_video;
#[path = "vendor/oplus/parser.rs"]
mod oppo;
mod scanner;
mod timeline;
#[path = "vendor/oplus/topology.rs"]
mod topology;
mod tracks;
mod vendor;
pub use conversion::*;

pub use asset::*;
pub use error::{MotionPhotoError, Result};
pub use facade::{Input, MotionPhoto, MotionPhotoReadError, ParseMode, ParseOptions, PathInput};
pub use live_photo_mov::{LivePhotoMovieError, LivePhotoMovieResult};
pub use live_photo_still::{LivePhotoStillError, LivePhotoStillResult};
pub use model::ByteRange;

pub(crate) use android::parse_android_motion_photo;
pub(crate) use live_photo_mov::{
    read_live_photo_content_identifier, read_live_photo_still_presentation,
    validate_live_photo_movie,
};
pub(crate) use live_photo_still::read_apple_content_identifier;
pub(crate) use model::{MotionPhotoAsset, MotionPhotoSourceKind, PresentationSource};
pub(crate) use motion_video::normalize_embedded_video;
pub(crate) use oppo::parse_oppo_motion_photo;
pub(crate) use scanner::is_ftyp_box_start;
pub(crate) use topology::resolve_video_stream_layout;

/// Migration interfaces for XDRemux. No stability guarantee applies to this module.
#[cfg(feature = "xdremux-compat")]
#[doc(hidden)]
pub mod compat {
    pub use crate::android::parse_android_motion_photo;
    pub use crate::heif::{is_heif_mime, resolve_heif_motion_photo_ranges};
    pub use crate::live_photo_mov::*;
    pub use crate::live_photo_still::*;
    pub use crate::lpex::parse_first_lpex_object;
    pub use crate::model::*;
    pub use crate::motion_video::*;
    pub use crate::oppo::{enrich_oppo_asset, parse_oppo_fallback, parse_oppo_motion_photo};
    pub use crate::scanner::{ftyp_box_offsets, is_ftyp_box_start};
    pub use crate::topology::{enrich_oppo_video_range, resolve_video_stream_layout};
}
