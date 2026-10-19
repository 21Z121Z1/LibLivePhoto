//! Explicit, offset-stable metadata-track deletion. The output retains unused
//! media bytes. It is not compaction or fast-start optimization.
use crate::{MotionPhotoReadError as Error, ResourceId, TrackKind};
use liblivephoto_format::{
    isobmff::{make_box, parse_boxes},
    FourCC,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataRemoval {
    pub bytes: Vec<u8>,
    pub removed_track_ids: Vec<u32>,
    pub retained_track_ids: Vec<u32>,
    /// Deleted sample payloads remain unreferenced in their original mdat.
    pub unreferenced_media_retained: bool,
}

/// Remove only explicitly selected metadata tracks. An empty selection copies
/// the original file. Reject fragmented movies, unknown moov children, and track
/// dependencies because those need additional reference rewriting. Extraction
/// and Motion Photo construction never call this function implicitly.
pub fn remove_metadata_tracks(movie: &[u8], track_ids: &[u32]) -> Result<MetadataRemoval, Error> {
    let model = crate::tracks::inspect(movie, ResourceId(1), false)?;
    let top = parse_boxes(movie, 0..movie.len()).map_err(|_| Error::InvalidResource)?;
    if top
        .iter()
        .any(|b| matches!(b.kind.as_bytes(), b"moof" | b"sidx" | b"mfra"))
    {
        return Err(Error::UnsupportedLayout(
            "metadata track removal does not support fragmented/indexed movies",
        ));
    }
    let moovs: Vec<_> = top
        .iter()
        .filter(|b| b.kind.as_bytes() == b"moov")
        .collect();
    if moovs.len() != 1 {
        return Err(Error::InvalidResource);
    }
    for (i, id) in track_ids.iter().enumerate() {
        if track_ids[..i].contains(id)
            || !model
                .tracks
                .iter()
                .any(|t| t.id == *id && t.kind == TrackKind::Metadata)
        {
            return Err(Error::UnsupportedLayout(
                "removal selection must contain unique metadata track IDs",
            ));
        }
    }
    let mut result = MetadataRemoval {
        bytes: movie.to_vec(),
        removed_track_ids: track_ids.to_vec(),
        retained_track_ids: model
            .tracks
            .iter()
            .filter(|t| !track_ids.contains(&t.id))
            .map(|t| t.id)
            .collect(),
        unreferenced_media_retained: !track_ids.is_empty(),
    };
    if track_ids.is_empty() {
        return Ok(result);
    }
    if !model
        .tracks
        .iter()
        .any(|t| t.kind == TrackKind::Video && !track_ids.contains(&t.id))
    {
        return Err(Error::UnsupportedLayout(
            "metadata removal requires a retained video track",
        ));
    }
    let moov = moovs[0];
    let children = parse_boxes(movie, moov.payload_range()).map_err(|_| Error::InvalidResource)?;
    let mut payload = Vec::new();
    let mut index = 0;
    for b in children {
        match b.kind.as_bytes() {
            b"trak" => {
                let track = model.tracks.get(index).ok_or(Error::InvalidResource)?;
                index += 1;
                let nested =
                    parse_boxes(movie, b.payload_range()).map_err(|_| Error::InvalidResource)?;
                if nested.iter().any(|c| c.kind.as_bytes() == b"tref") {
                    return Err(Error::UnsupportedLayout(
                        "metadata removal requires explicit handling of track references",
                    ));
                }
                if track_ids.contains(&track.id) {
                    continue;
                }
            }
            b"mvhd" | b"udta" | b"meta" | b"free" | b"skip" => {}
            _ => {
                return Err(Error::UnsupportedLayout(
                    "metadata removal cannot preserve an unknown movie-level reference contract",
                ))
            }
        }
        payload.extend_from_slice(&movie[b.box_range()]);
    }
    result.bytes[moov.box_start + 4..moov.box_start + 8].copy_from_slice(b"free");
    result.bytes[moov.data_start..moov.data_end].fill(0);
    // Preserve the meaning of a formerly final size-zero mdat before appending.
    for b in &top {
        if movie[b.box_start..b.box_start + 4] == [0; 4] {
            let size = u32::try_from(b.size).map_err(|_| Error::InvalidResource)?;
            result.bytes[b.box_start..b.box_start + 4].copy_from_slice(&size.to_be_bytes());
        }
    }
    result.bytes.extend_from_slice(
        &make_box(FourCC::new(*b"moov"), &payload).map_err(|_| Error::InvalidResource)?,
    );
    let after = crate::tracks::inspect(&result.bytes, ResourceId(1), false)?;
    for source in model.tracks.iter().filter(|t| !track_ids.contains(&t.id)) {
        let retained = after
            .tracks
            .iter()
            .find(|t| t.id == source.id)
            .ok_or(Error::InvalidResource)?;
        let mut expected = source.clone();
        expected.index = retained.index;
        if expected != *retained {
            return Err(Error::InvalidResource);
        }
    }
    Ok(result)
}
