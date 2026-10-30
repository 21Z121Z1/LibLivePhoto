//! Bounded, read-only framing of the UUID extensions in independent vivo originals.
//! Keys and encoding tags stay opaque; no portrait/depth, time or pairing inference.
use crate::{ByteRange, CameraAlbumFooter, MotionPhoto, MotionPhotoReadError as Error};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VivoMediaStream<'a> {
    pub key: u32,
    /// Observed wire tag only. This is not a decoded content/semantic type.
    pub encoding: u8,
    /// All extents are relative to the immutable input movie/container snapshot.
    pub payload_extent: ByteRange,
    pub framing_extent: ByteRange,
    pub directory_extent: ByteRange,
    pub raw_payload: &'a [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct VivoMediaExtensions<'a> {
    pub stream_box_extent: Option<ByteRange>,
    pub stream_directory_extent: Option<ByteRange>,
    pub streams: Vec<VivoMediaStream<'a>>,
    /// Ordered records retain duplicate keys; no key-based last-wins selection.
    pub duplicate_keys: bool,
    pub footer_box_extent: Option<ByteRange>,
    pub footer: Option<CameraAlbumFooter<'a>>,
}

fn range(start: usize, end: usize) -> Result<ByteRange, Error> {
    Ok(ByteRange::new(start as u64, end as u64)?)
}

fn streams<'a>(
    data: &'a [u8],
    start: usize,
    end: usize,
) -> Result<(Vec<VivoMediaStream<'a>>, ByteRange, bool), Error> {
    let count_at = end
        .checked_sub(13)
        .filter(|n| *n >= start)
        .ok_or(Error::MetadataMismatch("vivo stream count extent"))?;
    if &data[count_at..end - 2] != b"streamcount" {
        return Err(Error::MetadataMismatch("vivo stream count marker"));
    }
    let count = usize::from(u16::from_be_bytes([data[end - 2], data[end - 1]]));
    if count > 4096 {
        return Err(Error::InputLimit);
    }
    let table = count_at
        .checked_sub(count.checked_mul(19).ok_or(Error::InputLimit)?)
        .filter(|n| *n >= start)
        .ok_or(Error::MetadataMismatch("vivo stream directory extent"))?;
    // Preflight declarations before building record vectors or exposing a prefix.
    let rows = data[table..count_at].as_chunks::<19>().0;
    let mut declared = 0usize;
    for row in rows {
        if &row[..10] != b"streaminfo" {
            return Err(Error::MetadataMismatch("vivo stream directory marker"));
        }
        let size = u32::from_be_bytes(row[15..19].try_into().unwrap()) as usize;
        declared = declared.checked_add(size).ok_or(Error::InputLimit)?;
        if declared > 16 * 1024 * 1024 {
            return Err(Error::InputLimit);
        }
    }
    if declared.checked_add(count * 10) != Some(table - start) {
        return Err(Error::MetadataMismatch(
            "vivo stream lengths and enclosing UUID",
        ));
    }
    let (mut records, mut keys, mut duplicate_keys) =
        (Vec::with_capacity(count), BTreeSet::new(), false);
    let mut cursor = start;
    for (index, row) in rows.iter().enumerate() {
        let payload_start = cursor.checked_add(10).ok_or(Error::InputLimit)?;
        if data.get(cursor..payload_start) != Some(b"streamdata") {
            return Err(Error::MetadataMismatch("vivo stream data marker"));
        }
        let size = u32::from_be_bytes(row[15..19].try_into().unwrap()) as usize;
        let payload_end = payload_start
            .checked_add(size)
            .filter(|n| *n <= table)
            .ok_or(Error::MetadataMismatch("vivo stream data extent"))?;
        let key = u32::from_be_bytes(row[10..14].try_into().unwrap());
        duplicate_keys |= !keys.insert(key);
        records.push(VivoMediaStream {
            key,
            encoding: row[14],
            payload_extent: range(payload_start, payload_end)?,
            framing_extent: range(cursor, payload_start)?,
            directory_extent: range(table + index * 19, table + (index + 1) * 19)?,
            raw_payload: &data[payload_start..payload_end],
        });
        cursor = payload_end;
    }
    Ok((records, range(table, end)?, duplicate_keys))
}

/// Inspect only top-level `vivoMediaEStream` and `vivoMediaExtInfo` UUIDs.
/// The stream directory has fixed markers, u32 keys/lengths and an opaque u8 tag;
/// footer fields use the existing CameraAlbum contract. No source mutation,
/// writer, inferred imageTime unit, stream role or capture-pair verification.
/// This inspects framing, not movie codec validity or native Gallery acceptance.
/// Unrelated non-BMFF/no-matching-UUID inputs return None. Known malformed
/// declarations fail. Limits:256MiB input,65536 top-level boxes (shared parser),
/// one of each known UUID,4096 streams and16MiB aggregate stream payload;
/// CameraAlbum JSON/depth/token limits apply separately.
pub fn inspect_vivo_media_extensions(
    data: &[u8],
) -> Result<Option<VivoMediaExtensions<'_>>, Error> {
    if data.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    if data.get(4..8) != Some(b"ftyp") {
        return Ok(None);
    }
    let boxes = liblivephoto_format::isobmff::parse_boxes(data, 0..data.len()).map_err(|_| {
        Error::UnsupportedLayout("vivo extension inspection requires complete bounded BMFF boxes")
    })?;
    let mut out = VivoMediaExtensions {
        stream_box_extent: None,
        stream_directory_extent: None,
        streams: Vec::new(),
        duplicate_keys: false,
        footer_box_extent: None,
        footer: None,
    };
    for b in boxes {
        if b.kind.as_bytes() != b"uuid" || b.data_end - b.data_start < 16 {
            continue;
        }
        match &data[b.data_start..b.data_start + 16] {
            b"vivoMediaEStream" => {
                if out.stream_box_extent.is_some() {
                    return Err(Error::UnsupportedLayout(
                        "multiple vivo stream UUID boxes require an explicit association",
                    ));
                }
                let (records, directory, duplicates) =
                    streams(data, b.data_start + 16, b.data_end)?;
                out.stream_box_extent = Some(range(b.box_start, b.data_end)?);
                out.stream_directory_extent = Some(directory);
                out.streams = records;
                out.duplicate_keys = duplicates;
            }
            b"vivoMediaExtInfo" => {
                if out.footer.is_some() {
                    return Err(Error::UnsupportedLayout(
                        "multiple vivo footer UUID boxes require an explicit association",
                    ));
                }
                let footer = crate::inspect_camera_album_footer(&data[..b.data_end])?
                    .ok_or(Error::MetadataMismatch("vivo footer UUID envelope"))?;
                if footer.extent.lower_bound != b.data_start as u64 {
                    return Err(Error::MetadataMismatch(
                        "vivo footer escapes its UUID payload",
                    ));
                }
                out.footer_box_extent = Some(range(b.box_start, b.data_end)?);
                out.footer = Some(footer);
            }
            _ => {}
        }
    }
    Ok((out.stream_box_extent.is_some() || out.footer.is_some()).then_some(out))
}

impl MotionPhoto<'_> {
    /// UUID extents are relative to `motion_video_bytes()`, not the enclosing still.
    pub fn vivo_media_extensions(&self) -> Result<Option<VivoMediaExtensions<'_>>, Error> {
        inspect_vivo_media_extensions(self.motion_video_bytes()?)
    }
}
