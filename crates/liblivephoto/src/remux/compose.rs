//! Offset rewriting without decoding media. Unknown offset-bearing contracts
//! are rejected. Unreferenced mdat bytes are retained unless explicitly compacted.
use crate::{ByteRange, MediaContainer, MotionPhotoReadError as Error, ResourceId, Track};
use liblivephoto_format::{
    isobmff::{make_box, parse_boxes, BoxHeader},
    FourCC,
};
use std::collections::BTreeMap;
const MAX_BYTES: usize = 256 * 1024 * 1024;
fn err() -> Error {
    Error::InvalidResource
}
fn u32_at(d: &[u8], p: usize) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(
        d.get(p..p + 4)
            .and_then(|b| b.try_into().ok())
            .ok_or_else(err)?,
    ))
}
fn boxed(kind: &[u8; 4], p: &[u8]) -> Result<Vec<u8>, Error> {
    make_box(FourCC::new(*kind), p).map_err(|_| err())
}
fn children(d: &[u8], b: &BoxHeader) -> Result<Vec<BoxHeader>, Error> {
    parse_boxes(d, b.payload_range()).map_err(|_| err())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMapping {
    pub source: usize,
    pub original_id: u32,
    pub output_id: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemuxReport {
    pub track_mappings: Vec<TrackMapping>,
    pub removed_track_ids: Vec<u32>,
    pub discarded_unreferenced_media_bytes: u64,
    pub retained_unreferenced_media_bytes: u64,
    pub discarded_padding_bytes: u64,
    pub fast_start: bool,
    pub encoded_samples_unchanged: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemuxedMovie {
    pub bytes: Vec<u8>,
    pub report: RemuxReport,
    pub sidecars: Vec<RemuxSidecar>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemuxSidecar {
    pub source: usize,
    pub kind: [u8; 4],
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OptimizeOptions {
    /// Explicit permission to discard mdat bytes not referenced by retained
    /// samples. Unknown media bytes are otherwise copied in full.
    pub compact_unreferenced_media: bool,
}
struct Source<'a> {
    data: &'a [u8],
    model: MediaContainer,
    top: Vec<BoxHeader>,
    movie: Vec<BoxHeader>,
}
fn source(d: &[u8]) -> Result<Source<'_>, Error> {
    if d.len() > MAX_BYTES {
        return Err(Error::InputLimit);
    }
    let model = crate::tracks::inspect(d, ResourceId(0), false)?;
    let top = parse_boxes(d, 0..d.len()).map_err(|_| err())?;
    if top.iter().any(|b| {
        !matches!(
            b.kind.as_bytes(),
            b"ftyp" | b"moov" | b"mdat" | b"free" | b"skip" | b"wide"
        )
    }) {
        return Err(Error::UnsupportedLayout(
            "remux unknown or fragmented/indexed top-level box",
        ));
    }
    let moovs = top
        .iter()
        .filter(|b| b.kind.as_bytes() == b"moov")
        .collect::<Vec<_>>();
    if moovs.len() != 1 || top.iter().filter(|b| b.kind.as_bytes() == b"ftyp").count() != 1 {
        return Err(err());
    }
    let movie = children(d, moovs[0])?;
    if movie.iter().any(|b| {
        !matches!(
            b.kind.as_bytes(),
            b"mvhd" | b"trak" | b"udta" | b"meta" | b"free" | b"skip"
        )
    }) {
        return Err(Error::UnsupportedLayout(
            "unknown movie-level remux reference contract",
        ));
    }
    for b in &movie {
        if b.kind.as_bytes() == b"mvhd" {
            let p = &d[b.payload_range()];
            let needed = match p.first() {
                Some(0) => 100,
                Some(1) => 112,
                _ => return Err(err()),
            };
            if p.len() < needed {
                return Err(err());
            }
        }
        if matches!(b.kind.as_bytes(), b"meta" | b"udta") {
            check_metadata(d, b)?;
        }
        if b.kind.as_bytes() == b"trak" {
            check_track(d, b)?;
        }
    }
    Ok(Source {
        data: d,
        model,
        top,
        movie,
    })
}
fn check_metadata(d: &[u8], b: &BoxHeader) -> Result<(), Error> {
    check_metadata_depth(d, b, 0)
}
fn check_metadata_depth(d: &[u8], b: &BoxHeader, depth: usize) -> Result<(), Error> {
    if depth >= 128 {
        return Err(Error::InputLimit);
    }
    let range = if b.kind.as_bytes() == b"meta" {
        if d.get(b.data_start..b.data_start + 4) == Some(&[0, 0, 0, 0]) {
            b.data_start + 4..b.data_end
        } else if d.get(b.data_start + 4..b.data_start + 8) == Some(b"hdlr") {
            b.payload_range()
        } else {
            return Err(Error::UnsupportedLayout(
                "nonstandard movie metadata header",
            ));
        }
    } else {
        b.payload_range()
    };
    for child in parse_boxes(d, range).map_err(|_| err())? {
        match child.kind.as_bytes() {
            b"meta" => check_metadata_depth(d, &child, depth + 1)?,
            b"hdlr" | b"keys" | b"ilst" | b"name" | b"cprt" => {}
            key if key[0] == 0xa9 => {}
            _ => {
                return Err(Error::UnsupportedLayout(
                    "unknown movie metadata reference contract",
                ))
            }
        }
    }
    Ok(())
}
fn check_track(d: &[u8], b: &BoxHeader) -> Result<(), Error> {
    for c in children(d, b)? {
        match c.kind.as_bytes() {
            b"tkhd" => {
                let p = &d[c.payload_range()];
                let needed = match p.first() {
                    Some(0) => 84,
                    Some(1) => 96,
                    _ => return Err(err()),
                };
                if p.len() < needed {
                    return Err(err());
                }
            }
            b"udta" => check_metadata(d, &c)?,
            b"edts" => {
                if children(d, &c)?
                    .iter()
                    .any(|v| v.kind.as_bytes() != b"elst")
                {
                    return Err(Error::UnsupportedLayout("unknown edit container box"));
                }
            }
            b"mdia" => {
                for md in children(d, &c)? {
                    match md.kind.as_bytes() {
                        b"mdhd" | b"hdlr" | b"elng" => {}
                        b"minf" => {
                            for mi in children(d, &md)? {
                                match mi.kind.as_bytes() {
                                    b"vmhd" | b"smhd" | b"gmhd" | b"nmhd" | b"hdlr" => {}
                                    b"dinf" => {
                                        for refbox in children(d, &mi)? {
                                            if refbox.kind.as_bytes() != b"dref" {
                                                return Err(Error::UnsupportedLayout(
                                                    "unknown data-reference box",
                                                ));
                                            }
                                            let entries = parse_boxes(
                                                d,
                                                refbox.data_start + 8..refbox.data_end,
                                            )
                                            .map_err(|_| err())?;
                                            if entries.len()
                                                != u32_at(d, refbox.data_start + 4)? as usize
                                                || entries.iter().any(|e| {
                                                    !matches!(e.kind.as_bytes(), b"url " | b"alis")
                                                        || d.get(e.data_start..e.data_start + 4)
                                                            != Some(&[0, 0, 0, 1])
                                                })
                                            {
                                                return Err(Error::UnsupportedLayout(
                                                    "external or unknown media data reference",
                                                ));
                                            }
                                        }
                                    }
                                    b"stbl" => {
                                        for t in children(d, &mi)? {
                                            if !matches!(
                                                t.kind.as_bytes(),
                                                b"stsd"
                                                    | b"stts"
                                                    | b"ctts"
                                                    | b"stsc"
                                                    | b"stsz"
                                                    | b"stco"
                                                    | b"co64"
                                                    | b"stss"
                                                    | b"sdtp"
                                                    | b"cslg"
                                                    | b"sgpd"
                                                    | b"sbgp"
                                                    | b"subs"
                                                    | b"stdp"
                                                    | b"padb"
                                            ) {
                                                return Err(Error::UnsupportedLayout(
                                                    "unsupported sample-table offset contract",
                                                ));
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(Error::UnsupportedLayout(
                                            "unsupported media-information box",
                                        ))
                                    }
                                }
                            }
                        }
                        _ => return Err(Error::UnsupportedLayout("unsupported media-track box")),
                    }
                }
            }
            _ => {
                return Err(Error::UnsupportedLayout(
                    "track references or unknown track-level remux contract",
                ))
            }
        }
    }
    Ok(())
}
#[derive(Clone)]
struct Segment {
    source: usize,
    range: ByteRange,
    new_start: u64,
}
fn rewrite(
    d: &[u8],
    b: &BoxHeader,
    map: &impl Fn(u64) -> Result<u64, Error>,
    id: Option<u32>,
    scale: Option<(u32, u32)>,
) -> Result<Vec<u8>, Error> {
    let k = b.kind.as_bytes();
    if matches!(k, b"trak" | b"mdia" | b"minf" | b"stbl" | b"edts") {
        let mut p = Vec::new();
        for c in children(d, b)? {
            p.extend(rewrite(d, &c, map, id, scale)?);
        }
        return boxed(k, &p);
    }
    if matches!(k, b"stco" | b"co64") {
        let p = &d[b.payload_range()];
        if p.get(..4) != Some(&[0, 0, 0, 0]) {
            return Err(err());
        }
        let n = u32_at(p, 4)? as usize;
        let width = if k == b"stco" { 4 } else { 8 };
        if n > 1_000_000 || p.len() != 8 + n * width {
            return Err(err());
        }
        let mut out = p[..8].to_vec();
        for i in 0..n {
            let at = 8 + i * width;
            let v = if width == 4 {
                u64::from(u32_at(p, at)?)
            } else {
                u64::from_be_bytes(p[at..at + 8].try_into().map_err(|_| err())?)
            };
            out.extend(map(v)?.to_be_bytes());
        }
        return boxed(b"co64", &out);
    }
    let mut output = d[b.box_range()].to_vec();
    let header = b.data_start - b.box_start;
    if k == b"tkhd" {
        let p = &mut output[header..];
        let at = match p.first() {
            Some(0) => 12,
            Some(1) => 20,
            _ => return Err(err()),
        };
        if let Some(id) = id {
            p.get_mut(at..at + 4)
                .ok_or_else(err)?
                .copy_from_slice(&id.to_be_bytes());
        }
        if let Some((from, to)) = scale {
            let at = if p[0] == 0 { 20 } else { 28 };
            rescale_field(p, at, if p[0] == 0 { 4 } else { 8 }, from, to)?;
        }
    }
    if k == b"elst" {
        if let Some((from, to)) = scale {
            let p = &mut output[header..];
            let width = match p.first() {
                Some(0) => 12,
                Some(1) => 20,
                _ => return Err(err()),
            };
            let n = u32_at(p, 4)? as usize;
            if n > 4096 || p.len() != 8 + n * width {
                return Err(err());
            }
            for i in 0..n {
                rescale_field(p, 8 + i * width, if width == 12 { 4 } else { 8 }, from, to)?;
            }
        }
    }
    Ok(output)
}
fn rescale_field(p: &mut [u8], at: usize, width: usize, from: u32, to: u32) -> Result<(), Error> {
    if from == 0 || to == 0 {
        return Err(Error::InvalidPresentation);
    }
    let v = if width == 4 {
        u64::from(u32_at(p, at)?)
    } else {
        u64::from_be_bytes(
            p.get(at..at + 8)
                .and_then(|v| v.try_into().ok())
                .ok_or_else(err)?,
        )
    };
    let n = u128::from(v) * u128::from(to);
    if n % u128::from(from) != 0 {
        return Err(Error::InvalidPresentation);
    }
    let v = u64::try_from(n / u128::from(from)).map_err(|_| Error::InvalidPresentation)?;
    if width == 4 {
        p.get_mut(at..at + 4).ok_or_else(err)?.copy_from_slice(
            &u32::try_from(v)
                .map_err(|_| Error::InvalidPresentation)?
                .to_be_bytes(),
        );
    } else {
        p.get_mut(at..at + 8)
            .ok_or_else(err)?
            .copy_from_slice(&v.to_be_bytes());
    }
    Ok(())
}
fn compose(
    sources: &[Source<'_>],
    selection: &[(usize, &Track, u32)],
    removed: Vec<u32>,
    options: OptimizeOptions,
) -> Result<RemuxedMovie, Error> {
    let mut by_source: BTreeMap<usize, Vec<ByteRange>> = BTreeMap::new();
    let mut referenced = 0u64;
    let mut retained_unreferenced = 0u64;
    let mut discarded = 0u64;
    for (index, s) in sources.iter().enumerate() {
        let mut ranges = Vec::new();
        for (n, t, _) in selection.iter().filter(|(n, _, _)| *n == index) {
            let _ = n;
            ranges.extend(crate::tracks::sample_ranges(s.data, t)?);
        }
        ranges.sort_by_key(|r| r.lower_bound);
        let mut merged: Vec<ByteRange> = Vec::new();
        for r in ranges {
            if let Some(last) = merged.last_mut() {
                if r.lower_bound <= last.upper_bound {
                    last.upper_bound = last.upper_bound.max(r.upper_bound);
                    continue;
                }
            }
            merged.push(r);
        }
        let used = merged.iter().map(|r| r.length()).sum::<u64>();
        referenced = referenced.checked_add(used).ok_or_else(err)?;
        let all = s
            .top
            .iter()
            .filter(|b| b.kind.as_bytes() == b"mdat")
            .map(|b| ByteRange {
                lower_bound: b.data_start as u64,
                upper_bound: b.data_end as u64,
            })
            .collect::<Vec<_>>();
        let total = all.iter().map(|r| r.length()).sum::<u64>();
        let unused = total.checked_sub(used).ok_or_else(err)?;
        if options.compact_unreferenced_media {
            discarded += unused;
            by_source.insert(index, merged);
        } else {
            retained_unreferenced += unused;
            by_source.insert(index, all);
        }
    }
    let mut segments = Vec::new();
    let mut media = Vec::new();
    for (index, ranges) in &by_source {
        for range in ranges {
            segments.push(Segment {
                source: *index,
                range: *range,
                new_start: media.len() as u64,
            });
            media.extend_from_slice(
                &sources[*index].data[range.lower_bound as usize..range.upper_bound as usize],
            );
        }
    }
    if media.len() > MAX_BYTES {
        return Err(Error::InputLimit);
    }
    let primary = &sources[0];
    let ftyp = primary
        .top
        .iter()
        .find(|b| b.kind.as_bytes() == b"ftyp")
        .ok_or_else(err)?;
    // Only the primary movie-level metadata belongs to the destination asset.
    // Imported movie-level metadata is retained as explicit sidecars instead
    // of re-associating it with a different set of destination tracks.
    let build = |base: u64| -> Result<Vec<u8>, Error> {
        let mut p = Vec::new();
        for b in &primary.movie {
            if b.kind.as_bytes() != b"trak" {
                let mut raw = primary.data[b.box_range()].to_vec();
                if b.kind.as_bytes() == b"mvhd" {
                    let h = b.data_start - b.box_start;
                    let p = &mut raw[h..];
                    let n = selection
                        .iter()
                        .map(|(_, _, id)| *id)
                        .max()
                        .unwrap_or(0)
                        .checked_add(1)
                        .ok_or_else(err)?;
                    let tail = p.len().checked_sub(4).ok_or_else(err)?;
                    p[tail..].copy_from_slice(&n.to_be_bytes());
                    if sources.len() > 1 {
                        let movie_scale = primary
                            .model
                            .movie_timescale
                            .ok_or(Error::InvalidPresentation)?;
                        let mut longest = 0u64;
                        for (source_index, track, _) in selection {
                            let src = &sources[*source_index];
                            let tr = src
                                .movie
                                .iter()
                                .filter(|b| b.kind.as_bytes() == b"trak")
                                .nth(track.index)
                                .ok_or_else(err)?;
                            let tc = children(src.data, tr)?;
                            let h = tc
                                .iter()
                                .find(|v| v.kind.as_bytes() == b"tkhd")
                                .ok_or_else(err)?;
                            let mut payload = src.data[h.payload_range()].to_vec();
                            let at = if payload[0] == 0 { 20 } else { 28 };
                            let width = if payload[0] == 0 { 4 } else { 8 };
                            rescale_field(
                                &mut payload,
                                at,
                                width,
                                src.model
                                    .movie_timescale
                                    .ok_or(Error::InvalidPresentation)?,
                                movie_scale,
                            )?;
                            let duration = if width == 4 {
                                u64::from(u32_at(&payload, at)?)
                            } else {
                                u64::from_be_bytes(
                                    payload[at..at + 8].try_into().map_err(|_| err())?,
                                )
                            };
                            longest = longest.max(duration);
                        }
                        let at = if p[0] == 0 {
                            16
                        } else if p[0] == 1 {
                            24
                        } else {
                            return Err(err());
                        };
                        if p[0] == 0 {
                            let old = u32_at(p, at)?;
                            let duration = u32::try_from(longest.max(u64::from(old)))
                                .map_err(|_| Error::InvalidPresentation)?;
                            p[at..at + 4].copy_from_slice(&duration.to_be_bytes());
                        } else {
                            let old =
                                u64::from_be_bytes(p[at..at + 8].try_into().map_err(|_| err())?);
                            p[at..at + 8].copy_from_slice(&longest.max(old).to_be_bytes());
                        }
                    }
                }
                p.extend(raw);
            }
        }
        for (index, track, id) in selection {
            let s = &sources[*index];
            let b = s
                .movie
                .iter()
                .filter(|b| b.kind.as_bytes() == b"trak")
                .nth(track.index)
                .ok_or_else(err)?;
            let map = |offset: u64| -> Result<u64, Error> {
                let end = segments
                    .partition_point(|v| (v.source, v.range.lower_bound) <= (*index, offset));
                let segment = end
                    .checked_sub(1)
                    .and_then(|n| segments.get(n))
                    .filter(|v| v.source == *index && offset < v.range.upper_bound)
                    .ok_or_else(err)?;
                base.checked_add(segment.new_start)
                    .and_then(|n| n.checked_add(offset - segment.range.lower_bound))
                    .ok_or_else(err)
            };
            let scale = if *index == 0 {
                None
            } else {
                Some((
                    s.model.movie_timescale.ok_or(Error::InvalidPresentation)?,
                    primary
                        .model
                        .movie_timescale
                        .ok_or(Error::InvalidPresentation)?,
                ))
            };
            p.extend(rewrite(s.data, b, &map, Some(*id), scale)?);
        }
        boxed(b"moov", &p)
    };
    let provisional = build(0)?;
    let base = (ftyp.size + provisional.len() + 8) as u64;
    let moov = build(base)?;
    if provisional.len() != moov.len() {
        return Err(err());
    }
    let bytes = [
        primary.data[ftyp.box_range()].to_vec(),
        moov,
        boxed(b"mdat", &media)?,
    ]
    .concat();
    let output = crate::tracks::inspect(&bytes, ResourceId(0), false)?;
    if output.tracks.len() != selection.len() {
        return Err(err());
    }
    for ((source_index, old, id), new) in selection.iter().zip(&output.tracks) {
        if new.id != *id
            || new.timescale != old.timescale
            || new.duration != old.duration
            || new.sample_entries != old.sample_entries
        {
            return Err(err());
        }
        let old_ranges = crate::tracks::sample_ranges(sources[*source_index].data, old)?;
        let new_ranges = crate::tracks::sample_ranges(&bytes, new)?;
        if old_ranges.len() != new_ranges.len()
            || old_ranges.iter().zip(new_ranges).any(|(a, b)| {
                sources[*source_index].data[a.lower_bound as usize..a.upper_bound as usize]
                    != bytes[b.lower_bound as usize..b.upper_bound as usize]
            })
        {
            return Err(err());
        }
    }
    let _ = referenced;
    let sidecars = sources
        .iter()
        .enumerate()
        .skip(1)
        .flat_map(|(index, s)| {
            s.movie
                .iter()
                .filter(|b| matches!(b.kind.as_bytes(), b"udta" | b"meta"))
                .map(move |b| RemuxSidecar {
                    source: index,
                    kind: *b.kind.as_bytes(),
                    bytes: s.data[b.box_range()].to_vec(),
                })
        })
        .collect();
    Ok(RemuxedMovie {
        bytes,
        sidecars,
        report: RemuxReport {
            track_mappings: selection
                .iter()
                .map(|(source, t, id)| TrackMapping {
                    source: *source,
                    original_id: t.id,
                    output_id: *id,
                })
                .collect(),
            removed_track_ids: removed,
            discarded_unreferenced_media_bytes: discarded,
            retained_unreferenced_media_bytes: retained_unreferenced,
            discarded_padding_bytes: sources
                .iter()
                .map(|s| {
                    s.top
                        .iter()
                        .filter(|b| matches!(b.kind.as_bytes(), b"free" | b"skip" | b"wide"))
                        .map(|b| b.size as u64)
                        .sum::<u64>()
                })
                .sum(),
            fast_start: true,
            encoded_samples_unchanged: true,
        },
    })
}
/// Fast-start plus optional explicit compaction; encoded data, sample tables,
/// display properties and exact timing remain intact. Unlike mp4v2's referenced
/// optimizer this implementation can relocate hvc1 chunks without decoding.
pub fn optimize(movie: &[u8], options: OptimizeOptions) -> Result<RemuxedMovie, Error> {
    let s = source(movie)?;
    let selected = s
        .model
        .tracks
        .iter()
        .map(|t| (0, t, t.id))
        .collect::<Vec<_>>();
    compose(std::slice::from_ref(&s), &selected, Vec::new(), options)
}
pub fn optimization_support(movie: &[u8]) -> Result<(), Error> {
    optimize(movie, OptimizeOptions::default()).map(|_| ())
}
pub fn track_summary(movie: &[u8]) -> Result<MediaContainer, Error> {
    crate::tracks::inspect(movie, ResourceId(0), false)
}
/// Explicitly remove arbitrary selected track IDs. Track references are rejected
/// rather than retained as dangling links. Unreferenced media remains by default.
pub fn remove_tracks(
    movie: &[u8],
    ids: &[u32],
    options: OptimizeOptions,
) -> Result<RemuxedMovie, Error> {
    let s = source(movie)?;
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) || !s.model.tracks.iter().any(|t| t.id == *id) {
            return Err(err());
        }
    }
    let selected = s
        .model
        .tracks
        .iter()
        .filter(|t| !ids.contains(&t.id))
        .map(|t| (0, t, t.id))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(Error::UnsupportedLayout("cannot remove every track"));
    }
    compose(std::slice::from_ref(&s), &selected, ids.to_vec(), options)
}
/// Import selected tracks; original destination tracks retain their IDs. Imported
/// IDs are allocated deterministically. Source movie metadata is returned as
/// sidecars; track references are rejected. Edit durations rescale exactly or fail.
pub fn import_tracks(
    destination: &[u8],
    from: &[u8],
    ids: &[u32],
    options: OptimizeOptions,
) -> Result<RemuxedMovie, Error> {
    if destination
        .len()
        .checked_add(from.len())
        .is_none_or(|n| n > MAX_BYTES)
    {
        return Err(Error::InputLimit);
    }
    let sources = [source(destination)?, source(from)?];
    let a = &sources[0];
    let b = &sources[1];
    let mut selected = a
        .model
        .tracks
        .iter()
        .map(|t| (0, t, t.id))
        .collect::<Vec<_>>();
    let mut next = a.model.tracks.iter().map(|t| t.id).max().unwrap_or(0);
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return Err(err());
        }
        let t = b
            .model
            .tracks
            .iter()
            .find(|t| t.id == *id)
            .ok_or_else(err)?;
        next = next.checked_add(1).ok_or_else(err)?;
        selected.push((1, t, next));
    }
    compose(&sources, &selected, Vec::new(), options)
}
