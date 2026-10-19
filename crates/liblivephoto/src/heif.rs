use crate::error::{MotionPhotoError, Result};
use crate::model::{ByteRange, MotionPhotoItem};
use crate::scanner::is_ftyp_box_start;

const MAX_TOP_LEVEL_BOXES: usize = 4096;

#[derive(Debug, Clone, Copy)]
struct TopLevelBox {
    offset: u64,
    end_offset: u64,
    header_size: u64,
    kind: [u8; 4],
}

impl TopLevelBox {
    fn payload_offset(self) -> Result<u64> {
        self.offset
            .checked_add(self.header_size)
            .ok_or(MotionPhotoError::ArithmeticOverflow)
    }
}

fn top_level_boxes(data: &[u8]) -> Result<Vec<TopLevelBox>> {
    let parsed = liblivephoto_format::isobmff::parse_boxes(data, 0..data.len())
        .map_err(|_| MotionPhotoError::InvalidVideoPayload)?;
    if parsed.len() > MAX_TOP_LEVEL_BOXES {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    Ok(parsed
        .into_iter()
        .map(|b| TopLevelBox {
            offset: b.box_start as u64,
            end_offset: b.data_end as u64,
            header_size: (b.data_start - b.box_start) as u64,
            kind: *b.kind.as_bytes(),
        })
        .collect())
}

pub fn is_heif_mime(mime: &str) -> bool {
    mime.eq_ignore_ascii_case("image/heic")
        || mime.eq_ignore_ascii_case("image/heif")
        || mime.eq_ignore_ascii_case("image/avif")
}

pub fn resolve_heif_motion_photo_ranges(
    data: &[u8],
    items: &[MotionPhotoItem],
) -> Result<(ByteRange, ByteRange)> {
    let primary = items.first().ok_or(MotionPhotoError::InvalidDirectory)?;
    let motion = items.last().ok_or(MotionPhotoError::InvalidDirectory)?;
    if !is_heif_mime(&primary.mime) || !motion.semantic.eq_ignore_ascii_case("MotionPhoto") {
        return Err(MotionPhotoError::InvalidDirectory);
    }
    if !matches!(primary.padding, 8 | 16) {
        return Err(MotionPhotoError::InvalidItemLength);
    }

    let boxes = top_level_boxes(data)?;
    if boxes.first().map(|value| value.kind) != Some(*b"ftyp") {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    let mpvd: Vec<_> = boxes
        .iter()
        .filter(|value| value.kind == *b"mpvd")
        .collect();
    if mpvd.len() != 1 {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    let mpvd = *mpvd[0];
    if primary.padding != mpvd.header_size {
        return Err(MotionPhotoError::InvalidItemLength);
    }
    let payload_start = mpvd.payload_offset()?;
    if payload_start >= mpvd.end_offset {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    let file_size = u64::try_from(data.len()).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    let declared_start = file_size
        .checked_sub(motion.length)
        .ok_or(MotionPhotoError::InvalidByteRange)?;
    if declared_start != payload_start {
        return Err(MotionPhotoError::InvalidByteRange);
    }
    if !is_ftyp_box_start(data, payload_start, mpvd.end_offset)? {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    Ok((
        ByteRange::new(0, mpvd.offset)?,
        ByteRange::new(payload_start, mpvd.end_offset)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(payload.len() + 8).unwrap();
        let mut output = size.to_be_bytes().to_vec();
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output
    }

    #[test]
    fn extracts_mpvd_payload_without_trailing_vendor_box() {
        let video = make_box(b"ftyp", b"isom\0\0\0\0");
        let ftyp = make_box(b"ftyp", b"heic\0\0\0\0");
        let mpvd = make_box(b"mpvd", &video);
        let sefd = make_box(b"sefd", &[1, 2, 3, 4]);
        let mut data = ftyp.clone();
        let mpvd_offset = data.len() as u64;
        data.extend_from_slice(&mpvd);
        data.extend_from_slice(&sefd);
        let payload_start = mpvd_offset + 8;
        let motion_length = data.len() as u64 - payload_start;
        let items = vec![
            MotionPhotoItem {
                mime: "image/heic".into(),
                semantic: "Primary".into(),
                length: 0,
                padding: 8,
            },
            MotionPhotoItem {
                mime: "video/mp4".into(),
                semantic: "MotionPhoto".into(),
                length: motion_length,
                padding: 0,
            },
        ];
        let (still, extracted) = resolve_heif_motion_photo_ranges(&data, &items).unwrap();
        assert_eq!(still, ByteRange::new(0, mpvd_offset).unwrap());
        assert_eq!(
            extracted,
            ByteRange::new(payload_start, payload_start + video.len() as u64).unwrap()
        );
    }

    #[test]
    fn extended_mpvd_requires_matching_directory_padding() {
        let video = make_box(b"ftyp", b"isom\0\0\0\0");
        let still = make_box(b"ftyp", b"avif\0\0\0\0");
        let header = [
            1u32.to_be_bytes().to_vec(),
            b"mpvd".to_vec(),
            ((video.len() + 16) as u64).to_be_bytes().to_vec(),
        ]
        .concat();
        let data = [still.clone(), header, video.clone()].concat();
        let mut items = vec![
            MotionPhotoItem {
                mime: "image/avif".into(),
                semantic: "Primary".into(),
                length: 0,
                padding: 16,
            },
            MotionPhotoItem {
                mime: "video/mp4".into(),
                semantic: "MotionPhoto".into(),
                length: video.len() as u64,
                padding: 0,
            },
        ];
        let (image, motion) = resolve_heif_motion_photo_ranges(&data, &items).unwrap();
        assert_eq!(image, ByteRange::new(0, still.len() as u64).unwrap());
        assert_eq!(
            motion,
            ByteRange::new((still.len() + 16) as u64, data.len() as u64).unwrap()
        );
        items[0].padding = 8;
        assert!(matches!(
            resolve_heif_motion_photo_ranges(&data, &items),
            Err(MotionPhotoError::InvalidItemLength)
        ));
    }
}
