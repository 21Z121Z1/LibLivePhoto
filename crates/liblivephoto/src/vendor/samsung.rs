//! Independent implementation of evidence/SAMSUNG_SEF_SPEC.md.
use crate::{ByteRange, MotionPhotoReadError as Error};
type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub kind: u16,
    pub name: String,
    pub payload: ByteRange,
    pub raw: ByteRange,
}
pub(crate) struct Sef {
    pub version: u32,
    pub records: Vec<Record>,
    pub motion: ByteRange,
}
fn u32le(d: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        d.get(p..p + 4)
            .and_then(|b| b.try_into().ok())
            .ok_or(Error::InvalidResource)?,
    ))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamsungMetadata {
    pub version: u32,
    pub records: Vec<Record>,
    pub video: Option<ByteRange>,
    pub motion_photo_data_magic: Option<[u8; 4]>,
}
struct Directory {
    version: u32,
    records: Vec<Record>,
    mpvd: Option<ByteRange>,
}
pub(crate) fn parse(d: &[u8]) -> Result<Option<Sef>> {
    Ok(inspect_samsung_metadata(d)?.and_then(|m| {
        m.video.map(|motion| Sef {
            version: m.version,
            records: m.records,
            motion,
        })
    }))
}
fn directory(d: &[u8]) -> Result<Option<Directory>> {
    let mut footer_end = None;
    let mut mpvd = None;
    if d.len() >= 8 && d.get(d.len() - 4..) == Some(b"SEFT") {
        footer_end = Some(d.len());
    }
    if d.get(4..8) == Some(b"ftyp") {
        let scan = liblivephoto_format::isobmff::scan_top_level_boxes(d)
            .map_err(|_| Error::InvalidResource)?;
        for b in scan.boxes {
            if b.kind.as_bytes() == b"mpvd"
                && mpvd
                    .replace(ByteRange::new(b.data_start as u64, b.data_end as u64)?)
                    .is_some()
            {
                return Err(Error::AmbiguousRecovery);
            }
            if b.kind.as_bytes() == b"sefd"
                && d.get(b.data_end.saturating_sub(4)..b.data_end) == Some(b"SEFT")
            {
                if footer_end.is_some_and(|end| end != b.data_end) {
                    return Err(Error::AmbiguousRecovery);
                }
                footer_end = Some(b.data_end);
            }
        }
    }
    let Some(footer_end) = footer_end else {
        return Ok(None);
    };
    let size = u32le(d, footer_end - 8)? as usize;
    let start = footer_end
        .checked_sub(8)
        .and_then(|e| e.checked_sub(size))
        .ok_or(Error::InvalidResource)?;
    if size < 12 || d.get(start..start + 4) != Some(b"SEFH") {
        return Err(Error::InvalidResource);
    }
    let version = u32le(d, start + 4)?;
    let count = u32le(d, start + 8)? as usize;
    if count > 4096 || 12 + count * 12 > size {
        return Err(Error::InvalidResource);
    }
    let mut records = Vec::new();
    for n in 0..count {
        let e = start + 12 + n * 12;
        let offset = u32le(d, e + 4)? as usize;
        let len = u32le(d, e + 8)? as usize;
        let s = start.checked_sub(offset).ok_or(Error::InvalidResource)?;
        let end = s
            .checked_add(len)
            .filter(|e| *e <= start)
            .ok_or(Error::InvalidResource)?;
        if len < 8 || d.get(s..s + 4) != d.get(e..e + 4) {
            return Err(Error::InvalidResource);
        }
        let name_len = u32le(d, s + 4)? as usize;
        if name_len > 4096 || name_len > len - 8 {
            return Err(Error::InvalidResource);
        }
        let name = std::str::from_utf8(&d[s + 8..s + 8 + name_len])
            .map_err(|_| Error::InvalidResource)?
            .to_owned();
        let kind = u16::from_le_bytes([d[e + 2], d[e + 3]]);
        let payload = ByteRange::new((s + 8 + name_len) as u64, end as u64)?;
        records.push(Record {
            kind,
            name,
            payload,
            raw: ByteRange::new(s as u64, end as u64)?,
        });
    }
    let mut ranges: Vec<_> = records.iter().map(|r| r.raw).collect();
    ranges.sort_by_key(|r| r.lower_bound);
    if ranges
        .windows(2)
        .any(|w| w[0].upper_bound > w[1].lower_bound)
    {
        return Err(Error::InvalidResource);
    }
    Ok(Some(Directory {
        version,
        records,
        mpvd,
    }))
}

fn primary(record: &Record) -> bool {
    record.kind == 0x0a30 && record.name == "MotionPhoto_Data"
}
fn bytes(data: &[u8], range: ByteRange) -> Result<&[u8]> {
    let start = usize::try_from(range.lower_bound).map_err(|_| Error::InvalidResource)?;
    let end = usize::try_from(range.upper_bound).map_err(|_| Error::InvalidResource)?;
    data.get(start..end).ok_or(Error::InvalidResource)
}
fn movie_range(data: &[u8], dir: &Directory, index: usize) -> Result<(ByteRange, Option<[u8; 4]>)> {
    let record = &dir.records[index];
    let p = bytes(data, record.payload)?;
    let magic = primary(record)
        .then(|| p.get(..4).filter(|m| matches!(*m, b"mpv2" | b"mpv3")))
        .flatten();
    let range = if magic.is_some() {
        if p.len() < 12 {
            return Err(Error::InvalidResource);
        }
        let at = u32::from_be_bytes(p[4..8].try_into().unwrap()) as u64;
        let length = u32::from_be_bytes(p[8..12].try_into().unwrap()) as u64;
        if p.len() > 12 {
            if length != 0 && length != p.len() as u64 - 12 {
                return Err(Error::MetadataMismatch("SEF inline movie length"));
            }
            ByteRange::new(record.payload.lower_bound + 12, record.payload.upper_bound)?
        } else if length > 0 {
            ByteRange::new(
                at,
                at.checked_add(length)
                    .filter(|e| *e <= data.len() as u64)
                    .ok_or(Error::InvalidResource)?,
            )?
        } else {
            dir.mpvd.ok_or(Error::InvalidResource)?
        }
    } else if primary(record) && p.is_empty() {
        dir.mpvd.ok_or(Error::InvalidResource)?
    } else {
        record.payload
    };
    let video = bytes(data, range)?;
    if !crate::scanner::is_ftyp_box_start(video, 0, video.len() as u64)? {
        return Err(Error::InvalidResource);
    }
    if dir.records.iter().enumerate().any(|(other, r)| {
        other != index
            && r.raw.lower_bound < range.upper_bound
            && range.lower_bound < r.raw.upper_bound
    }) {
        return Err(Error::InvalidResource);
    }
    Ok((range, magic.map(|m| m.try_into().unwrap())))
}

/// Existing unique-primary inspector. Metadata-only directories remain usable;
/// conflicting primary records are rejected rather than choosing the first.
/// For a complete record inventory and explicit selection use inspect_samsung_videos.
pub fn inspect_samsung_metadata(data: &[u8]) -> Result<Option<SamsungMetadata>> {
    let Some(dir) = directory(data)? else {
        return Ok(None);
    };
    let indices: Vec<_> = dir
        .records
        .iter()
        .enumerate()
        .filter(|(_, r)| primary(r))
        .map(|(i, _)| i)
        .collect();
    if indices.len() > 1 {
        return Err(Error::AmbiguousRecovery);
    }
    let (video, magic) = if let Some(&i) = indices.first() {
        let (range, magic) = movie_range(data, &dir, i)?;
        let video = bytes(data, range)?;
        if crate::normalize_embedded_video(video)?.data.len() != video.len() {
            return Err(Error::InvalidResource);
        }
        (Some(range), magic)
    } else {
        (None, None)
    };
    Ok(Some(SamsungMetadata {
        version: dir.version,
        records: dir.records,
        video,
        motion_photo_data_magic: magic,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SamsungVideo<'a> {
    /// SEF directory index, not a track ID or a native playback role.
    pub record_index: usize,
    pub extent: ByteRange,
    pub motion_photo_data_magic: Option<[u8; 4]>,
    /// Original encoded container bytes, borrowed from the inspected snapshot.
    pub bytes: &'a [u8],
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SamsungVideoDiagnostic {
    pub record_index: usize,
    pub message: String,
}
#[derive(Debug)]
#[non_exhaustive]
pub struct SamsungVideoInventory<'a> {
    pub version: u32,
    pub records: Vec<Record>,
    pub videos: Vec<SamsungVideo<'a>>,
    /// All matching wire declarations, including invalid or conflicting ones.
    pub declared_primary_records: Vec<usize>,
    pub diagnostics: Vec<SamsungVideoDiagnostic>,
    source: &'a [u8],
}
impl<'a> SamsungVideoInventory<'a> {
    /// An explicit index never falls back to another record. With None, only
    /// one declared primary is accepted; absence/ambiguity fails explicitly.
    pub fn select(&self, record_index: Option<usize>) -> Result<&SamsungVideo<'a>> {
        let index = match record_index {
            Some(i) => i,
            None => match self.declared_primary_records.as_slice() {
                [i] => *i,
                [] => return Err(Error::Unrecognized),
                _ => return Err(Error::AmbiguousRecovery),
            },
        };
        self.videos
            .iter()
            .find(|v| v.record_index == index)
            .ok_or(Error::InvalidResource)
    }
    /// Complete opaque record payload, even when it is not a supported video.
    pub fn record_payload(&self, index: usize) -> Result<&'a [u8]> {
        bytes(
            self.source,
            self.records
                .get(index)
                .ok_or(Error::InvalidResource)?
                .payload,
        )
    }
    pub fn raw_record(&self, index: usize) -> Result<&'a [u8]> {
        bytes(
            self.source,
            self.records.get(index).ok_or(Error::InvalidResource)?.raw,
        )
    }
}

/// Read-only catalogue for JPEG/HEIF SEF, even with absent/conflicting primary
/// declarations. Only complete, structurally checked BMFF movies with a video
/// track are selectable. No codec decoding or name-based autoplay/depth role.
/// Unknown/unsupported records remain byte-extractable. Bounds:256MiB source,
///4096 records;64 distinct candidate ranges and64MiB unique candidate bytes.
pub fn inspect_samsung_videos(data: &[u8]) -> Result<Option<SamsungVideoInventory<'_>>> {
    if data.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    let Some(dir) = directory(data)? else {
        return Ok(None);
    };
    let mut out = SamsungVideoInventory {
        version: dir.version,
        records: dir.records.clone(),
        videos: Vec::new(),
        declared_primary_records: Vec::new(),
        diagnostics: Vec::new(),
        source: data,
    };
    let mut validated = std::collections::BTreeMap::new();
    let mut total = 0usize;
    for (index, record) in dir.records.iter().enumerate() {
        if primary(record) {
            out.declared_primary_records.push(index);
        } else if !crate::scanner::is_ftyp_box_start(
            bytes(data, record.payload)?,
            0,
            record.payload.length(),
        )
        .unwrap_or(false)
        {
            continue;
        }
        let result = movie_range(data, &dir, index);
        let (range, magic) = match result {
            Ok(pair) => pair,
            Err(error) => {
                out.diagnostics.push(SamsungVideoDiagnostic {
                    record_index: index,
                    message: error.to_string(),
                });
                continue;
            }
        };
        let key = (range.lower_bound, range.upper_bound);
        if !validated.contains_key(&key) {
            total = total
                .checked_add(range.length() as usize)
                .ok_or(Error::InputLimit)?;
            if total > 64 * 1024 * 1024 || validated.len() >= 64 {
                return Err(Error::InputLimit);
            }
            let candidate = bytes(data, range)?;
            let result = crate::normalize_embedded_video(candidate)
                .map_err(Error::from)
                .and_then(|normalized| {
                    if normalized.data.len() != candidate.len() {
                        return Err(Error::InvalidResource);
                    }
                    crate::tracks::inspect(candidate, crate::ResourceId(index), false)
                })
                .and_then(|container| {
                    if container
                        .tracks
                        .iter()
                        .any(|t| t.kind == crate::TrackKind::Video)
                    {
                        Ok(())
                    } else {
                        Err(Error::InvalidResource)
                    }
                })
                .map_err(|e| e.to_string());
            validated.insert(key, result);
        }
        match &validated[&key] {
            Ok(()) => out.videos.push(SamsungVideo {
                record_index: index,
                extent: range,
                motion_photo_data_magic: magic,
                bytes: bytes(data, range)?,
            }),
            Err(error) => out.diagnostics.push(SamsungVideoDiagnostic {
                record_index: index,
                message: error.clone(),
            }),
        }
    }
    Ok(Some(out))
}

pub(crate) fn write(jpeg: &[u8], movie: &[u8], source: Option<(&[u8], &Sef)>) -> Result<Vec<u8>> {
    let mut output = jpeg.to_vec();
    let mut entries = Vec::new();
    if let Some((data, sef)) = source {
        for r in &sef.records {
            if r.kind == 0x0a30 && r.name == "MotionPhoto_Data" {
                continue;
            }
            let raw = data
                .get(r.raw.lower_bound as usize..r.raw.upper_bound as usize)
                .ok_or(Error::InvalidResource)?;
            entries.push((output.len(), raw.len(), raw[..4].to_vec()));
            output.extend_from_slice(raw);
        }
    }
    let start = output.len();
    let name = b"MotionPhoto_Data";
    let tag = [0, 0, 0x30, 0x0a];
    output.extend_from_slice(&tag);
    output.extend_from_slice(&(name.len() as u32).to_le_bytes());
    output.extend_from_slice(name);
    output.extend_from_slice(movie);
    entries.push((start, output.len() - start, tag.to_vec()));
    let dir = output.len();
    output.extend_from_slice(b"SEFH");
    output.extend_from_slice(&source.map_or(107, |(_, s)| s.version).to_le_bytes());
    output.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (s, len, tag) in entries {
        output.extend_from_slice(&tag);
        output.extend_from_slice(
            &u32::try_from(dir - s)
                .map_err(|_| Error::InvalidResource)?
                .to_le_bytes(),
        );
        output.extend_from_slice(
            &u32::try_from(len)
                .map_err(|_| Error::InvalidResource)?
                .to_le_bytes(),
        );
    }
    let dir_len = u32::try_from(output.len() - dir).map_err(|_| Error::InvalidResource)?;
    output.extend_from_slice(&dir_len.to_le_bytes());
    output.extend_from_slice(b"SEFT");
    Ok(output)
}
