//! Optional declarations/timeline subset of Google's public Motion Photo1.0
//! "Video container contents" section. No bitstream or device certification.
use crate::{
    FormatConformanceReport, FormatIssue, FormatIssueKind as Kind, MediaTime, MotionPhoto,
    MotionPhotoReadError as Error, ResourceId, TrackKind,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoogleVideoTracks {
    /// Explicit track identity, never inferred from a vendor's opaque role tag.
    pub primary: u32,
    pub secondary: Option<u32>,
}

fn issue(
    report: &mut FormatConformanceReport,
    rule: &'static str,
    kind: Kind,
    track: Option<u32>,
    message: impl Into<String>,
) -> Result<(), Error> {
    report.issue(FormatIssue {
        rule,
        kind,
        item_id: None,
        track_id: track,
        message: message.into(),
    })
}

fn rational(time: MediaTime) -> (i64, u32) {
    let (mut a, mut b) = (time.value.unsigned_abs(), u64::from(time.timescale));
    while b != 0 {
        (a, b) = (b, a % b);
    }
    (
        (i128::from(time.value) / i128::from(a)) as i64,
        (u64::from(time.timescale) / a) as u32,
    )
}

/// Check only listed declaration/count/order rules and the recommended exact
/// secondary-to-primary timestamp correspondence. IDs are selected by the caller;
/// every video track must be accounted for. Dimensions remain observed facts:
/// the source defines no fixed primary/secondary pixel dimensions. No nearest
/// timestamp fallback is selected. All sample descriptions are checked.
/// AAC PCE, QuickTime-v2/wave, unknown codec entries and the page's ambiguous
///44kHz versus44.1kHz wording stay explicit Unsupported.48000/96000 are exact;
///44000 is the literal printed value, not an inferred44100 alias.
/// Bounds:256MiB input,4096 descriptions/issues,16MiB properties,100000 aggregate
/// edited video-frame timestamps; underlying sample/edit work remains bounded.
pub fn validate_google_video_container(
    video: &[u8],
    selected: GoogleVideoTracks,
) -> Result<FormatConformanceReport, Error> {
    if video.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    let container = crate::tracks::inspect(video, ResourceId(0), false)?;
    let mut report = FormatConformanceReport {
        profile: "google-video-container-1.0-declarations",
        checked_rules: vec![
            "G08.primary-video",
            "G08.audio-count",
            "G09.video-count",
            "G09.track-association",
            "G09.secondary-order",
        ],
        issues: Vec::new(),
    };
    let videos: Vec<_> = container
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Video)
        .collect();
    if videos.is_empty() {
        issue(
            &mut report,
            "G08.primary-video",
            Kind::Violation,
            None,
            "a primary video track is required",
        )?;
        return Ok(report);
    }
    if videos.len() > 2 {
        issue(
            &mut report,
            "G09.video-count",
            Kind::Violation,
            None,
            "more than one secondary video track is outside this Google profile",
        )?;
    }
    let primary = videos
        .iter()
        .find(|t| t.id == selected.primary)
        .ok_or(Error::InvalidResource)?;
    if selected.secondary == Some(selected.primary) {
        return Err(Error::InvalidResource);
    }
    let secondary = selected
        .secondary
        .map(|id| {
            videos
                .iter()
                .find(|t| t.id == id)
                .copied()
                .ok_or(Error::InvalidResource)
        })
        .transpose()?;
    for track in &videos {
        if track.id != primary.id && Some(track.id) != selected.secondary {
            issue(
                &mut report,
                "G09.track-association",
                Kind::Unsupported,
                Some(track.id),
                "video track is not identified by the caller's primary/secondary association",
            )?;
        }
    }
    if let Some(secondary) = secondary {
        if secondary.index <= primary.index {
            issue(
                &mut report,
                "G09.secondary-order",
                Kind::Violation,
                Some(secondary.id),
                "secondary track index must follow the primary track index",
            )?;
        }
    }
    let audios: Vec<_> = container
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .collect();
    if audios.len() > 1 {
        issue(
            &mut report,
            "G08.audio-count",
            Kind::Violation,
            None,
            "the Google profile allows at most one audio track",
        )?;
    }
    let descriptions = match crate::inspect_video_sample_descriptions(video) {
        Ok(d) => d,
        Err(Error::UnsupportedLayout(reason)) => {
            issue(
                &mut report,
                "G08.declarations",
                Kind::Unsupported,
                None,
                reason,
            )?;
            return Ok(report);
        }
        Err(error) => return Err(error),
    };
    report.checked_rules.extend([
        "G08.video-codec",
        "G08.audio-codec",
        "G08.audio-depth",
        "G08.audio-channels",
        "G08.audio-rate",
    ]);
    for track in videos.iter().copied().chain(audios.iter().copied()) {
        if !descriptions.iter().any(|d| d.track_id == track.id) {
            issue(
                &mut report,
                "G08.declarations",
                Kind::Unsupported,
                Some(track.id),
                "sample descriptions unavailable",
            )?;
        }
    }
    for desc in &descriptions {
        let id = Some(desc.track_id);
        if desc.kind == TrackKind::Video {
            match &desc.format {
                b"avc1" | b"avc3" | b"hvc1" | b"hev1" | b"av01" => {}
                b"vp08" | b"vp09" | b"mp4v" | b"jpeg" | b"png " | b"raw " => issue(
                    &mut report,
                    "G08.video-codec",
                    Kind::Violation,
                    id,
                    "declared video codec is not AVC/HEVC/AV1",
                )?,
                _ => issue(
                    &mut report,
                    "G08.video-codec",
                    Kind::Unsupported,
                    id,
                    "video sample-entry codec family is not established",
                )?,
            }
        }
        if desc.kind != TrackKind::Audio {
            continue;
        }
        if desc.format != *b"mp4a" {
            issue(
                &mut report,
                "G08.audio-codec",
                if matches!(
                    &desc.format,
                    b"alac" | b"lpcm" | b"sowt" | b"twos" | b"Opus" | b"ac-3" | b"ec-3"
                ) {
                    Kind::Violation
                } else {
                    Kind::Unsupported
                },
                id,
                "audio declaration does not establish AAC",
            )?;
            continue;
        }
        let Some(audio) = &desc.audio else {
            issue(
                &mut report,
                "G08.audio-declarations",
                Kind::Unsupported,
                id,
                "audio sample-entry fields unavailable",
            )?;
            continue;
        };
        if desc.properties.iter().any(|p| p.kind == *b"wave") {
            issue(
                &mut report,
                "G08.audio-declarations",
                Kind::Unsupported,
                id,
                "QuickTime wave signaling is not interpreted, including mixed direct esds/wave",
            )?;
            continue;
        }
        if audio.sample_entry_version > 1 {
            issue(
                &mut report,
                "G08.audio-declarations",
                Kind::Unsupported,
                id,
                "QuickTime v2 audio layout is not interpreted",
            )?;
            continue;
        }
        if audio.sample_entry_sample_size != 16 {
            issue(&mut report, "G08.audio-depth", Kind::Violation, id, "audio sample-size declaration is not16 bits; decoded source depth is not certified")?;
        }
        let Some(aac) = &audio.aac else {
            issue(
                &mut report,
                "G08.audio-codec",
                Kind::Unsupported,
                id,
                "AAC AudioSpecificConfig is absent or uses unsupported esds/wave signaling",
            )?;
            continue;
        };
        if !matches!(aac.object_type, 2 | 5 | 29) {
            issue(
                &mut report,
                "G08.audio-codec",
                Kind::Unsupported,
                id,
                "AudioSpecificConfig object type is outside this checker's interpreted AAC subset",
            )?;
        }
        match aac.channels {
            Some(1 | 2) => {}
            Some(_) => issue(
                &mut report,
                "G08.audio-channels",
                Kind::Violation,
                id,
                "AAC configuration is neither mono nor stereo",
            )?,
            None => issue(
                &mut report,
                "G08.audio-channels",
                Kind::Unsupported,
                id,
                "AAC PCE channel configuration is not interpreted",
            )?,
        }
        match aac.sample_rate {
            44000 | 48000 | 96000 => {}
            44100 => issue(
                &mut report,
                "G08.audio-rate",
                Kind::Unsupported,
                id,
                "public page prints44kHz; this checker does not silently reinterpret it as44100Hz",
            )?,
            _ => issue(
                &mut report,
                "G08.audio-rate",
                Kind::Violation,
                id,
                "AAC rate is outside the printed44/48/96kHz set",
            )?,
        }
    }
    // Validate exact supported timing even without a secondary track, so a
    // fragmented/non-unit layout cannot accidentally receive an unqualified pass.
    let mut budget = 100000usize;
    let mut primary_times = BTreeSet::new();
    let mut primary_timing_known = false;
    report
        .checked_rules
        .push("G09.frame-correspondence-recommendation");
    for (track, is_primary) in [(Some(*primary), true), (secondary, false)] {
        let Some(track) = track else {
            continue;
        };
        let frames = match crate::video_frame_times(video, track.id, budget) {
            Ok(frames) => frames,
            Err(Error::UnsupportedLayout(reason)) => {
                issue(
                    &mut report,
                    "G09.frame-correspondence-recommendation",
                    Kind::Unsupported,
                    Some(track.id),
                    reason,
                )?;
                continue;
            }
            Err(error) => return Err(error),
        };
        budget -= frames.len();
        if is_primary {
            primary_times.extend(frames.into_iter().map(|f| rational(f.time)));
            primary_timing_known = true;
        } else if primary_timing_known {
            let unmatched = frames
                .iter()
                .filter(|f| !primary_times.contains(&rational(f.time)))
                .count();
            if unmatched != 0 {
                issue(&mut report, "G09.frame-correspondence-recommendation", Kind::Recommendation,
                Some(track.id), format!("{unmatched} secondary frames lack an exact primary presentation timestamp; no closest-frame fallback selected"))?;
            }
        }
    }
    Ok(report)
}

impl MotionPhoto<'_> {
    pub fn google_video_conformance(
        &self,
        tracks: GoogleVideoTracks,
    ) -> Result<FormatConformanceReport, Error> {
        validate_google_video_container(self.motion_video_bytes()?, tracks)
    }
}
