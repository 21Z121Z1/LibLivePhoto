//! Bounded BMFF track inspection. No vendor name assigns a track role.
use crate::{
    ContainerKind, Edit, MediaContainer, MotionPhotoReadError as Error, Provenance, ResourceId,
    ResourceRole, Track, TrackKind,
};
use liblivephoto_format::isobmff::{parse_boxes, BoxHeader};

type Result<T> = std::result::Result<T, Error>;
fn children(data: &[u8], header: &BoxHeader) -> Result<Vec<BoxHeader>> {
    parse_boxes(data, header.data_start..header.data_end).map_err(|_| Error::InvalidResource)
}
fn field<const N: usize>(data: &[u8], pos: usize) -> Result<[u8; N]> {
    data.get(pos..pos.checked_add(N).ok_or(Error::InvalidResource)?)
        .and_then(|v| v.try_into().ok())
        .ok_or(Error::InvalidResource)
}
fn u32_at(data: &[u8], pos: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(field(data, pos)?))
}
fn u64_at(data: &[u8], pos: usize) -> Result<u64> {
    Ok(u64::from_be_bytes(field(data, pos)?))
}
fn payload<'a>(data: &'a [u8], b: &BoxHeader) -> &'a [u8] {
    &data[b.data_start..b.data_end]
}
fn find<'a>(bs: &'a [BoxHeader], kind: &[u8; 4]) -> Option<&'a BoxHeader> {
    bs.iter().find(|b| b.kind.as_bytes() == kind)
}
fn timeline(p: &[u8]) -> Result<(u32, u64)> {
    match p.first() {
        Some(0) => Ok((u32_at(p, 12)?, u64::from(u32_at(p, 16)?))),
        Some(1) => Ok((u32_at(p, 20)?, u64_at(p, 24)?)),
        _ => Err(Error::InvalidPresentation),
    }
    .and_then(|v| {
        if v.0 == 0 {
            Err(Error::InvalidPresentation)
        } else {
            Ok(v)
        }
    })
}

fn validate_samples(
    data: &[u8],
    tables: &[BoxHeader],
    top: &[BoxHeader],
    budget: &mut usize,
) -> Result<()> {
    let Some(stsz) = find(tables, b"stsz") else {
        return Ok(());
    };
    let p = payload(data, stsz);
    let constant = u32_at(p, 4)? as u64;
    let count = u32_at(p, 8)? as usize;
    *budget = budget
        .checked_add(count)
        .filter(|n| *n <= 1_000_000)
        .ok_or(Error::InvalidResource)?;
    if p.len() != 12 + if constant == 0 { count * 4 } else { 0 } {
        return Err(Error::InvalidResource);
    }
    let stts = find(tables, b"stts").ok_or(Error::InvalidResource)?;
    let t = payload(data, stts);
    let entries = u32_at(t, 4)? as usize;
    if t.len() != 8 + entries.checked_mul(8).ok_or(Error::InvalidResource)? {
        return Err(Error::InvalidResource);
    }
    let mut timed = 0u64;
    for e in t[8..].as_chunks::<8>().0 {
        timed = timed
            .checked_add(u64::from(u32_at(e, 0)?))
            .ok_or(Error::InvalidResource)?;
    }
    if timed != count as u64 {
        return Err(Error::InvalidResource);
    }
    let stsc = find(tables, b"stsc").ok_or(Error::InvalidResource)?;
    let s = payload(data, stsc);
    let entries = u32_at(s, 4)? as usize;
    if s.len() != 8 + entries.checked_mul(12).ok_or(Error::InvalidResource)? {
        return Err(Error::InvalidResource);
    }
    let map = s[8..]
        .as_chunks::<12>()
        .0
        .iter()
        .map(|e| Ok((u32_at(e, 0)?, u32_at(e, 4)?, u32_at(e, 8)?)))
        .collect::<Result<Vec<_>>>()?;
    if map.iter().any(|e| e.0 == 0 || e.1 == 0 || e.2 == 0)
        || map.windows(2).any(|w| w[0].0 >= w[1].0)
    {
        return Err(Error::InvalidResource);
    }
    let (offsets, width) = match (find(tables, b"stco"), find(tables, b"co64")) {
        (Some(b), None) => (b, 4),
        (None, Some(b)) => (b, 8),
        _ => return Err(Error::InvalidResource),
    };
    let o = payload(data, offsets);
    let chunks = u32_at(o, 4)? as usize;
    if o.len() != 8 + chunks.checked_mul(width).ok_or(Error::InvalidResource)?
        || (chunks > 0 && map.first().map(|e| e.0) != Some(1))
    {
        return Err(Error::InvalidResource);
    }
    let mdats: Vec<_> = top
        .iter()
        .filter(|b| b.kind.as_bytes() == b"mdat")
        .collect();
    let mut sample = 0usize;
    let mut row = 0;
    for chunk in 0..chunks {
        while row + 1 < map.len() && map[row + 1].0 as usize <= chunk + 1 {
            row += 1;
        }
        let mut offset = if width == 4 {
            u64::from(u32_at(o, 8 + chunk * 4)?)
        } else {
            u64_at(o, 8 + chunk * 8)?
        };
        for _ in 0..map[row].1 {
            if sample >= count {
                return Err(Error::InvalidResource);
            }
            let len = if constant == 0 {
                u64::from(u32_at(p, 12 + sample * 4)?)
            } else {
                constant
            };
            let end = offset.checked_add(len).ok_or(Error::InvalidResource)?;
            if !mdats
                .iter()
                .any(|m| offset >= m.data_start as u64 && end <= m.data_end as u64)
            {
                return Err(Error::InvalidResource);
            }
            offset = end;
            sample += 1;
        }
    }
    if sample != count || map.last().is_some_and(|e| e.0 as usize > chunks) {
        return Err(Error::InvalidResource);
    }
    Ok(())
}

pub(crate) fn inspect(
    data: &[u8],
    resource: ResourceId,
    standard_roles: bool,
) -> Result<MediaContainer> {
    let top = parse_boxes(data, 0..data.len()).map_err(|_| Error::InvalidResource)?;
    let moov = find(&top, b"moov").ok_or(Error::InvalidResource)?;
    let bs = children(data, moov)?;
    let movie_timescale = find(&bs, b"mvhd")
        .map(|b| timeline(payload(data, b)).map(|v| v.0))
        .transpose()?;
    let mut tracks = Vec::new();
    let mut sample_budget = 0;
    for b in bs.iter().filter(|b| b.kind.as_bytes() == b"trak") {
        if tracks.len() >= 256 {
            return Err(Error::InvalidResource);
        }
        let tc = children(data, b)?;
        let tkhd = find(&tc, b"tkhd").ok_or(Error::InvalidResource)?;
        let p = payload(data, tkhd);
        let id = u32_at(
            p,
            match p.first() {
                Some(0) => 12,
                Some(1) => 20,
                _ => return Err(Error::InvalidResource),
            },
        )?;
        if id == 0 || tracks.iter().any(|t: &Track| t.id == id) {
            return Err(Error::InvalidResource);
        }
        let mdia = find(&tc, b"mdia").ok_or(Error::InvalidResource)?;
        let mc = children(data, mdia)?;
        let mdhd = find(&mc, b"mdhd").ok_or(Error::InvalidResource)?;
        let (timescale, duration) = timeline(payload(data, mdhd))?;
        let handler = match find(&mc, b"hdlr") {
            Some(h) => field(payload(data, h), 8)?,
            None => *b"    ",
        };
        let kind = match &handler {
            b"vide" => TrackKind::Video,
            b"soun" => TrackKind::Audio,
            b"meta" | b"mdta" => TrackKind::Metadata,
            _ => TrackKind::Unknown,
        };
        let mut sample_entries = Vec::new();
        if let Some(minf) = find(&mc, b"minf") {
            let minfc = children(data, minf)?;
            if let Some(stbl) = find(&minfc, b"stbl") {
                let stbc = children(data, stbl)?;
                validate_samples(data, &stbc, &top, &mut sample_budget)?;
                if let Some(stsd) = find(&stbc, b"stsd") {
                    let count = u32_at(payload(data, stsd), 4)? as usize;
                    let entries = parse_boxes(data, stsd.data_start + 8..stsd.data_end)
                        .map_err(|_| Error::InvalidResource)?;
                    if entries.len() != count || count > 256 {
                        return Err(Error::InvalidResource);
                    }
                    sample_entries = entries.iter().map(|b| *b.kind.as_bytes()).collect();
                }
            }
        }
        let mut edits = Vec::new();
        if let Some(edts) = find(&tc, b"edts") {
            let ec = children(data, edts)?;
            if let Some(elst) = find(&ec, b"elst") {
                let p = payload(data, elst);
                let width = match p.first() {
                    Some(0) => 12,
                    Some(1) => 20,
                    _ => return Err(Error::InvalidPresentation),
                };
                let count = u32_at(p, 4)? as usize;
                if count > 4096 || p.len().checked_sub(8) != count.checked_mul(width) {
                    return Err(Error::InvalidPresentation);
                }
                for e in p[8..].chunks_exact(width) {
                    let (movie_duration, media_start) = if width == 12 {
                        (
                            u64::from(u32_at(e, 0)?),
                            i64::from(i32::from_be_bytes(field(e, 4)?)),
                        )
                    } else {
                        (u64_at(e, 0)?, i64::from_be_bytes(field(e, 8)?))
                    };
                    edits.push(Edit {
                        movie_duration,
                        media_start,
                        rate_integer: i16::from_be_bytes(field(e, width - 4)?),
                        rate_fraction: i16::from_be_bytes(field(e, width - 2)?),
                    });
                }
            }
        }
        tracks.push(Track {
            id,
            index: tracks.len(),
            kind,
            role: if kind == TrackKind::Video {
                ResourceRole::UnknownVideo
            } else {
                ResourceRole::Unknown
            },
            handler,
            sample_entries,
            timescale,
            duration,
            edits,
            relationship: None,
            vendor_role: None,
            provenance: Provenance::observed("BMFF tkhd/mdhd/hdlr/stsd/elst"),
        });
    }
    let videos: Vec<_> = tracks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == TrackKind::Video)
        .map(|(i, _)| i)
        .collect();
    if videos.len() == 1 || (standard_roles && videos.len() == 2) {
        tracks[videos[0]].role = ResourceRole::PrimaryMotionVideo;
        if videos.len() == 2 {
            tracks[videos[1]].role = ResourceRole::SubstitutionVideo;
            tracks[videos[1]].relationship = Some(crate::TrackRelationship {
                kind: crate::RelationshipKind::Substitutes,
                target_track_id: tracks[videos[0]].id,
            });
            tracks[videos[1]].vendor_role = Some("Android optional secondary video track".into());
            tracks[videos[1]].provenance = Provenance::declared(
                "Android Motion Photo 1.0 secondary-track ordering and substitution contract",
            );
        }
    }
    Ok(MediaContainer {
        resource,
        kind: ContainerKind::IsoBmff,
        movie_timescale,
        tracks,
    })
}
