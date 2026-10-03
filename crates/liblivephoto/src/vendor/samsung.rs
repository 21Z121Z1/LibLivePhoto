//! Independent implementation of evidence/SAMSUNG_SEF_SPEC.md.
use crate::{ByteRange, MotionPhotoReadError as Error};
type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub(crate) struct Record {
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
pub(crate) fn parse(d: &[u8]) -> Result<Option<Sef>> {
    if d.len() < 8 || d.get(d.len() - 4..) != Some(b"SEFT") {
        return Ok(None);
    }
    let size = u32le(d, d.len() - 8)? as usize;
    let start = d
        .len()
        .checked_sub(8)
        .and_then(|e| e.checked_sub(size))
        .ok_or(Error::InvalidResource)?;
    if size < 12 || d.get(start..start + 4) != Some(b"SEFH") {
        return Err(Error::InvalidResource);
    }
    let version = u32le(d, start + 4)?;
    let count = u32le(d, start + 8)? as usize;
    if count > 4096 || 12 + count * 12 != size {
        return Err(Error::InvalidResource);
    }
    let mut records = Vec::new();
    let mut motion = None;
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
        if kind == 0x0a30 && name == "MotionPhoto_Data" {
            let p = &d[payload.lower_bound as usize..payload.upper_bound as usize];
            let r = if p.len() == 12 && p.starts_with(b"mpv2") {
                let a = u32::from_be_bytes(p[4..8].try_into().map_err(|_| Error::InvalidResource)?)
                    as u64;
                let l = u32::from_be_bytes(p[8..12].try_into().map_err(|_| Error::InvalidResource)?)
                    as u64;
                ByteRange::new(
                    a,
                    a.checked_add(l)
                        .filter(|e| *e <= d.len() as u64)
                        .ok_or(Error::InvalidResource)?,
                )?
            } else {
                payload
            };
            let video = d
                .get(r.lower_bound as usize..r.upper_bound as usize)
                .ok_or(Error::InvalidResource)?;
            if !crate::scanner::is_ftyp_box_start(video, 0, video.len() as u64)?
                || crate::normalize_embedded_video(video)?.data.len() != video.len()
                || motion.replace(r).is_some()
            {
                return Err(Error::InvalidResource);
            }
        }
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
    let Some(motion) = motion else {
        return Ok(None);
    };
    for r in &records {
        if r.kind != 0x0a30
            && r.raw.lower_bound < motion.upper_bound
            && motion.lower_bound < r.raw.upper_bound
        {
            return Err(Error::InvalidResource);
        }
    }
    Ok(Some(Sef {
        version,
        records,
        motion,
    }))
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
