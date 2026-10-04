//! Portable Apple on-disk metadata operations. No Apple framework is used.
use crate::{LivePhotoMovieResult, LivePhotoStillResult, MediaTime};

/// Caller-supplied on-disk Apple presentation metadata. The library does not
/// derive vendor geometry or select a capture stabilization policy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GeometryMetadata {
    pub transform: Option<[f64; 9]>,
    pub dimensions: Option<(f32, f32)>,
}

pub fn read_still_identifier(still: &[u8]) -> LivePhotoStillResult<Option<String>> {
    crate::live_photo_still::read_apple_content_identifier(still)
}
pub fn read_movie_identifier(movie: &[u8]) -> LivePhotoMovieResult<Option<String>> {
    crate::live_photo_mov::read_live_photo_content_identifier(movie)
}
pub fn read_presentation(movie: &[u8]) -> LivePhotoMovieResult<Option<MediaTime>> {
    crate::live_photo_mov::read_live_photo_still_presentation(movie)
}
/// Preserve media chunks and write pairing metadata. Rescale the movie timeline
/// when an exact timestamp requires a finer tick. Reject unrepresentable fields.
pub fn remux_movie(
    movie: &[u8],
    identifier: &str,
    presentation: MediaTime,
) -> LivePhotoMovieResult<Vec<u8>> {
    crate::live_photo_mov::write_movie_exact(movie, identifier, presentation, None)
}
pub fn remux_movie_with_geometry(
    movie: &[u8],
    identifier: &str,
    presentation: MediaTime,
    geometry: GeometryMetadata,
) -> LivePhotoMovieResult<Vec<u8>> {
    if geometry
        .transform
        .is_some_and(|m| m.iter().any(|x| !x.is_finite()))
        || geometry
            .dimensions
            .is_some_and(|(w, h)| !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0)
    {
        return Err(crate::live_photo_mov::LivePhotoMovieError::invalid(
            "invalid caller presentation geometry",
        ));
    }
    crate::live_photo_mov::write_movie_exact(movie, identifier, presentation, Some(geometry))
}
/// Return encoded media payloads for byte comparison. Metadata marker payloads are excluded.
pub fn media_payloads(movie: &[u8]) -> LivePhotoMovieResult<Vec<Vec<u8>>> {
    crate::live_photo_mov::media_mdat_payloads(movie)
}
