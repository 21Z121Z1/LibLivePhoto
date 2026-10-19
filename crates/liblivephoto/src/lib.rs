#![forbid(unsafe_code)]
//! Parse, inspect, validate and remux related still and motion resources.
mod agtm;
mod agtm_decode;
mod android;
pub mod apple;
mod asset;
mod camera_album;
pub use camera_album::*;
mod conformance;
mod conversion;
mod error;
mod facade;
mod file_input;
mod google_video_conformance;
mod heif;
mod heif_io;
mod icc;
pub use google_video_conformance::*;
pub use icc::*;
mod jpeg_write;
mod live_photo_mov;
mod live_photo_still;
#[path = "vendor/oplus/lpex.rs"]
mod lpex;
mod media;
mod model;
mod providers;
mod xmp;
pub use providers::*;
pub use xmp::*;
mod motion_video;
#[path = "vendor/oplus/olive.rs"]
mod olive;
#[path = "vendor/oplus/parser.rs"]
mod oppo;
pub use olive::{inspect_oplus_metadata, inspect_oplus_xmp_metadata, OplusMetadata, OplusRecord};
mod metadata_engine;
mod metadata_samples;
pub use metadata_engine::*;
mod presentation;
pub mod remux;
mod scanner;
mod scoring_time;
pub use scoring_time::*;
mod timeline;
mod video_metadata;
mod vivo_media;
pub use metadata_samples::*;
pub use video_metadata::*;
pub use vivo_media::*;
#[path = "vendor/oplus/topology.rs"]
mod topology;
mod tracks;
mod vendor;
pub use conformance::*;
pub use conversion::*;

pub use agtm::{decode_agtm_header, extract_agtm_from_video, AgtmHeader, AgtmSample};
pub use agtm_decode::{
    decode_agtm, AgtmComponentMix, AgtmControlPoint, AgtmMetadata, AgtmToneMappingRule,
};
pub use asset::*;
pub use error::{MotionPhotoError, Result};
pub use facade::{Input, MotionPhoto, MotionPhotoReadError, ParseMode, ParseOptions, PathInput};
pub use file_input::{FileInput, FileRange};
pub use live_photo_mov::{LivePhotoMovieError, LivePhotoMovieResult};
pub use live_photo_still::{LivePhotoStillError, LivePhotoStillResult};
pub use media::*;
pub use model::ByteRange;
pub use presentation::*;
pub use vendor::honor::{inspect_honor_metadata, HonorMetadata, HonorRecord, HonorVariant};
pub use vendor::samsung::{inspect_samsung_metadata, Record as SamsungRecord, SamsungMetadata};

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

/// Version of the public Rust core, independent of the reference implementation.
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Identify the physical MP4/QuickTime major brand. This does not validate
/// tracks, codecs, pairing or native-player acceptance.
pub fn movie_container_mime(data: &[u8]) -> Result<&'static str> {
    motion_video::container_mime(data)
}
