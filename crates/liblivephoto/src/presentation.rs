//! Platform-independent, exact timing for display adapters. No pixel decoding.
use crate::{MediaTime, MotionPhoto, MotionPhotoReadError as Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameTime {
    /// Decode-order sample index. The containing list is in presentation order.
    pub sample_index: usize,
    pub time: MediaTime,
}

pub(crate) fn add_exact(a: MediaTime, b: MediaTime) -> Result<MediaTime, Error> {
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    let scale = a
        .timescale
        .checked_div(gcd(a.timescale, b.timescale))
        .and_then(|n| n.checked_mul(b.timescale))
        .filter(|s| *s != 0)
        .ok_or(Error::InvalidPresentation)?;
    let a = a.rescale_exact(scale).ok_or(Error::InvalidPresentation)?;
    let b = b.rescale_exact(scale).ok_or(Error::InvalidPresentation)?;
    Ok(MediaTime {
        value: a
            .value
            .checked_add(b.value)
            .ok_or(Error::InvalidPresentation)?,
        timescale: scale,
    })
}

pub fn video_frame_times(
    video: &[u8],
    track_id: u32,
    max_frames: usize,
) -> Result<Vec<FrameTime>, Error> {
    crate::tracks::video_frames(video, track_id, max_frames)
}
impl MotionPhoto<'_> {
    pub fn frame_times(&self, track_id: u32, max_frames: usize) -> Result<Vec<FrameTime>, Error> {
        video_frame_times(self.motion_video_bytes()?, track_id, max_frames)
    }
}
