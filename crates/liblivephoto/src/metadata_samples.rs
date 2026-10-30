//! Metadata samples are associated with actual tracks and sample tables. A byte
//! pattern anywhere in mdat does not establish a metadata kind or pixel meaning.
use crate::{
    MediaTime, MotionPhoto, MotionPhotoReadError as Error, PhotoTimeExpectation, ResourceId,
    TrackKind, VideoMetadata, VideoMetadataIssue,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataPacket<'a> {
    pub track_id: u32,
    pub sample_index: usize,
    pub sample_entry: [u8; 4],
    pub description_bytes: &'a [u8],
    pub presentation: MediaTime,
    pub duration: MediaTime,
    pub bytes: &'a [u8],
}
/// Raw packets in decode order. Presentation/duration use the track media
/// timeline (including composition offsets), before any edit list. Packet bytes
/// and embedded timestamps are unchanged, including for non-unit edits.
/// Fragmented samples are unsupported. For edited movie-time AGTM selection,
/// use `extract_agtm_from_video` instead.
pub fn metadata_packets(video: &[u8], track_id: u32) -> Result<Vec<MetadataPacket<'_>>, Error> {
    let container = crate::tracks::inspect(video, ResourceId(0), false)?;
    let track = container
        .tracks
        .iter()
        .find(|t| t.id == track_id && !matches!(t.kind, TrackKind::Video | TrackKind::Audio))
        .ok_or(Error::InvalidResource)?;
    Ok(crate::tracks::metadata_samples(video, track)?
        .into_iter()
        .map(|p| MetadataPacket {
            track_id,
            sample_index: p.index,
            sample_entry: p.description,
            description_bytes: p.description_bytes,
            presentation: p.presentation,
            duration: MediaTime {
                value: i64::from(p.duration),
                timescale: track.timescale,
            },
            bytes: p.bytes,
        })
        .collect())
}
#[derive(Debug, Clone, PartialEq)]
pub struct ScoringMetadata {
    pub track_id: u32,
    pub sample_index: usize,
    pub metadata: VideoMetadata,
    pub issues: Vec<VideoMetadataIssue>,
}
/// Decode only declared Google samples, bounded across all tracks to 4096
/// samples, 16 MiB of referenced payload and 65536 semantic records. Repeated
/// references to the same mdat bytes each consume the budget.
/// Payload-internal frame timestamps are validated as declared movie presentation
/// times, not shifted by the metadata track's ctts or edits. For caller-declared
/// pre-edit video media times, use `project_scoring_media_times` explicitly.
pub fn extract_video_metadata(
    video: &[u8],
    expected: PhotoTimeExpectation,
) -> Result<Vec<ScoringMetadata>, Error> {
    let container = crate::tracks::inspect(video, ResourceId(0), false)?;
    let mut result = Vec::new();
    let (mut payload_bytes, mut records) = (0usize, 0usize);
    for track in container.tracks.iter().filter(|t| {
        !matches!(t.kind, TrackKind::Video | TrackKind::Audio) && t.sample_entries.contains(b"mett")
    }) {
        // Test the actual sample description before interpreting the sample bytes.
        for packet in crate::tracks::metadata_samples(video, track)? {
            const MIME: &[u8] = b"application/motionphoto-image-meta";
            if packet.description != *b"mett" {
                continue;
            }
            let description = packet
                .description_bytes
                .get(8..)
                .ok_or(Error::InvalidResource)?;
            let encoding_end = description
                .iter()
                .position(|b| *b == 0)
                .ok_or(Error::InvalidResource)?;
            let rest = &description[encoding_end + 1..];
            let mime_end = rest
                .iter()
                .position(|b| *b == 0)
                .ok_or(Error::InvalidResource)?;
            if &rest[..mime_end] != MIME {
                continue;
            }
            let legacy = packet.description_bytes.get(..8) == Some(b"applicat");
            if encoding_end > 0 && !legacy {
                return Err(Error::UnsupportedLayout(
                    "compressed motion-photo metadata sample",
                ));
            }
            payload_bytes = payload_bytes
                .checked_add(packet.bytes.len())
                .filter(|n| *n <= 16 * 1024 * 1024)
                .ok_or(Error::InputLimit)?;
            if result.len() >= 4096 {
                return Err(Error::InputLimit);
            }
            let metadata = crate::decode_video_metadata(packet.bytes).map_err(|e| match e {
                Error::InputLimit => e,
                _ => Error::MetadataMismatch("invalid declared Google video metadata"),
            })?;
            records = records
                .checked_add(
                    metadata.unknown.len()
                        + metadata.high_resolution_scores.as_ref().map_or(0, Vec::len)
                        + usize::from(metadata.primary_score.is_some()),
                )
                .filter(|n| *n <= 65_536)
                .ok_or(Error::InputLimit)?;
            let issues = metadata.validate(expected);
            result.push(ScoringMetadata {
                track_id: track.id,
                sample_index: packet.index,
                metadata,
                issues,
            });
        }
    }
    Ok(result)
}
impl MotionPhoto<'_> {
    pub fn metadata_packets(&self, track_id: u32) -> Result<Vec<MetadataPacket<'_>>, Error> {
        metadata_packets(self.motion_video_bytes()?, track_id)
    }
    pub fn scoring_metadata(&self) -> Result<Vec<ScoringMetadata>, Error> {
        extract_video_metadata(
            self.motion_video_bytes()?,
            self.presentation_time()
                .map_or(PhotoTimeExpectation::Unknown, PhotoTimeExpectation::Known),
        )
    }
}
