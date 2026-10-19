//! Bounded JPEG trailer contracts from the pinned Google Honor provider.
//! Original Honor device acceptance and a redistributable device corpus are absent.
use crate::{ByteRange, MediaTime, MotionPhotoReadError as Error};
use liblivephoto_format::jpeg_image_end;

type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HonorVariant {
    LiveFooter,
    BackwardTlv,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HonorRecord {
    pub tag: String,
    pub payload: ByteRange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HonorMetadata {
    pub variant: HonorVariant,
    pub jpeg_end: u64,
    pub video: Option<ByteRange>,
    pub presentation: Option<MediaTime>,
    pub scene_mode: Option<i32>,
    /// Reverse file order, as stored in the backward chain.
    pub records: Vec<HonorRecord>,
}

fn invalid() -> Error {
    Error::UnsupportedLayout("malformed or unsupported Honor JPEG trailer")
}
fn tag(raw: &[u8]) -> Option<&str> {
    std::str::from_utf8(raw)
        .ok()
        .map(|s| s.trim_matches(['\0', ' ', '\r', '\n', '\t']))
}
fn tlv_tag(s: &str) -> bool {
    s.starts_with("HiHonor_") || s.starts_with("LivePhoto_")
}
fn movie(data: &[u8], start: usize, end: usize) -> bool {
    start < end
        && crate::scanner::is_ftyp_box_start(data, start as u64, end as u64).unwrap_or(false)
        && crate::normalize_embedded_video(&data[start..end])
            .is_ok_and(|v| v.removed_vendor_bytes == 0)
}

/// Inspect only explicitly tagged JPEG trailers. A metadata-only TLV chain is
/// returned with `video: None`; it is not thereby a Motion Photo.
pub fn inspect_honor_metadata(data: &[u8]) -> Result<Option<HonorMetadata>> {
    if !data.starts_with(&[255, 216]) || data.len() < 24 {
        return Ok(None);
    }
    let Some(tail) = tag(&data[data.len() - 20..]) else {
        return Ok(None);
    };
    if !tail.starts_with("LIVE_") && !tlv_tag(tail) {
        return Ok(None);
    }
    let jpeg_end = jpeg_image_end(data, 0).map_err(|_| invalid())?;
    let mut result = HonorMetadata {
        variant: HonorVariant::LiveFooter,
        jpeg_end: jpeg_end as u64,
        video: None,
        presentation: None,
        scene_mode: None,
        records: Vec::new(),
    };
    if let Some(size) = tail.strip_prefix("LIVE_") {
        if size.is_empty() || !size.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let size = size.parse::<usize>().map_err(|_| invalid())?;
        if size == 0 {
            return Err(invalid());
        }
        let end = data.len() - 20;
        let normal = end
            .checked_sub(size)
            .filter(|s| *s >= jpeg_end && movie(data, *s, end));
        let extended = end
            .checked_sub(20)
            .and_then(|e| e.checked_sub(size).map(|s| (s, e)))
            .filter(|(s, e)| *s >= jpeg_end && movie(data, *s, *e));
        if normal.is_some() && extended.is_some() {
            return Err(Error::AmbiguousRecovery);
        }
        let (start, end) = if let Some((start, end)) = extended {
            let cover = tag(&data[end..end + 20]).ok_or_else(invalid)?;
            let ms = cover.parse::<i64>().map_err(|_| invalid())?;
            if ms < 0 {
                return Err(invalid());
            }
            result.presentation = MediaTime::new(
                ms.checked_mul(1000).ok_or(Error::InvalidPresentation)?,
                1_000_000,
            );
            (start, end)
        } else {
            (normal.ok_or_else(invalid)?, end)
        };
        result.video = Some(ByteRange::new(start as u64, end as u64)?);
        return Ok(Some(result));
    }
    result.variant = HonorVariant::BackwardTlv;
    let mut pos = data.len();
    while pos >= jpeg_end + 24 {
        let Some(name) = tag(&data[pos - 20..pos]).filter(|s| tlv_tag(s)) else {
            break;
        };
        if result.records.len() >= 128 {
            return Err(invalid());
        }
        let len = u32::from_le_bytes(data[pos - 24..pos - 20].try_into().map_err(|_| invalid())?)
            as usize;
        let end = pos - 24;
        let start = end
            .checked_sub(len)
            .filter(|s| *s >= jpeg_end)
            .ok_or_else(invalid)?;
        let range = ByteRange::new(start as u64, end as u64)?;
        if movie(data, start, end) && result.video.replace(range).is_some() {
            return Err(Error::AmbiguousRecovery);
        }
        if name == "HiHonor_OfflineData" && (4..=65_536).contains(&len) {
            let declared =
                u32::from_le_bytes(data[start..start + 4].try_into().map_err(|_| invalid())?)
                    as usize;
            if declared != len - 4 {
                return Err(invalid());
            }
            let value: serde_json::Value =
                serde_json::from_slice(&data[start + 4..end]).map_err(|_| invalid())?;
            if let Some(mode) = value
                .pointer("/common/sceneMode")
                .or_else(|| value.get("sceneMode"))
            {
                let mode = mode
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .ok_or_else(invalid)?;
                if result
                    .scene_mode
                    .replace(mode)
                    .is_some_and(|old| old != mode)
                {
                    return Err(invalid());
                }
            }
        }
        result.records.push(HonorRecord {
            tag: name.into(),
            payload: range,
        });
        pos = start;
    }
    if result.records.is_empty() {
        return Err(invalid());
    }
    if pos > jpeg_end
        && movie(data, jpeg_end, pos)
        && result
            .video
            .replace(ByteRange::new(jpeg_end as u64, pos as u64)?)
            .is_some()
    {
        return Err(Error::AmbiguousRecovery);
    }
    Ok(Some(result))
}
