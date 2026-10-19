//! Explicit projection of existing scoring fields from video media CTS to movie
//! time. This never guesses a time domain, authors descriptors or changes bytes.
use crate::{
    FrameScore, MediaTime, MotionPhotoReadError as Error, ResourceId, TrackKind, VideoMetadata,
};
use std::collections::{BTreeMap, BTreeSet};

/// Caller-declared association of the payload's primary/high-resolution scores
/// with video tracks. Passing these IDs asserts that the supplied fields are
/// media CTS before those tracks' edits. The default Google scoring reader does
/// not make that assertion: it checks declared movie times against XMP directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoringMediaTracks {
    pub primary: u32,
    pub high_resolution: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreTimeDisposition {
    Mapped,
    Trimmed,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedFrameScore<'a> {
    pub source: &'a FrameScore,
    pub disposition: ScoreTimeDisposition,
    /// Exact movie times at 1000000 ticks/second, sorted and unique. A repeated
    /// edit can map one source score to multiple times; none is chosen for you.
    pub movie_times: Vec<MediaTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoringTimeProjection<'a> {
    pub source: &'a VideoMetadata,
    pub primary: Option<ProjectedFrameScore<'a>>,
    pub high_resolution: Option<Vec<ProjectedFrameScore<'a>>>,
}

fn collect_needed(scores: impl Iterator<Item = i64>) -> Result<BTreeSet<i64>, Error> {
    scores
        .filter(|t| *t != -1)
        .map(|t| {
            if t < 0 {
                Err(Error::InvalidPresentation)
            } else {
                Ok(t)
            }
        })
        .collect()
}

/// Only unit-rate, self-contained, non-fragmented video tracks are projected.
/// Every specified source time must match a sample CTS exactly, including ctts;
/// edits are then applied by source sample index, once. Empty/trim/repeated edits
/// remain explicit. No nearest-frame choice, float conversion or score rewriting.
/// Bounds:1MiB raw payload,65536 score/unknown records,100000 aggregate source
/// frames and100000 aggregate edited frames, and min(max_output_times,65536)
/// total projected times. The output budget is checked before per-score copies.
pub fn project_scoring_media_times<'a>(
    video: &[u8],
    metadata: &'a VideoMetadata,
    tracks: ScoringMediaTracks,
    max_output_times: usize,
) -> Result<ScoringTimeProjection<'a>, Error> {
    let high = metadata.high_resolution_scores.as_deref().unwrap_or(&[]);
    let records = high
        .len()
        .checked_add(usize::from(metadata.primary_score.is_some()))
        .and_then(|n| n.checked_add(metadata.unknown.len()))
        .ok_or(Error::InputLimit)?;
    if video.len() > 256 * 1024 * 1024 || metadata.raw_bytes.len() > 1024 * 1024 || records > 65536
    {
        return Err(Error::InputLimit);
    }
    let high_id = if high.is_empty() {
        tracks.high_resolution
    } else {
        Some(tracks.high_resolution.ok_or(Error::UnsupportedLayout(
            "high-resolution score video-track association required",
        ))?)
    };
    let mut needed = BTreeMap::<u32, BTreeSet<i64>>::new();
    needed.insert(
        tracks.primary,
        collect_needed(metadata.primary_score.iter().map(|s| s.timestamp_us))?,
    );
    if let Some(id) = high_id {
        needed
            .entry(id)
            .or_default()
            .extend(collect_needed(high.iter().map(|s| s.timestamp_us))?);
    }
    let container = crate::tracks::inspect(video, ResourceId(0), false)?;
    let mut maps = BTreeMap::<u32, BTreeMap<i64, BTreeSet<i64>>>::new();
    let (mut raw_budget, mut edited_budget) = (100000usize, 100000usize);
    for (id, wanted) in needed {
        let track = container
            .tracks
            .iter()
            .find(|t| t.id == id && t.kind == TrackKind::Video)
            .ok_or(Error::InvalidResource)?;
        let wanted_ticks: BTreeMap<_, _> = wanted
            .into_iter()
            .map(|us| {
                let tick = MediaTime {
                    value: us,
                    timescale: 1000000,
                }
                .rescale_exact(track.timescale)
                .ok_or(Error::InvalidPresentation)?
                .value;
                Ok((tick, us))
            })
            .collect::<Result<_, Error>>()?;
        let raw = crate::tracks::video_frames_unedited(video, id, raw_budget)?;
        raw_budget -= raw.len();
        let mut indices = BTreeMap::new();
        let mut times = BTreeMap::new();
        for frame in raw {
            if let Some(us) = wanted_ticks.get(&frame.time.value) {
                indices.insert(frame.sample_index, *us);
                times.entry(*us).or_insert_with(BTreeSet::new);
            }
        }
        if times.len() != wanted_ticks.len() {
            return Err(Error::MetadataMismatch(
                "score time does not match a source video frame CTS",
            ));
        }
        let edited = crate::tracks::video_frames(video, id, edited_budget)?;
        edited_budget -= edited.len();
        for frame in edited {
            if let Some(us) = indices.get(&frame.sample_index) {
                let movie_us = frame
                    .time
                    .rescale_exact(1000000)
                    .ok_or(Error::InvalidPresentation)?
                    .value;
                times
                    .get_mut(us)
                    .ok_or(Error::InvalidPresentation)?
                    .insert(movie_us);
            }
        }
        maps.insert(id, times);
    }
    let count = |id: u32, score: &FrameScore| -> Result<usize, Error> {
        if score.timestamp_us == -1 {
            return Ok(0);
        }
        maps.get(&id)
            .and_then(|m| m.get(&score.timestamp_us))
            .map(BTreeSet::len)
            .ok_or(Error::InvalidPresentation)
    };
    let mut projected_count = 0usize;
    for (id, score) in metadata
        .primary_score
        .iter()
        .map(|s| (tracks.primary, s))
        .chain(high.iter().map(|s| (high_id.unwrap_or(tracks.primary), s)))
    {
        projected_count = projected_count
            .checked_add(count(id, score)?)
            .filter(|n| *n <= max_output_times.min(65536))
            .ok_or(Error::InputLimit)?;
    }
    let project = |id: u32, score: &'a FrameScore| -> Result<ProjectedFrameScore<'a>, Error> {
        let movie_times: Vec<_> = if score.timestamp_us == -1 {
            Vec::new()
        } else {
            maps.get(&id)
                .and_then(|m| m.get(&score.timestamp_us))
                .ok_or(Error::InvalidPresentation)?
                .iter()
                .map(|us| MediaTime {
                    value: *us,
                    timescale: 1000000,
                })
                .collect()
        };
        let disposition = if score.timestamp_us == -1 {
            ScoreTimeDisposition::Unspecified
        } else if movie_times.is_empty() {
            ScoreTimeDisposition::Trimmed
        } else {
            ScoreTimeDisposition::Mapped
        };
        Ok(ProjectedFrameScore {
            source: score,
            disposition,
            movie_times,
        })
    };
    Ok(ScoringTimeProjection {
        source: metadata,
        primary: metadata
            .primary_score
            .as_ref()
            .map(|s| project(tracks.primary, s))
            .transpose()?,
        high_resolution: metadata
            .high_resolution_scores
            .as_ref()
            .map(|scores| {
                scores
                    .iter()
                    .map(|s| project(high_id.unwrap_or(tracks.primary), s))
                    .collect()
            })
            .transpose()?,
    })
}
