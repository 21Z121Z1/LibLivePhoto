//! Vendor-neutral descriptions. Resources and tracks occupy different levels.

use crate::ByteRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaTime {
    pub value: i64,
    pub timescale: u32,
}

impl MediaTime {
    pub fn new(value: i64, timescale: u32) -> Option<Self> {
        (timescale != 0).then_some(Self { value, timescale })
    }
    pub fn seconds(self) -> f64 {
        self.value as f64 / f64::from(self.timescale)
    }
    /// Return an exact conversion. Reject overflow and fractional destination ticks.
    pub fn rescale_exact(self, timescale: u32) -> Option<Self> {
        if self.timescale == 0 || timescale == 0 {
            return None;
        }
        let scaled = i128::from(self.value) * i128::from(timescale);
        if scaled % i128::from(self.timescale) != 0 {
            return None;
        }
        Self::new(
            i64::try_from(scaled / i128::from(self.timescale)).ok()?,
            timescale,
        )
    }
    pub fn equivalent(self, other: Self) -> bool {
        self.timescale != 0
            && other.timescale != 0
            && i128::from(self.value) * i128::from(other.timescale)
                == i128::from(other.value) * i128::from(self.timescale)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PhysicalLayout {
    PairedFiles,
    AppendedResources,
    BmffVideoBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssetFormat {
    AppleLivePhoto,
    AndroidMotionPhoto,
    LegacyMicroVideo,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VendorDialect {
    Apple,
    AndroidStandard,
    LegacyMicroVideo,
    Oplus,
    SamsungSef,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContainerKind {
    Jpeg,
    Heif,
    Avif,
    IsoBmff,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Declared,
    Observed,
    Recovered,
    Inferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub kind: EvidenceKind,
    pub evidence: String,
}

impl Provenance {
    pub fn observed(evidence: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Observed,
            evidence: evidence.into(),
        }
    }
    pub fn declared(evidence: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Declared,
            evidence: evidence.into(),
        }
    }
    pub fn recovered(evidence: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Recovered,
            evidence: evidence.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub index: usize,
    pub byte_length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceRole {
    PrimaryStill,
    PrimaryMotionVideo,
    SecondaryVideo,
    SubstitutionVideo,
    AutoplayVideo,
    GeometryDepthVideo,
    PortraitAuxiliaryVideo,
    VendorAuxiliaryVideo,
    UnknownVideo,
    AuxiliaryImage,
    ContainerMetadata,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipKind {
    AuxiliaryTo,
    Substitutes,
    Autoplays,
    SharedData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRelationship {
    pub kind: RelationshipKind,
    pub target: ResourceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub id: ResourceId,
    pub role: ResourceRole,
    pub mime: String,
    pub source_index: usize,
    pub extents: Vec<ByteRange>,
    /// A child can overlap only the enclosing resource named here.
    pub parent: Option<ResourceId>,
    /// Standard container item identity, scoped to the parent container.
    pub container_item_id: Option<u32>,
    pub relationship: Option<ResourceRelationship>,
    pub vendor_role: Option<String>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrackKind {
    Video,
    Audio,
    Metadata,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub movie_duration: u64,
    pub media_start: i64,
    pub rate_integer: i16,
    pub rate_fraction: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRelationship {
    pub kind: RelationshipKind,
    /// A tkhd identifier in the same MediaContainer.
    pub target_track_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The tkhd track identifier. It is distinct from track order and resource identity.
    pub id: u32,
    pub index: usize,
    pub kind: TrackKind,
    pub role: ResourceRole,
    pub handler: [u8; 4],
    pub sample_entries: Vec<[u8; 4]>,
    pub timescale: u32,
    pub duration: u64,
    pub edits: Vec<Edit>,
    pub relationship: Option<TrackRelationship>,
    pub vendor_role: Option<String>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaContainer {
    pub resource: ResourceId,
    pub kind: ContainerKind,
    pub movie_timescale: Option<u32>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    Xmp,
    AppleTimedMetadata,
    VendorMetadata,
    Caller,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presentation {
    pub time: MediaTime,
    pub source: TimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PairingMetadata {
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionAsset {
    pub layout: PhysicalLayout,
    pub format: AssetFormat,
    pub dialect: VendorDialect,
    pub provenance: Provenance,
    pub sources: Vec<Source>,
    pub still: Resource,
    pub motion: Resource,
    pub presentation: Option<Presentation>,
    pub pairing: PairingMetadata,
    pub auxiliary: Vec<Resource>,
    pub containers: Vec<MediaContainer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Diagnostic {
    /// The declared time is outside mvhd duration. Sample timestamp semantics
    /// and original consumer behavior require separate evidence.
    PresentationOutsideMovieHeader,
    VendorCompatibility,
    RecoveredWithoutDirectory,
    RecoveredAfterParseError {
        code: String,
    },
    OpaqueSuffix {
        source_index: usize,
        range: ByteRange,
    },
    UnknownTrackRole {
        resource: ResourceId,
        track_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationCheck {
    ResourceRanges,
    StillContainer,
    MotionContainer,
    Tracks,
    ApplePairing,
    AppleMovieMetadata,
}

/// Structural evidence. This report does not establish device acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub checks: Vec<ValidationCheck>,
    pub diagnostics: Vec<Diagnostic>,
}
