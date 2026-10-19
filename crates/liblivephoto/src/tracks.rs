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
    mut visit: impl FnMut(usize, u32, crate::ByteRange) -> Result<()>,
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
            visit(sample, map[row].2, crate::ByteRange::new(offset, end)?)?;
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
                validate_samples(data, &stbc, &top, &mut sample_budget, |_, _, _| Ok(()))?;
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

/// Raw sample descriptions and validated metadata samples. Offsets stay relative
/// to the movie. No whole-file byte scanning can create an AGTM association.
pub(crate) struct MetadataSample<'a> {
    pub index: usize,
    pub description: [u8; 4],
    pub description_bytes: &'a [u8],
    pub bytes: &'a [u8],
    pub presentation: crate::MediaTime,
    pub duration: u32,
}

pub(crate) fn metadata_samples<'a>(
    data: &'a [u8],
    track: &Track,
) -> Result<Vec<MetadataSample<'a>>> {
    if track.kind == TrackKind::Video || track.kind == TrackKind::Audio {
        return Ok(Vec::new());
    }
    let top = parse_boxes(data, 0..data.len()).map_err(|_| Error::InvalidResource)?;
    if find(&top, b"moof").is_some() {
        return Err(Error::UnsupportedLayout("fragmented metadata samples"));
    }
    let moov = find(&top, b"moov").ok_or(Error::InvalidResource)?;
    let children = children(data, moov)?;
    if find(&children, b"mvex").is_some() {
        return Err(Error::UnsupportedLayout("fragmented metadata samples"));
    }
    let trak = children
        .iter()
        .filter(|b| b.kind.as_bytes() == b"trak")
        .nth(track.index)
        .ok_or(Error::InvalidResource)?;
    let tc = self::children(data, trak)?;
    let mdia = find(&tc, b"mdia").ok_or(Error::InvalidResource)?;
    let mc = self::children(data, mdia)?;
    let minf = find(&mc, b"minf").ok_or(Error::InvalidResource)?;
    let mi = self::children(data, minf)?;
    let stbl = find(&mi, b"stbl").ok_or(Error::InvalidResource)?;
    let tables = self::children(data, stbl)?;
    let Some(stsz) = find(&tables, b"stsz") else {
        return Ok(Vec::new());
    };
    let count = u32_at(payload(data, stsz), 8)? as usize;
    if count > 1_000_000 {
        return Err(Error::InvalidResource);
    }
    let times = sample_timing(data, &tables, count)?;
    let stsd = find(&tables, b"stsd").ok_or(Error::InvalidResource)?;
    let entries = parse_boxes(data, stsd.data_start + 8..stsd.data_end)
        .map_err(|_| Error::InvalidResource)?;
    let mut result = Vec::new();
    let mut budget = 0;
    validate_samples(
        data,
        &tables,
        &top,
        &mut budget,
        |index, description, range| {
            let entry = entries
                .get(description as usize - 1)
                .ok_or(Error::InvalidResource)?;
            let (pts, duration) = times[index];
            result.push(MetadataSample {
                index,
                description: *entry.kind.as_bytes(),
                description_bytes: payload(data, entry),
                bytes: &data[range.lower_bound as usize..range.upper_bound as usize],
                presentation: crate::MediaTime {
                    value: pts,
                    timescale: track.timescale,
                },
                duration,
            });
            Ok(())
        },
    )?;
    Ok(result)
}

pub(crate) fn metadata_at<'a>(
    data: &'a [u8],
    track: &Track,
    movie_timescale: Option<u32>,
    time: crate::MediaTime,
) -> Result<Vec<MetadataSample<'a>>> {
    let target = time
        .rescale_exact(track.timescale)
        .filter(|t| t.value >= 0)
        .ok_or(Error::InvalidPresentation)?
        .value;
    let samples = metadata_samples(data, track)?;
    // Raw packets use the track's media timeline. Selection uses the edited
    // movie timeline; payload bytes and embedded timestamps are never changed.
    let selected_edit = if track.edits.is_empty() {
        None
    } else {
        let movie_scale = movie_timescale.ok_or(Error::InvalidPresentation)?;
        let (mut elapsed, mut selected) = (0i64, None);
        for edit in &track.edits {
            if edit.rate_integer != 1 || edit.rate_fraction != 0 || edit.media_start < -1 {
                return Err(Error::UnsupportedLayout("non-unit metadata timing edit"));
            }
            let duration = crate::MediaTime {
                value: i64::try_from(edit.movie_duration)
                    .map_err(|_| Error::InvalidPresentation)?,
                timescale: movie_scale,
            }
            .rescale_exact(track.timescale)
            .ok_or(Error::InvalidPresentation)?
            .value;
            let end = elapsed
                .checked_add(duration)
                .ok_or(Error::InvalidPresentation)?;
            if edit.media_start >= 0 {
                let media_end = edit
                    .media_start
                    .checked_add(duration)
                    .ok_or(Error::InvalidPresentation)?;
                if target >= elapsed && target < end {
                    selected = Some((elapsed, edit.media_start, media_end));
                }
            }
            elapsed = end;
        }
        // Edits are sequential movie intervals: at most one can contain target.
        // Select before mapping samples, avoiding a samples × edits expansion.
        let Some(selected) = selected else {
            return Ok(Vec::new());
        };
        Some(selected)
    };
    Ok(samples
        .into_iter()
        .filter_map(|mut sample| {
            if let Some((elapsed, media_start, media_end)) = selected_edit {
                let start = i128::from(sample.presentation.value).max(i128::from(media_start));
                let end = (i128::from(sample.presentation.value) + i128::from(sample.duration))
                    .min(i128::from(media_end));
                if start >= end {
                    return None;
                }
                // The intersection is inside the validated edit, so its mapped
                // start fits elapsed..end and its duration fits the raw u32.
                sample.presentation.value =
                    (i128::from(elapsed) + start - i128::from(media_start)) as i64;
                sample.duration = (end - start) as u32;
            }
            Some(sample)
        })
        .filter(|s| {
            i128::from(target) >= i128::from(s.presentation.value)
                && i128::from(target) < i128::from(s.presentation.value) + i128::from(s.duration)
        })
        .collect())
}

fn sample_timing(data: &[u8], tables: &[BoxHeader], count: usize) -> Result<Vec<(i64, u32)>> {
    let stts = find(tables, b"stts").ok_or(Error::InvalidResource)?;
    let timing = payload(data, stts);
    let mut times = Vec::with_capacity(count);
    let mut decode = 0i64;
    for row in timing
        .get(8..)
        .ok_or(Error::InvalidResource)?
        .as_chunks::<8>()
        .0
    {
        let n = u32_at(row, 0)? as usize;
        let duration = u32_at(row, 4)?;
        if times.len().checked_add(n).is_none_or(|v| v > count) {
            return Err(Error::InvalidResource);
        }
        for _ in 0..n {
            times.push((decode, duration));
            decode = decode
                .checked_add(i64::from(duration))
                .ok_or(Error::InvalidPresentation)?;
        }
    }
    if times.len() != count {
        return Err(Error::InvalidResource);
    }
    if let Some(ctts) = find(tables, b"ctts") {
        let p = payload(data, ctts);
        if !matches!(p.first(), Some(0 | 1))
            || p.len()
                != 8 + (u32_at(p, 4)? as usize)
                    .checked_mul(8)
                    .ok_or(Error::InvalidResource)?
        {
            return Err(Error::InvalidResource);
        }
        let mut index = 0usize;
        for row in p[8..].as_chunks::<8>().0 {
            let n = u32_at(row, 0)? as usize;
            let offset = if p[0] == 1 {
                i64::from(i32::from_be_bytes(field(row, 4)?))
            } else {
                i64::from(u32_at(row, 4)?)
            };
            let end = index
                .checked_add(n)
                .filter(|v| *v <= count)
                .ok_or(Error::InvalidResource)?;
            for value in &mut times[index..end] {
                value.0 = value
                    .0
                    .checked_add(offset)
                    .ok_or(Error::InvalidPresentation)?;
            }
            index = end;
        }
        if index != count {
            return Err(Error::InvalidResource);
        }
    }
    Ok(times)
}

/// Exact presentation timestamps for one video track, including unit-rate edits.
/// Sample indices remain decode-order indices; returned timestamps are sorted.
/// Does not decode frames or assume a constant frame rate. Fragmentation and
/// non-unit edit rates are explicitly unsupported. Allocation is caller-bounded.
pub(crate) fn video_frames(
    data: &[u8],
    id: u32,
    max_frames: usize,
) -> Result<Vec<crate::FrameTime>> {
    video_frames_with_edits(data, id, max_frames, true)
}

/// Source sample CTS, before elst. Used only with an explicitly selected video
/// track; embedded scoring timestamps are not metadata-track decode timestamps.
pub(crate) fn video_frames_unedited(
    data: &[u8],
    id: u32,
    max_frames: usize,
) -> Result<Vec<crate::FrameTime>> {
    video_frames_with_edits(data, id, max_frames, false)
}

fn video_frames_with_edits(
    data: &[u8],
    id: u32,
    max_frames: usize,
    apply_edits: bool,
) -> Result<Vec<crate::FrameTime>> {
    let container = inspect(data, ResourceId(0), false)?;
    let track = container
        .tracks
        .iter()
        .find(|t| t.id == id && t.kind == TrackKind::Video)
        .ok_or(Error::InvalidResource)?;
    let top = parse_boxes(data, 0..data.len()).map_err(|_| Error::InvalidResource)?;
    if find(&top, b"moof").is_some() {
        return Err(Error::UnsupportedLayout("fragmented frame timing"));
    }
    let moov = find(&top, b"moov").ok_or(Error::InvalidResource)?;
    let movie = children(data, moov)?;
    if find(&movie, b"mvex").is_some() {
        return Err(Error::UnsupportedLayout("fragmented frame timing"));
    }
    let trak = movie
        .iter()
        .filter(|b| b.kind.as_bytes() == b"trak")
        .nth(track.index)
        .ok_or(Error::InvalidResource)?;
    let tc = children(data, trak)?;
    let mdia = find(&tc, b"mdia").ok_or(Error::InvalidResource)?;
    let mc = children(data, mdia)?;
    let minf = find(&mc, b"minf").ok_or(Error::InvalidResource)?;
    let mi = children(data, minf)?;
    let stbl = find(&mi, b"stbl").ok_or(Error::InvalidResource)?;
    let tables = children(data, stbl)?;
    let count = u32_at(
        payload(data, find(&tables, b"stsz").ok_or(Error::InvalidResource)?),
        8,
    )? as usize;
    if count > max_frames.min(1_000_000) {
        return Err(Error::InputLimit);
    }
    let mut budget = 0;
    validate_samples(data, &tables, &top, &mut budget, |_, _, _| Ok(()))?;
    if apply_edits
        && count
            .checked_mul(track.edits.len().max(1))
            .is_none_or(|n| n > 1_000_000)
    {
        return Err(Error::InputLimit);
    }
    let times = sample_timing(data, &tables, count)?;
    let mut frames = Vec::new();
    if !apply_edits || track.edits.is_empty() {
        frames.extend(
            times
                .into_iter()
                .enumerate()
                .map(|(sample_index, (value, _))| crate::FrameTime {
                    sample_index,
                    time: crate::MediaTime {
                        value,
                        timescale: track.timescale,
                    },
                }),
        );
    } else {
        let movie_scale = container
            .movie_timescale
            .ok_or(Error::InvalidPresentation)?;
        let mut elapsed = 0i64;
        for edit in &track.edits {
            if edit.rate_integer != 1 || edit.rate_fraction != 0 || edit.media_start < -1 {
                return Err(Error::UnsupportedLayout("non-unit frame timing edit"));
            }
            let duration =
                i64::try_from(edit.movie_duration).map_err(|_| Error::InvalidPresentation)?;
            // Compare rational endpoints without rounding fractional track ticks.
            if edit.media_start >= 0
                && i128::from(edit.media_start) * i128::from(movie_scale)
                    + i128::from(duration) * i128::from(track.timescale)
                    > i128::from(i64::MAX) * i128::from(movie_scale)
            {
                return Err(Error::InvalidPresentation);
            }
            if edit.media_start >= 0 {
                for (sample_index, (pts, _)) in times.iter().enumerate() {
                    let offset = pts
                        .checked_sub(edit.media_start)
                        .ok_or(Error::InvalidPresentation)?;
                    if offset < 0
                        || i128::from(offset) * i128::from(movie_scale)
                            >= i128::from(duration) * i128::from(track.timescale)
                    {
                        continue;
                    }
                    if frames.len() >= max_frames.min(1_000_000) {
                        return Err(Error::InputLimit);
                    }
                    let time = crate::presentation::add_exact(
                        crate::MediaTime {
                            value: elapsed,
                            timescale: movie_scale,
                        },
                        crate::MediaTime {
                            value: offset,
                            timescale: track.timescale,
                        },
                    )?;
                    frames.push(crate::FrameTime { sample_index, time });
                }
            }
            elapsed = elapsed
                .checked_add(duration)
                .ok_or(Error::InvalidPresentation)?;
        }
    }
    frames.sort_by(|a, b| {
        (i128::from(a.time.value) * i128::from(b.time.timescale))
            .cmp(&(i128::from(b.time.value) * i128::from(a.time.timescale)))
    });
    Ok(frames)
}

pub(crate) fn sample_ranges(data: &[u8], track: &Track) -> Result<Vec<crate::ByteRange>> {
    let top = parse_boxes(data, 0..data.len()).map_err(|_| Error::InvalidResource)?;
    let moov = find(&top, b"moov").ok_or(Error::InvalidResource)?;
    let mc = children(data, moov)?;
    let trak = mc
        .iter()
        .filter(|b| b.kind.as_bytes() == b"trak")
        .nth(track.index)
        .ok_or(Error::InvalidResource)?;
    let tc = children(data, trak)?;
    let mdia = find(&tc, b"mdia").ok_or(Error::InvalidResource)?;
    let md = children(data, mdia)?;
    let minf = find(&md, b"minf").ok_or(Error::InvalidResource)?;
    let mi = children(data, minf)?;
    let stbl = find(&mi, b"stbl").ok_or(Error::InvalidResource)?;
    let tables = children(data, stbl)?;
    let mut result = Vec::new();
    let mut budget = 0;
    validate_samples(data, &tables, &top, &mut budget, |_, _, r| {
        result.push(r);
        Ok(())
    })?;
    Ok(result)
}
