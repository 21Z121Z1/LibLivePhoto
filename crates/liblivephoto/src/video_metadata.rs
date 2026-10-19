//! Read existing Google score/stabilization descriptors. This module does not
//! create Moments, calculate scores or author a generated metadata track.
use crate::{MediaTime, MotionPhotoReadError as Error};
#[derive(Debug, Clone, PartialEq)]
pub struct FrameScore {
    pub score: f32,
    pub encoded_score: u32,
    pub timestamp_us: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDescriptor {
    pub parent: String,
    pub tag: u8,
    pub payload: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct VideoMetadata {
    pub raw_bytes: Vec<u8>,
    pub model_version: Option<u32>,
    pub primary_score: Option<FrameScore>,
    pub high_resolution_scores: Option<Vec<FrameScore>>,
    pub low_resolution_flags: Option<u8>,
    pub high_resolution_flags: Option<u8>,
    pub unknown: Vec<UnknownDescriptor>,
    pub trailing_bytes: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoTimeExpectation {
    Unchecked,
    Unknown,
    Known(MediaTime),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoMetadataIssue {
    ScoreOutOfRange { index: Option<usize> },
    InvalidTimestamp { index: Option<usize> },
    UnsortedScores,
    MissingPrimaryTimestamp,
    PhotoTimeMismatch,
}
struct Descriptor<'a> {
    tag: u8,
    bytes: &'a [u8],
}
fn descriptor<'a>(data: &mut &'a [u8]) -> Result<Descriptor<'a>, Error> {
    let tag = *data.first().ok_or(Error::InvalidResource)?;
    let mut size = 0usize;
    for i in 1..=4 {
        let byte = *data.get(i).ok_or(Error::InvalidResource)?;
        size = size
            .checked_mul(128)
            .and_then(|n| n.checked_add(usize::from(byte & 127)))
            .ok_or(Error::InvalidResource)?;
        if byte & 128 == 0 {
            let end = (i + 1)
                .checked_add(size)
                .filter(|e| *e <= data.len())
                .ok_or(Error::InvalidResource)?;
            let bytes = &data[i + 1..end];
            *data = &data[end..];
            return Ok(Descriptor { tag, bytes });
        }
    }
    Err(Error::InvalidResource)
}
fn frame(d: &Descriptor<'_>) -> Result<FrameScore, Error> {
    if d.bytes.len() != 12 {
        return Err(Error::MetadataMismatch("frame-score descriptor length"));
    }
    let encoded = u32::from_be_bytes(
        d.bytes[..4]
            .try_into()
            .map_err(|_| Error::InvalidResource)?,
    );
    Ok(FrameScore {
        score: f32::from_bits(encoded),
        encoded_score: encoded,
        timestamp_us: i64::from_be_bytes(
            d.bytes[4..]
                .try_into()
                .map_err(|_| Error::InvalidResource)?,
        ),
    })
}
fn unknown(out: &mut VideoMetadata, parent: &str, d: Descriptor<'_>) {
    out.unknown.push(UnknownDescriptor {
        parent: parent.into(),
        tag: d.tag,
        payload: d.bytes.to_vec(),
    });
}
/// All input bytes and unrecognized descriptors are retained. Known containers
/// and field counts must be complete; absent values stay absent rather than the
/// pinned C++ decoder's default objects. Semantic checking is separate below.
pub fn decode_video_metadata(bytes: &[u8]) -> Result<VideoMetadata, Error> {
    if bytes.len() > 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    let mut data = bytes;
    let root = descriptor(&mut data)?;
    if root.tag != 0xc0 {
        return Err(Error::MetadataMismatch("Google video metadata root tag"));
    }
    let mut out = VideoMetadata {
        raw_bytes: bytes.to_vec(),
        model_version: None,
        primary_score: None,
        high_resolution_scores: None,
        low_resolution_flags: None,
        high_resolution_flags: None,
        unknown: Vec::new(),
        trailing_bytes: data.to_vec(),
    };
    let mut children = root.bytes;
    let mut flags_seen = false;
    while !children.is_empty() {
        let d = descriptor(&mut children)?;
        match d.tag {
            0xc1 => {
                if out.model_version.is_some() || d.bytes.len() < 4 {
                    return Err(Error::MetadataMismatch(
                        "duplicate/truncated score descriptor",
                    ));
                }
                out.model_version = Some(u32::from_be_bytes(
                    d.bytes[..4]
                        .try_into()
                        .map_err(|_| Error::InvalidResource)?,
                ));
                let mut scores = &d.bytes[4..];
                while !scores.is_empty() {
                    let s = descriptor(&mut scores)?;
                    match s.tag {
                        0xc3 => {
                            if out.primary_score.is_some() {
                                return Err(Error::MetadataMismatch("duplicate primary score"));
                            }
                            out.primary_score = Some(frame(&s)?);
                        }
                        0xc2 => {
                            if out.high_resolution_scores.is_some() || s.bytes.len() < 4 {
                                return Err(Error::MetadataMismatch(
                                    "duplicate/truncated track scores",
                                ));
                            }
                            let count = u32::from_be_bytes(
                                s.bytes[..4]
                                    .try_into()
                                    .map_err(|_| Error::InvalidResource)?,
                            ) as usize;
                            let mut list = &s.bytes[4..];
                            if count > 100_000 || count > list.len() / 2 {
                                return Err(Error::InputLimit);
                            }
                            let mut frames = Vec::new();
                            for _ in 0..count {
                                let f = descriptor(&mut list)?;
                                if f.tag == 0xc3 {
                                    frames.push(frame(&f)?);
                                } else {
                                    unknown(&mut out, "trackScores", f);
                                }
                            }
                            if !list.is_empty() {
                                return Err(Error::MetadataMismatch("track score count"));
                            }
                            out.high_resolution_scores = Some(frames);
                        }
                        _ => unknown(&mut out, "scores", s),
                    }
                }
            }
            0xc4 => {
                if flags_seen {
                    return Err(Error::MetadataMismatch("duplicate metadata flags"));
                }
                flags_seen = true;
                let mut flags = d.bytes;
                let mut index = 0;
                while !flags.is_empty() {
                    let f = descriptor(&mut flags)?;
                    if f.tag == 0xc5 {
                        if f.bytes.len() != 1 || index > 1 {
                            return Err(Error::MetadataMismatch("track flag count/length"));
                        }
                        if index == 0 {
                            out.low_resolution_flags = Some(f.bytes[0]);
                        } else {
                            out.high_resolution_flags = Some(f.bytes[0]);
                        }
                        index += 1;
                    } else {
                        unknown(&mut out, "flags", f);
                    }
                }
            }
            _ => unknown(&mut out, "metadata", d),
        }
    }
    Ok(out)
}
impl VideoMetadata {
    pub fn low_resolution_stabilized(&self) -> Option<bool> {
        self.low_resolution_flags.map(|b| b & 0x80 != 0)
    }
    pub fn high_resolution_stabilized(&self) -> Option<bool> {
        self.high_resolution_flags.map(|b| b & 0x80 != 0)
    }
    pub fn validate(&self, expected: PhotoTimeExpectation) -> Vec<VideoMetadataIssue> {
        let mut issues = Vec::new();
        let mut check = |s: &FrameScore, index| {
            if !s.score.is_finite() || !(0.0..=1.0).contains(&s.score) {
                issues.push(VideoMetadataIssue::ScoreOutOfRange { index });
            }
            if s.timestamp_us < -1 {
                issues.push(VideoMetadataIssue::InvalidTimestamp { index });
            }
        };
        if let Some(s) = &self.primary_score {
            check(s, None);
        }
        if let Some(scores) = &self.high_resolution_scores {
            for (i, s) in scores.iter().enumerate() {
                check(s, Some(i));
            }
        }
        if self
            .high_resolution_scores
            .as_ref()
            .is_some_and(|s| s.windows(2).any(|w| w[0].timestamp_us > w[1].timestamp_us))
        {
            issues.push(VideoMetadataIssue::UnsortedScores);
        }
        match expected {
            PhotoTimeExpectation::Unchecked => {}
            PhotoTimeExpectation::Unknown => {
                if self
                    .primary_score
                    .as_ref()
                    .is_some_and(|s| s.timestamp_us != -1)
                {
                    issues.push(VideoMetadataIssue::PhotoTimeMismatch);
                }
            }
            PhotoTimeExpectation::Known(time) => match self.primary_score.as_ref() {
                None => issues.push(VideoMetadataIssue::MissingPrimaryTimestamp),
                Some(s) => {
                    if !(MediaTime {
                        value: s.timestamp_us,
                        timescale: 1_000_000,
                    })
                    .equivalent(time)
                    {
                        issues.push(VideoMetadataIssue::PhotoTimeMismatch);
                    }
                }
            },
        }
        issues
    }
}
