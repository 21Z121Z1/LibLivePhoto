//! Timed AGTM payload extraction. This module does not render pixels or infer a
//! baseline when a sample is missing or truncated.
use crate::{MediaTime, MotionPhoto, MotionPhotoReadError as Error, TrackKind};
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub struct AgtmHeader {
    pub application_version: u8,
    pub minimum_application_version: u8,
    pub hdr_reference_white_nits: f32,
    pub has_adaptive_tone_map: bool,
    pub baseline_hdr_headroom_log2: Option<f32>,
    pub use_reference_white_tone_mapping: Option<bool>,
    /// Offset where decode_agtm continues into the complete curve/mix fields.
    pub remaining_bit_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgtmSample<'a> {
    pub track_id: u32,
    /// Zero-based sample index, unlike Google's one-based mp4v2 sample ID.
    pub sample_index: usize,
    pub presentation: MediaTime,
    pub duration: MediaTime,
    pub bytes: &'a [u8],
    pub header: AgtmHeader,
}

pub(crate) fn raw_payload(data: &[u8]) -> &[u8] {
    let data = if data.starts_with(&[5, 0xb5, 0, 0x90]) {
        &data[1..]
    } else {
        data
    };
    if data.starts_with(&[0xb5, 0, 0x90]) {
        let data = &data[3..];
        if data.starts_with(&[0, 1]) || data.starts_with(&[0, 5]) {
            &data[2..]
        } else {
            data
        }
    } else {
        data
    }
}

/// Decode the version-0 baseline fields of a raw ST 2094-50 payload or an exact
/// T.35 prefix. Unknown curve/mix fields are left opaque. No substring search.
pub fn decode_agtm_header(data: &[u8]) -> Result<AgtmHeader> {
    let data = raw_payload(data);
    let header = data
        .get(..2)
        .ok_or(Error::UnsupportedLayout("truncated AGTM header"))?;
    let application_version = header[0] >> 5;
    let minimum_application_version = (header[0] >> 2) & 7;
    if application_version != 0 || minimum_application_version != 0 {
        return Err(Error::UnsupportedLayout(
            "unsupported AGTM application version",
        ));
    }
    let mut offset = 2;
    let mut word = || -> Result<u16> {
        let bytes = data
            .get(offset..offset + 2)
            .ok_or(Error::UnsupportedLayout("truncated AGTM baseline fields"))?;
        offset += 2;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    };
    let hdr_reference_white_nits = if header[1] & 0x80 != 0 {
        let value = word()?;
        if !(1..=50_000).contains(&value) {
            return Err(Error::UnsupportedLayout(
                "AGTM reference white is outside its supported range",
            ));
        }
        f32::from(value) / 5.0
    } else {
        203.0
    };
    let adaptive = header[1] & 0x40 != 0;
    let baseline = if adaptive { Some(word()?) } else { None };
    if baseline.is_some_and(|v| v > 60_000) {
        return Err(Error::UnsupportedLayout(
            "AGTM headroom is outside its supported range",
        ));
    }
    let use_reference_white_tone_mapping = if adaptive {
        Some(
            *data
                .get(offset)
                .ok_or(Error::UnsupportedLayout("truncated AGTM tone mapping flag"))?
                & 0x80
                != 0,
        )
    } else {
        None
    };
    Ok(AgtmHeader {
        application_version,
        minimum_application_version,
        hdr_reference_white_nits,
        has_adaptive_tone_map: adaptive,
        baseline_hdr_headroom_log2: baseline.map(|v| f32::from(v) / 10_000.0),
        use_reference_white_tone_mapping,
        remaining_bit_offset: offset * 8 + usize::from(adaptive),
    })
}

fn explicit_t35(data: &[u8]) -> bool {
    data.starts_with(&[0xb5, 0, 0x90]) || data.starts_with(&[5, 0xb5, 0, 0x90])
}

impl MotionPhoto<'_> {
    /// Select AGTM metadata samples by their exact presentation intervals. A
    /// missing track/sample returns an empty vector. Edit lists are unsupported;
    /// nonintegral timestamp conversion fails explicitly. No first-sample or
    /// arbitrary whole-file scanner fallback is used.
    pub fn agtm_at(&self, time: MediaTime) -> Result<Vec<AgtmSample<'_>>> {
        let video = self.motion_video_bytes()?;
        extract_agtm_from_video(video, time)
    }
}

impl AgtmSample<'_> {
    pub fn metadata(&self) -> Result<crate::AgtmMetadata> {
        crate::decode_agtm(self.bytes)
    }
}

/// Extract from a standalone MP4/MOV, with the same preservation and exact-time
/// contract as MotionPhoto::agtm_at. No still-image association is required.
/// Across tracks, selected samples are limited to 4096, referenced payload to
/// 16 MiB, and decoded rules/control points to 65536. Overlapping samples and
/// repeated references each consume the budget before a result is returned.
pub fn extract_agtm_from_video(video: &[u8], time: MediaTime) -> Result<Vec<AgtmSample<'_>>> {
    let container = crate::tracks::inspect(video, crate::ResourceId(1), false)?;
    let mut result = Vec::new();
    let (mut payload_bytes, mut records) = (0usize, 0usize);
    for track in &container.tracks {
        if matches!(track.kind, TrackKind::Video | TrackKind::Audio)
            || !track
                .sample_entries
                .iter()
                .any(|e| matches!(e, b"it35" | b"mett" | b"mp4s"))
        {
            continue;
        }
        for sample in crate::tracks::metadata_at(video, track, container.movie_timescale, time)? {
            let signaled = sample.description == *b"it35"
                && sample.description_bytes.get(8..).is_some_and(explicit_t35);
            if !signaled && !explicit_t35(sample.bytes) {
                continue;
            }
            payload_bytes = payload_bytes
                .checked_add(sample.bytes.len())
                .filter(|n| *n <= 16 * 1024 * 1024)
                .ok_or(Error::InputLimit)?;
            if result.len() >= 4096 {
                return Err(Error::InputLimit);
            }
            let metadata = crate::decode_agtm(sample.bytes)?;
            records = records
                .checked_add(
                    metadata
                        .rules
                        .iter()
                        .map(|r| r.curve.len() + 1)
                        .sum::<usize>(),
                )
                .filter(|n| *n <= 65_536)
                .ok_or(Error::InputLimit)?;
            result.push(AgtmSample {
                track_id: track.id,
                sample_index: sample.index,
                presentation: sample.presentation,
                duration: MediaTime {
                    value: i64::from(sample.duration),
                    timescale: track.timescale,
                },
                bytes: sample.bytes,
                header: metadata.header,
            });
        }
    }
    Ok(result)
}
