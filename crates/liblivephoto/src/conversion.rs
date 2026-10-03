//! Plans disclose resource mapping before any output is generated.
use crate::{
    AssetFormat, Input, MediaTime, MotionPhoto, MotionPhotoReadError as Error, PairingMetadata,
    ParseOptions, ResourceId, ResourceRole,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TargetProfile {
    PreserveSource,
    AndroidJpeg,
    AndroidHeif,
    SamsungJpegSef,
    ApplePair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmappedPolicy {
    Reject,
    PreserveSidecars,
    DropExplicitly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingPolicy {
    Preserve,
    Set(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritePolicy {
    pub unmapped: UnmappedPolicy,
    pub pairing: PairingPolicy,
}
impl Default for WritePolicy {
    fn default() -> Self {
        Self {
            unmapped: UnmappedPolicy::Reject,
            pairing: PairingPolicy::Preserve,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Preserved,
    Rewritten,
    Generated,
    Unmapped,
    Dropped,
    TranscodeRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConversion {
    pub resource: Option<ResourceId>,
    pub disposition: Disposition,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportState {
    Unverified,
    Unsupported,
    Implemented,
    FixtureVerified,
    ConsumerVerified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub read: SupportState,
    pub write: SupportState,
    pub validation: SupportState,
    pub target_consumer: SupportState,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreservationEvidence {
    SourceBytesEqual,
    VideoContainerBytesEqual,
    EncodedMoviePayloadsEqual,
    EncodedStillPayloadsEqual,
    ExactPresentationEqual,
    ExtractedResourceBytesEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionReport {
    pub target: TargetProfile,
    pub resources: Vec<ResourceConversion>,
    pub evidence: Vec<PreservationEvidence>,
    pub pairing: PairingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub role: ResourceRole,
    pub mime: String,
    pub bytes: Vec<u8>,
    /// A sidecar remains outside the target's native resource graph.
    pub sidecar_for: Option<ResourceId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedAsset {
    pub files: Vec<OutputFile>,
    pub report: ConversionReport,
}

/// The plan borrows an immutable source. Callers cannot remove its blockers.
#[derive(Debug)]
pub struct ConversionPlan<'p, 'a> {
    photo: &'p MotionPhoto<'a>,
    target: TargetProfile,
    policy: WritePolicy,
    report: ConversionReport,
    compatibility: CompatibilityReport,
    unmapped: Vec<ResourceId>,
}
type Result<T> = std::result::Result<T, Error>;

impl<'a> MotionPhoto<'a> {
    pub fn plan<'p>(
        &'p self,
        target: TargetProfile,
        policy: WritePolicy,
    ) -> ConversionPlan<'p, 'a> {
        let mut report = ConversionReport {
            target,
            resources: Vec::new(),
            evidence: Vec::new(),
            pairing: self.asset().pairing.clone(),
        };
        let mut blockers: Vec<String> = Vec::new();
        let mut unmapped = Vec::new();
        if matches!(
            target,
            TargetProfile::AndroidJpeg | TargetProfile::SamsungJpegSef
        ) && self.still().mime != "image/jpeg"
            || target == TargetProfile::AndroidHeif && self.still().mime != "image/heic"
        {
            blockers.push("target still container requires caller-controlled transcoding".into());
            report.resources.push(ResourceConversion {
                resource: Some(self.still().id),
                disposition: Disposition::TranscodeRequired,
                reason: blockers[0].clone(),
            });
        }
        if target == TargetProfile::ApplePair {
            if self.presentation_time().is_none() {
                blockers.push("Apple target requires an explicit presentation timestamp".into());
            }
            match &policy.pairing {
                PairingPolicy::Preserve if report.pairing.identifier.is_none() => {
                    blockers.push("Apple target requires a pairing identifier".into())
                }
                PairingPolicy::Set(value) => {
                    if value.is_empty() || !value.is_ascii() || value.as_bytes().contains(&0) {
                        blockers
                            .push("pairing identifier must be nonempty ASCII without NUL".into());
                    }
                    report.pairing.identifier = Some(value.to_ascii_uppercase());
                    report.resources.push(ResourceConversion {
                        resource: None,
                        disposition: Disposition::Generated,
                        reason: "caller supplied a pairing identifier".into(),
                    });
                }
                _ => {}
            }
            if let Some(time) = self.presentation_time() {
                if self
                    .motion_video_bytes()
                    .is_ok_and(|movie| crate::timeline::patches(movie, time).is_err())
                {
                    blockers.push(
                        "exact Apple movie timeline is not representable within supported fields"
                            .into(),
                    );
                }
            }
        }
        if target == TargetProfile::SamsungJpegSef && self.presentation_time().is_some() {
            blockers.push("SEF target has no supported exact presentation-time mapping; use Android or PreserveSource".into());
        }
        if matches!(
            target,
            TargetProfile::AndroidJpeg | TargetProfile::AndroidHeif
        ) && self
            .presentation_time()
            .is_some_and(|t| t.rescale_exact(1_000_000).is_none())
        {
            blockers.push("Android XMP time requires an exact integer microsecond value".into());
        }
        for r in self.resources() {
            let copy = target == TargetProfile::PreserveSource
                || (target == TargetProfile::ApplePair
                    && self.format() == AssetFormat::AppleLivePhoto
                    && policy.pairing == PairingPolicy::Preserve);
            let xmp = r.vendor_role.as_deref() == Some("JPEG XMP")
                || r.provenance.evidence == "HEIF primary XMP item";
            let rewritten_exif = target == TargetProfile::ApplePair
                && self.format() != AssetFormat::AppleLivePhoto
                && r.vendor_role
                    .as_deref()
                    .is_some_and(|n| n == "JPEG EXIF" || n.ends_with("(Exif)"));
            let native = (target == TargetProfile::PreserveSource
                || (target == TargetProfile::ApplePair
                    && self.format() == AssetFormat::AppleLivePhoto
                    && policy.pairing == PairingPolicy::Preserve)
                || r.role == ResourceRole::PrimaryStill
                || r.role == ResourceRole::PrimaryMotionVideo
                || (r.role == ResourceRole::ContainerMetadata && r.parent.is_some())
                || (r.vendor_role.as_deref() == Some("mpvd header")
                    && target == TargetProfile::AndroidHeif)
                || (target == TargetProfile::SamsungJpegSef
                    && r.provenance.evidence.starts_with("Samsung SEF"))
                || (r.parent == Some(self.still().id) && !r.mime.starts_with("video/")))
                && !rewritten_exif
                && !(target == TargetProfile::ApplePair
                    && !copy
                    && xmp
                    && self.has_auxiliary_directory());
            if !native {
                unmapped.push(r.id);
                if policy.unmapped == UnmappedPolicy::Reject {
                    blockers.push(format!(
                        "target cannot express resource {} ({:?})",
                        r.id.0, r.vendor_role
                    ));
                }
            }
            report.resources.push(ResourceConversion {
                resource: Some(r.id),
                disposition: if !native {
                    Disposition::Unmapped
                } else if !copy
                    && (r.id == self.still().id
                        || target == TargetProfile::ApplePair && r.id == self.motion_video().id || xmp
                        || r.vendor_role.as_deref()==Some("JPEG MPF") || (target==TargetProfile::ApplePair && r.mime=="application/x-exif"))
                {
                    Disposition::Rewritten
                } else {
                    Disposition::Preserved
                },
                reason: if !copy && native && (r.id==self.still().id || xmp || r.vendor_role.as_deref()==Some("JPEG MPF") || target==TargetProfile::ApplePair && r.id==self.motion_video().id || target==TargetProfile::ApplePair && r.mime=="application/x-exif") {
                    "container metadata or references are rewritten; media preservation is checked separately".into()
                } else if native {
                    "resource bytes remain in the native output".into()
                } else {
                    "target has no evidenced native mapping".into()
                },
            });
        }
        if target != TargetProfile::PreserveSource
            && !(target == TargetProfile::ApplePair
                && self.format() == AssetFormat::AppleLivePhoto
                && policy.pairing == PairingPolicy::Preserve)
        {
            report.resources.push(ResourceConversion {
                resource: None,
                disposition: Disposition::Generated,
                reason: match target {
                    TargetProfile::ApplePair => {
                        "target pairing metadata and still-time metadata sample are generated"
                    }
                    TargetProfile::SamsungJpegSef => {
                        "target SEF directory and primary record header are generated"
                    }
                    _ => "target Android resource directory and recognition metadata are generated",
                }
                .into(),
            });
        }
        ConversionPlan {
            photo: self,
            target,
            policy,
            report,
            unmapped,
            compatibility: CompatibilityReport {
                read: SupportState::Implemented,
                write: if blockers.is_empty() {
                    SupportState::Implemented
                } else {
                    SupportState::Unsupported
                },
                validation: SupportState::Implemented,
                target_consumer: SupportState::Unverified,
                blockers,
            },
        }
    }
}

impl ConversionPlan<'_, '_> {
    pub fn report(&self) -> &ConversionReport {
        &self.report
    }
    pub fn compatibility(&self) -> &CompatibilityReport {
        &self.compatibility
    }
    pub fn can_execute(&self) -> bool {
        self.compatibility.blockers.is_empty()
    }
    pub fn execute(&self) -> Result<ComposedAsset> {
        if !self.can_execute() {
            return Err(Error::ConversionBlocked(
                self.compatibility.blockers.clone(),
            ));
        }
        self.photo.validate()?;
        let mut report = self.report.clone();
        let mut files = Vec::new();
        let copy_pair = self.target == TargetProfile::ApplePair
            && self.photo.format() == AssetFormat::AppleLivePhoto
            && self.policy.pairing == PairingPolicy::Preserve;
        if self.target == TargetProfile::PreserveSource || copy_pair {
            for s in &self.photo.asset().sources {
                files.push(OutputFile {
                    role: if s.index == 0 {
                        ResourceRole::PrimaryStill
                    } else {
                        ResourceRole::PrimaryMotionVideo
                    },
                    mime: if s.index == 0 {
                        self.photo.still().mime.clone()
                    } else {
                        self.photo.motion_video().mime.clone()
                    },
                    bytes: self
                        .photo
                        .source_bytes(s.index)
                        .ok_or(Error::InvalidResource)?
                        .to_vec(),
                    sidecar_for: None,
                });
            }
            report.evidence.push(PreservationEvidence::SourceBytesEqual);
        } else if self.target == TargetProfile::SamsungJpegSef {
            let source = self.photo.source_bytes(0).ok_or(Error::InvalidResource)?;
            let sef = crate::vendor::samsung::parse(source)?;
            let description="<rdf:Description xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:Camera=\"http://ns.google.com/photos/1.0/camera/\" Camera:MotionPhoto=\"0\"/>";
            // SEF has no evidenced exact-time field. Preserve the original XMP time
            // as a non-recognition attribute; the SEF parser does not invent a value.
            if self.photo.presentation_time().is_some() {
                return Err(Error::ConversionBlocked(vec!["SEF target has no supported presentation-time mapping; use Android or PreserveSource".into()]));
            }
            let still = crate::jpeg_write::write_xmp(self.photo.still_bytes()?, description)?;
            let bytes = crate::vendor::samsung::write(
                &still,
                self.photo.motion_video_bytes()?,
                sef.as_ref().map(|s| (source, s)),
            )?;
            let parsed = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::compatible())?;
            if parsed.motion_video_bytes()? != self.photo.motion_video_bytes()? {
                return Err(Error::InvalidResource);
            }
            report
                .evidence
                .push(PreservationEvidence::VideoContainerBytesEqual);
            files.push(OutputFile {
                role: ResourceRole::PrimaryStill,
                mime: "image/jpeg".into(),
                bytes,
                sidecar_for: None,
            });
        } else if self.target == TargetProfile::AndroidJpeg {
            let items = self
                .photo
                .asset()
                .auxiliary
                .iter()
                .filter(|r| {
                    r.parent == Some(self.photo.still().id)
                        && r.provenance.evidence == "XMP Container:Directory item"
                })
                .map(|r| {
                    let shared = r
                        .relationship
                        .as_ref()
                        .is_some_and(|rel| rel.kind == crate::RelationshipKind::SharedData);
                    (
                        r.mime.clone(),
                        r.vendor_role.clone().unwrap_or_else(|| "Unknown".into()),
                        if shared {
                            0
                        } else {
                            r.extents.iter().map(|e| e.length()).sum()
                        },
                    )
                })
                .collect::<Vec<_>>();
            let video = self.photo.motion_video_bytes()?;
            let description = crate::jpeg_write::android_description(
                video.len(),
                "image/jpeg",
                self.photo.presentation_time(),
                &items,
            )?;
            let still = crate::jpeg_write::write_xmp(self.photo.still_bytes()?, &description)?;
            let bytes = [still, video.to_vec()].concat();
            let parsed = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict())?;
            if parsed.motion_video_bytes()? != video {
                return Err(Error::InvalidResource);
            }
            check_time(self.photo.presentation_time(), parsed.presentation_time())?;
            if crate::jpeg_write::encoded_payload(self.photo.still_bytes()?)?
                != crate::jpeg_write::encoded_payload(parsed.still_bytes()?)?
            {
                return Err(Error::InvalidStill);
            }
            report
                .evidence
                .push(PreservationEvidence::EncodedStillPayloadsEqual);
            report.evidence.extend([
                PreservationEvidence::VideoContainerBytesEqual,
                PreservationEvidence::ExactPresentationEqual,
            ]);
            files.push(OutputFile {
                role: ResourceRole::PrimaryStill,
                mime: "image/jpeg".into(),
                bytes,
                sidecar_for: None,
            });
        } else if self.target == TargetProfile::AndroidHeif {
            let video = self.photo.motion_video_bytes()?;
            let description = crate::jpeg_write::android_description(
                video.len(),
                "image/heic",
                self.photo.presentation_time(),
                &[],
            )?;
            let mut bytes = crate::heif_io::replace_xmp(self.photo.still_bytes()?, &description)?;
            let mpvd = liblivephoto_format::isobmff::make_box(
                liblivephoto_format::FourCC::new(*b"mpvd"),
                video,
            )
            .map_err(|_| Error::InvalidResource)?;
            bytes.extend_from_slice(&mpvd);
            let parsed = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict())?;
            if parsed.motion_video_bytes()? != video {
                return Err(Error::InvalidResource);
            }
            check_time(self.photo.presentation_time(), parsed.presentation_time())?;
            check_heif_images(self.photo, &parsed)?;
            report.evidence.extend([
                PreservationEvidence::VideoContainerBytesEqual,
                PreservationEvidence::ExactPresentationEqual,
                PreservationEvidence::EncodedStillPayloadsEqual,
            ]);
            files.push(OutputFile {
                role: ResourceRole::PrimaryStill,
                mime: "image/heic".into(),
                bytes,
                sidecar_for: None,
            });
        } else if self.target == TargetProfile::ApplePair {
            let identifier = report
                .pairing
                .identifier
                .as_deref()
                .ok_or(Error::MissingPairMetadata)?;
            let time = self
                .photo
                .presentation_time()
                .ok_or(Error::MissingPairMetadata)?;
            let still = if self.photo.still().mime == "image/jpeg" {
                crate::jpeg_write::apple_still(
                    self.photo.still_bytes()?,
                    identifier,
                    self.policy.unmapped != UnmappedPolicy::Reject,
                )?
            } else {
                crate::live_photo_still::write_live_photo_heif_still(
                    self.photo.still_bytes()?,
                    identifier,
                )?
            };
            let original = self.photo.motion_video_bytes()?;
            let movie = crate::apple::remux_movie(original, identifier, time)?;
            let parsed = MotionPhoto::parse(
                Input::ApplePair {
                    still: &still,
                    movie: &movie,
                },
                ParseOptions::strict(),
            )?;
            check_time(Some(time), parsed.presentation_time())?;
            if crate::apple::media_payloads(original)? != crate::apple::media_payloads(&movie)? {
                return Err(Error::InvalidResource);
            }
            if self.photo.still().mime == "image/jpeg" {
                if crate::jpeg_write::encoded_payload(self.photo.still_bytes()?)?
                    != crate::jpeg_write::encoded_payload(&still)?
                {
                    return Err(Error::InvalidStill);
                }
                report
                    .evidence
                    .push(PreservationEvidence::EncodedStillPayloadsEqual);
            } else {
                check_heif_images(self.photo, &parsed)?;
                report
                    .evidence
                    .push(PreservationEvidence::EncodedStillPayloadsEqual);
            }
            report.evidence.extend([
                PreservationEvidence::EncodedMoviePayloadsEqual,
                PreservationEvidence::ExactPresentationEqual,
            ]);
            files.push(OutputFile {
                role: ResourceRole::PrimaryStill,
                mime: self.photo.still().mime.clone(),
                bytes: still,
                sidecar_for: None,
            });
            files.push(OutputFile {
                role: ResourceRole::PrimaryMotionVideo,
                mime: "video/quicktime".into(),
                bytes: movie,
                sidecar_for: None,
            });
        } else {
            return Err(Error::ConversionBlocked(vec![
                "target writer is unavailable".into(),
            ]));
        }
        for id in &self.unmapped {
            if self.policy.unmapped == UnmappedPolicy::PreserveSidecars {
                let r = self
                    .photo
                    .resources()
                    .find(|r| r.id == *id)
                    .ok_or(Error::InvalidResource)?;
                let mut bytes = Vec::new();
                self.photo.extract(r, &mut bytes)?;
                files.push(OutputFile {
                    role: r.role,
                    mime: r.mime.clone(),
                    bytes,
                    sidecar_for: Some(*id),
                });
                report.resources.push(ResourceConversion {
                    resource: Some(*id),
                    disposition: Disposition::Preserved,
                    reason: "exact resource bytes retained in an explicit sidecar".into(),
                });
                report
                    .evidence
                    .push(PreservationEvidence::ExtractedResourceBytesEqual);
            } else if self.policy.unmapped == UnmappedPolicy::DropExplicitly {
                report.resources.push(ResourceConversion {
                    resource: Some(*id),
                    disposition: Disposition::Dropped,
                    reason: "caller explicitly allowed the unmapped resource to be dropped".into(),
                });
            }
        }
        Ok(ComposedAsset { files, report })
    }
}

fn check_heif_images(source: &MotionPhoto<'_>, target: &MotionPhoto<'_>) -> Result<()> {
    for item in source
        .resources()
        .filter(|r| r.role == ResourceRole::AuxiliaryImage)
    {
        let output = target
            .resources()
            .find(|r| {
                r.container_item_id == item.container_item_id
                    && r.role == ResourceRole::AuxiliaryImage
            })
            .ok_or(Error::InvalidStill)?;
        let mut original = Vec::new();
        let mut rewritten = Vec::new();
        source.extract(item, &mut original)?;
        target.extract(output, &mut rewritten)?;
        if original != rewritten {
            return Err(Error::InvalidStill);
        }
    }
    Ok(())
}

fn check_time(source: Option<MediaTime>, target: Option<MediaTime>) -> Result<()> {
    if match (source, target) {
        (Some(a), Some(b)) => a.equivalent(b),
        (None, None) => true,
        _ => false,
    } {
        Ok(())
    } else {
        Err(Error::InvalidPresentation)
    }
}
