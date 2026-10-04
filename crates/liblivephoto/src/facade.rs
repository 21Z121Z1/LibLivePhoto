//! File-level reading, resource extraction and structural validation.

use std::borrow::Cow;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use liblivephoto_format::{isobmff::parse_boxes, jpeg_image_end};

use crate::{
    normalize_embedded_video, parse_android_motion_photo, parse_oppo_motion_photo,
    read_apple_content_identifier, read_live_photo_content_identifier,
    read_live_photo_still_presentation, resolve_video_stream_layout, validate_live_photo_movie,
    AssetFormat, ByteRange, ContainerKind, Diagnostic, LivePhotoMovieError, LivePhotoStillError,
    MediaTime, MotionAsset, MotionPhotoAsset, MotionPhotoError, MotionPhotoSourceKind,
    PairingMetadata, PhysicalLayout, Presentation, Provenance, RelationshipKind, Resource,
    ResourceId, ResourceRelationship, ResourceRole, Source, TimeSource, ValidationCheck,
    ValidationReport, VendorDialect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Strict,
    Compatible,
    Forensic,
}

/// The byte budget is shared by both files of an Apple pair. Recovery scans
/// only the bounded file tail, and never silently selects among multiple videos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub mode: ParseMode,
    pub max_input_bytes: usize,
    pub max_scan_bytes: usize,
    pub max_candidates: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::compatible()
    }
}

impl ParseOptions {
    pub const fn compatible() -> Self {
        Self {
            mode: ParseMode::Compatible,
            max_input_bytes: 256 * 1024 * 1024,
            max_scan_bytes: 64 * 1024 * 1024,
            max_candidates: 128,
        }
    }

    pub const fn strict() -> Self {
        Self {
            mode: ParseMode::Strict,
            ..Self::compatible()
        }
    }

    pub const fn forensic() -> Self {
        Self {
            mode: ParseMode::Forensic,
            ..Self::compatible()
        }
    }
}

/// Apple pairing is explicit. Filenames are never treated as pairing evidence.
#[derive(Debug, Clone, Copy)]
pub enum Input<'a> {
    SingleFile(&'a [u8]),
    ApplePair {
        still: &'a [u8],
        movie: &'a [u8],
    },
    /// Caller-declared association for composition. No Apple identity is inferred.
    Parts {
        still: &'a [u8],
        movie: &'a [u8],
        presentation: Option<MediaTime>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum PathInput<'a> {
    SingleFile(&'a Path),
    ApplePair { still: &'a Path, movie: &'a Path },
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MotionPhotoReadError {
    Parse(MotionPhotoError),
    Movie(LivePhotoMovieError),
    Still(LivePhotoStillError),
    Io(io::Error),
    Unrecognized,
    InputLimit,
    ScanLimit,
    AmbiguousRecovery,
    InvalidResource,
    InvalidStill,
    InvalidPresentation,
    IdentifierMismatch,
    MissingPairMetadata,
    StrictViolation,
    ConversionBlocked(Vec<String>),
}

impl fmt::Display for MotionPhotoReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Movie(error) => error.fmt(f),
            Self::Still(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::Unrecognized => f.write_str("input is not a supported motion asset"),
            Self::InputLimit => f.write_str("motion asset exceeds the input byte budget"),
            Self::ScanLimit => f.write_str("motion asset recovery exceeds the scan budget"),
            Self::AmbiguousRecovery => {
                f.write_str("multiple complete recovery candidates; selection requires metadata")
            }
            Self::InvalidResource => {
                f.write_str("motion asset resource ranges are invalid or overlap")
            }
            Self::InvalidStill => {
                f.write_str("motion asset still container is invalid or unsupported")
            }
            Self::InvalidPresentation => f.write_str("motion asset presentation time is invalid"),
            Self::IdentifierMismatch => f.write_str("Apple still and movie identifiers differ"),
            Self::MissingPairMetadata => {
                f.write_str("Apple pair lacks identifier or still-image-time metadata")
            }
            Self::StrictViolation => f.write_str("input requires vendor compatibility or recovery"),
            Self::ConversionBlocked(reasons) => {
                write!(f, "conversion is blocked: {}", reasons.join("; "))
            }
        }
    }
}

impl std::error::Error for MotionPhotoReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Movie(error) => Some(error),
            Self::Still(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MotionPhotoError> for MotionPhotoReadError {
    fn from(value: MotionPhotoError) -> Self {
        Self::Parse(value)
    }
}
impl From<LivePhotoMovieError> for MotionPhotoReadError {
    fn from(value: LivePhotoMovieError) -> Self {
        Self::Movie(value)
    }
}
impl From<LivePhotoStillError> for MotionPhotoReadError {
    fn from(value: LivePhotoStillError) -> Self {
        Self::Still(value)
    }
}
impl From<io::Error> for MotionPhotoReadError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

type ReadResult<T> = std::result::Result<T, MotionPhotoReadError>;

/// An immutable parsed asset and its original sources. Byte parsing borrows;
/// path opening owns a bounded snapshot. No decode or metadata rewrite occurs.
#[derive(Debug)]
pub struct MotionPhoto<'a> {
    sources: Vec<Cow<'a, [u8]>>,
    asset: MotionAsset,
    android_asset: Option<MotionPhotoAsset>,
    diagnostics: Vec<Diagnostic>,
}

fn resource(role: ResourceRole, mime: &str, source_index: usize, range: ByteRange) -> Resource {
    Resource {
        id: ResourceId(match role {
            ResourceRole::PrimaryStill => 0,
            ResourceRole::PrimaryMotionVideo => 1,
            _ => usize::MAX,
        }),
        role,
        mime: mime.to_owned(),
        source_index,
        extents: vec![range],
        parent: None,
        container_item_id: None,
        relationship: None,
        vendor_role: None,
        provenance: Provenance::observed("validated resource extent"),
    }
}

fn full_range(data: &[u8]) -> ReadResult<ByteRange> {
    Ok(ByteRange::new(0, data.len() as u64)?)
}

fn slice(data: &[u8], range: ByteRange) -> ReadResult<&[u8]> {
    let start =
        usize::try_from(range.lower_bound).map_err(|_| MotionPhotoReadError::InvalidResource)?;
    let end =
        usize::try_from(range.upper_bound).map_err(|_| MotionPhotoReadError::InvalidResource)?;
    if start >= end {
        return Err(MotionPhotoReadError::InvalidResource);
    }
    data.get(start..end)
        .ok_or(MotionPhotoReadError::InvalidResource)
}

// Walk top-level boxes without allocating an unbounded BoxHeader list.
fn validate_boxes(data: &[u8], heif: bool) -> ReadResult<()> {
    let mut offset = 0usize;
    let mut count = 0;
    let mut has_meta = false;
    while offset < data.len() {
        count += 1;
        if count > 4096 {
            return Err(MotionPhotoReadError::InvalidStill);
        }
        let header = data
            .get(offset..offset.saturating_add(8))
            .ok_or(MotionPhotoReadError::InvalidStill)?;
        let size32 = u32::from_be_bytes(
            header[..4]
                .try_into()
                .map_err(|_| MotionPhotoReadError::InvalidStill)?,
        );
        let size = match size32 {
            0 => data.len() - offset,
            1 => {
                let raw = data
                    .get(offset + 8..offset + 16)
                    .ok_or(MotionPhotoReadError::InvalidStill)?;
                usize::try_from(u64::from_be_bytes(
                    raw.try_into()
                        .map_err(|_| MotionPhotoReadError::InvalidStill)?,
                ))
                .map_err(|_| MotionPhotoReadError::InvalidStill)?
            }
            value => value as usize,
        };
        let header_size = if size32 == 1 { 16 } else { 8 };
        if size < header_size {
            return Err(MotionPhotoReadError::InvalidStill);
        }
        let end = offset
            .checked_add(size)
            .ok_or(MotionPhotoReadError::InvalidStill)?;
        let item =
            parse_boxes(data, offset..end).map_err(|_| MotionPhotoReadError::InvalidStill)?;
        if offset == 0 {
            if item[0].kind.as_bytes() != b"ftyp" {
                return Err(MotionPhotoReadError::InvalidStill);
            }
            let payload = &data[item[0].data_start..item[0].data_end];
            if payload.len() < 8 || !(payload.len() - 8).is_multiple_of(4) {
                return Err(MotionPhotoReadError::InvalidStill);
            }
            let brand_is_heif = |brand: &[u8]| {
                matches!(
                    brand,
                    b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
                )
            };
            if heif
                && !brand_is_heif(&payload[..4])
                && !payload[8..]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|b| brand_is_heif(b))
            {
                return Err(MotionPhotoReadError::InvalidStill);
            }
        }
        has_meta |= item[0].kind.as_bytes() == b"meta";
        offset = end;
    }
    if offset == 0 || (heif && !has_meta) {
        return Err(MotionPhotoReadError::InvalidStill);
    }
    Ok(())
}

fn still_mime(data: &[u8]) -> ReadResult<&'static str> {
    if data.starts_with(&[0xff, 0xd8]) {
        jpeg_image_end(data, 0).map_err(|_| MotionPhotoReadError::InvalidStill)?;
        Ok("image/jpeg")
    } else {
        validate_boxes(data, true)?;
        Ok("image/heic")
    }
}

impl<'a> MotionPhoto<'a> {
    pub fn parse(input: Input<'a>, options: ParseOptions) -> ReadResult<Self> {
        let lengths = match input {
            Input::SingleFile(data) => Some(data.len()),
            Input::ApplePair { still, movie } | Input::Parts { still, movie, .. } => {
                still.len().checked_add(movie.len())
            }
        };
        if lengths.is_none_or(|length| length > options.max_input_bytes) {
            return Err(MotionPhotoReadError::InputLimit);
        }
        let mut photo = match input {
            Input::SingleFile(data) => Self::parse_single(data, options)?,
            Input::ApplePair { still, movie } => Self::parse_pair(still, movie)?,
            Input::Parts {
                still,
                movie,
                presentation,
            } => Self::parse_parts(still, movie, presentation)?,
        };
        photo.complete_model()?;
        photo.validate()?;
        Ok(photo)
    }

    fn parse_pair(still: &'a [u8], movie: &'a [u8]) -> ReadResult<Self> {
        let mime = still_mime(still)?;
        validate_boxes(movie, false).map_err(|_| MotionPhotoError::InvalidVideoPayload)?;
        let still_id = read_apple_content_identifier(still)?
            .ok_or(MotionPhotoReadError::MissingPairMetadata)?;
        let movie_id = read_live_photo_content_identifier(movie)?
            .ok_or(MotionPhotoReadError::MissingPairMetadata)?;
        if still_id != movie_id {
            return Err(MotionPhotoReadError::IdentifierMismatch);
        }
        let time = read_live_photo_still_presentation(movie)?
            .ok_or(MotionPhotoReadError::MissingPairMetadata)?;
        Ok(Self {
            sources: vec![Cow::Borrowed(still), Cow::Borrowed(movie)],
            asset: MotionAsset {
                layout: PhysicalLayout::PairedFiles,
                format: AssetFormat::AppleLivePhoto,
                dialect: VendorDialect::Apple,
                provenance: Provenance::observed("matching Apple identifiers and timed metadata"),
                sources: Vec::new(),
                containers: Vec::new(),
                still: resource(ResourceRole::PrimaryStill, mime, 0, full_range(still)?),
                motion: resource(
                    ResourceRole::PrimaryMotionVideo,
                    "video/quicktime",
                    1,
                    full_range(movie)?,
                ),
                presentation: Some(Presentation {
                    time,
                    source: TimeSource::AppleTimedMetadata,
                }),
                pairing: PairingMetadata {
                    identifier: Some(still_id),
                },
                auxiliary: Vec::new(),
            },
            android_asset: None,
            diagnostics: Vec::new(),
        })
    }

    fn parse_parts(
        still: &'a [u8],
        movie: &'a [u8],
        presentation: Option<MediaTime>,
    ) -> ReadResult<Self> {
        let mime = still_mime(still)?;
        validate_boxes(movie, false)?;
        Ok(Self {
            sources: vec![Cow::Borrowed(still), Cow::Borrowed(movie)],
            asset: MotionAsset {
                layout: PhysicalLayout::PairedFiles,
                format: AssetFormat::Unspecified,
                dialect: VendorDialect::Unknown,
                provenance: Provenance::declared("caller supplied resource association"),
                sources: Vec::new(),
                containers: Vec::new(),
                still: resource(ResourceRole::PrimaryStill, mime, 0, full_range(still)?),
                motion: resource(
                    ResourceRole::PrimaryMotionVideo,
                    "video/mp4",
                    1,
                    full_range(movie)?,
                ),
                presentation: presentation.map(|time| Presentation {
                    time,
                    source: TimeSource::Caller,
                }),
                pairing: PairingMetadata::default(),
                auxiliary: Vec::new(),
            },
            android_asset: None,
            diagnostics: Vec::new(),
        })
    }

    fn parse_single(data: &'a [u8], options: ParseOptions) -> ReadResult<Self> {
        let sef = crate::vendor::samsung::parse(data)?;
        let parsed = if options.mode == ParseMode::Strict {
            parse_android_motion_photo(data)
        } else {
            parse_oppo_motion_photo(data)
        };
        let result = match parsed {
            Ok(Some(asset)) => Self::from_android(data, asset, options),
            Ok(None) if sef.is_some() && options.mode != ParseMode::Strict => Self::from_sef(
                data,
                sef.as_ref().ok_or(MotionPhotoReadError::InvalidResource)?,
            ),
            Err(_) if sef.is_some() && options.mode != ParseMode::Strict => Self::from_sef(
                data,
                sef.as_ref().ok_or(MotionPhotoReadError::InvalidResource)?,
            ),
            Ok(None) if options.mode == ParseMode::Forensic => Self::recover(data, options, None),
            Err(error) if options.mode == ParseMode::Forensic => {
                Self::recover(data, options, Some(error))
            }
            Ok(None) => Err(MotionPhotoReadError::Unrecognized),
            Err(error) => Err(error.into()),
        };
        let mut photo = result?;
        if let Some(sef) = sef {
            if photo.asset.motion.extents != vec![sef.motion] {
                return Err(MotionPhotoReadError::InvalidResource);
            }
            photo.asset.dialect = VendorDialect::SamsungSef;
            // Replace coarse suffix views with the evidenced payload views.
            // complete_model retains every intervening header and padding byte.
            photo
                .asset
                .auxiliary
                .retain(|r| r.role != ResourceRole::Unknown && r.parent.is_some());
            for r in sef.records {
                if r.payload == sef.motion || r.payload.length() == 0 {
                    continue;
                }
                let payload = &data[r.payload.lower_bound as usize..r.payload.upper_bound as usize];
                let video = crate::scanner::is_ftyp_box_start(payload, 0, payload.len() as u64)
                    .unwrap_or(false);
                let mut item = resource(
                    if video {
                        ResourceRole::UnknownVideo
                    } else {
                        ResourceRole::Unknown
                    },
                    if video {
                        "video/mp4"
                    } else {
                        "application/octet-stream"
                    },
                    0,
                    r.payload,
                );
                if r.payload.upper_bound <= photo.asset.still.extents[0].upper_bound {
                    item.parent = Some(photo.asset.still.id);
                }
                if video {
                    item.relationship = Some(ResourceRelationship {
                        kind: RelationshipKind::AuxiliaryTo,
                        target: ResourceId(1),
                    });
                }
                item.vendor_role = Some(r.name);
                item.provenance = Provenance::observed(format!(
                    "Samsung SEF {} type {:#06x}",
                    sef.version, r.kind
                ));
                photo.asset.auxiliary.push(item);
            }
        }
        Ok(photo)
    }

    fn from_sef(data: &'a [u8], sef: &crate::vendor::samsung::Sef) -> ReadResult<Self> {
        let end = if data.starts_with(&[255, 216]) {
            sef.records
                .iter()
                .map(|r| r.raw.lower_bound as usize)
                .min()
                .ok_or(MotionPhotoReadError::InvalidResource)?
        } else {
            let top = liblivephoto_format::isobmff::scan_top_level_boxes(data)
                .map_err(|_| MotionPhotoReadError::InvalidStill)?;
            top.boxes
                .iter()
                .find(|b| matches!(b.kind.as_bytes(), b"mpvd" | b"sefd"))
                .map(|b| b.box_start)
                .ok_or(MotionPhotoReadError::InvalidStill)?
        };
        let mime = still_mime(&data[..end])?;
        Ok(Self {
            sources: vec![Cow::Borrowed(data)],
            asset: MotionAsset {
                layout: if mime == "image/jpeg" {
                    PhysicalLayout::AppendedResources
                } else {
                    PhysicalLayout::BmffVideoBox
                },
                format: AssetFormat::Unspecified,
                dialect: VendorDialect::SamsungSef,
                provenance: Provenance::observed("Samsung SEF primary video record"),
                sources: Vec::new(),
                containers: Vec::new(),
                still: resource(
                    ResourceRole::PrimaryStill,
                    mime,
                    0,
                    ByteRange::new(0, end as u64)?,
                ),
                motion: resource(ResourceRole::PrimaryMotionVideo, "video/mp4", 0, sef.motion),
                presentation: crate::android::declared_time(data)?.map(|value| Presentation {
                    time: MediaTime {
                        value,
                        timescale: 1_000_000,
                    },
                    source: TimeSource::Xmp,
                }),
                pairing: PairingMetadata { identifier: None },
                auxiliary: Vec::new(),
            },
            android_asset: None,
            diagnostics: vec![Diagnostic::VendorCompatibility],
        })
    }

    fn from_android(
        data: &'a [u8],
        android: MotionPhotoAsset,
        options: ParseOptions,
    ) -> ReadResult<Self> {
        let mime = still_mime(slice(data, android.still_resource_range)?)?;
        let is_oppo = android.source_kind == MotionPhotoSourceKind::OppoLivePhoto;
        let stream_count = android
            .vendor_metadata
            .as_ref()
            .map_or(1, |metadata| metadata.stream_count);
        let layout =
            resolve_video_stream_layout(data, android.video_resource_range, is_oppo, stream_count)?;
        let mut auxiliary = Vec::new();
        let mut diagnostics = Vec::new();
        if is_oppo {
            diagnostics.push(Diagnostic::VendorCompatibility);
        }
        let raw_video = slice(data, layout.primary.range)?;
        let normalized = normalize_embedded_video(raw_video)?;
        let end = layout
            .primary
            .range
            .lower_bound
            .checked_add(normalized.data.len() as u64)
            .ok_or(MotionPhotoReadError::InvalidResource)?;
        let primary_range = ByteRange::new(layout.primary.range.lower_bound, end)?;
        if normalized.removed_vendor_bytes > 0 {
            let range = ByteRange::new(end, layout.primary.range.upper_bound)?;
            auxiliary.push(resource(
                ResourceRole::Unknown,
                "application/octet-stream",
                0,
                range,
            ));
            diagnostics.push(Diagnostic::OpaqueSuffix {
                source_index: 0,
                range,
            });
        }
        for stream in layout.auxiliary_geometry {
            auxiliary.push(resource(
                ResourceRole::VendorAuxiliaryVideo,
                "video/mp4",
                0,
                stream.range,
            ));
        }
        if android.video_resource_range.upper_bound < data.len() as u64 {
            let range =
                ByteRange::new(android.video_resource_range.upper_bound, data.len() as u64)?;
            auxiliary.push(resource(
                ResourceRole::Unknown,
                "application/octet-stream",
                0,
                range,
            ));
            diagnostics.push(Diagnostic::OpaqueSuffix {
                source_index: 0,
                range,
            });
        }
        if options.mode == ParseMode::Strict && !diagnostics.is_empty() {
            return Err(MotionPhotoReadError::StrictViolation);
        }
        let presentation = match (
            android.presentation_timestamp_us,
            android.presentation_source,
        ) {
            (Some(value), Some(source)) if value >= 0 => Some(Presentation {
                time: MediaTime {
                    value,
                    timescale: 1_000_000,
                },
                source: if source == crate::PresentationSource::OppoCoverFrame {
                    TimeSource::VendorMetadata
                } else {
                    TimeSource::Xmp
                },
            }),
            (None, _) => None,
            _ => return Err(MotionPhotoReadError::InvalidPresentation),
        };
        let format = match android.source_kind {
            MotionPhotoSourceKind::LegacyMicroVideoV1b => AssetFormat::LegacyMicroVideo,
            _ => AssetFormat::AndroidMotionPhoto,
        };
        Ok(Self {
            sources: vec![Cow::Borrowed(data)],
            asset: MotionAsset {
                format,
                layout: if mime == "image/heic" {
                    PhysicalLayout::BmffVideoBox
                } else {
                    PhysicalLayout::AppendedResources
                },
                dialect: if is_oppo {
                    VendorDialect::Oplus
                } else if format == AssetFormat::LegacyMicroVideo {
                    VendorDialect::LegacyMicroVideo
                } else {
                    VendorDialect::AndroidStandard
                },
                provenance: Provenance::declared("Motion Photo directory or vendor extension"),
                sources: Vec::new(),
                containers: Vec::new(),
                still: resource(
                    ResourceRole::PrimaryStill,
                    mime,
                    0,
                    android.still_resource_range,
                ),
                motion: resource(
                    ResourceRole::PrimaryMotionVideo,
                    "video/mp4",
                    0,
                    primary_range,
                ),
                presentation,
                pairing: PairingMetadata::default(),
                auxiliary,
            },
            android_asset: Some(android),
            diagnostics,
        })
    }

    fn recover(
        data: &'a [u8],
        options: ParseOptions,
        error: Option<MotionPhotoError>,
    ) -> ReadResult<Self> {
        let still_end = jpeg_image_end(data, 0).map_err(|_| MotionPhotoReadError::Unrecognized)?;
        let start = still_end.max(data.len().saturating_sub(options.max_scan_bytes));
        // A partial tail scan cannot establish that there is only one candidate.
        if start != still_end {
            return Err(MotionPhotoReadError::ScanLimit);
        }
        let offsets = bounded_candidates(data, start, options.max_candidates)?;
        let mut selected = None;
        for offset in offsets {
            let raw = &data[offset..];
            let Ok(normalized) = normalize_embedded_video(raw) else {
                continue;
            };
            if selected.is_some() {
                return Err(MotionPhotoReadError::AmbiguousRecovery);
            }
            selected = Some(ByteRange::new(
                offset as u64,
                (offset + normalized.data.len()) as u64,
            )?);
        }
        let video = selected.ok_or(MotionPhotoReadError::Unrecognized)?;
        let mut diagnostics = vec![Diagnostic::RecoveredWithoutDirectory];
        if let Some(error) = error {
            diagnostics.push(Diagnostic::RecoveredAfterParseError {
                code: error.code().to_owned(),
            });
        }
        let mut auxiliary = Vec::new();
        if video.upper_bound < data.len() as u64 {
            let range = ByteRange::new(video.upper_bound, data.len() as u64)?;
            auxiliary.push(resource(
                ResourceRole::Unknown,
                "application/octet-stream",
                0,
                range,
            ));
            diagnostics.push(Diagnostic::OpaqueSuffix {
                source_index: 0,
                range,
            });
        }
        Ok(Self {
            sources: vec![Cow::Borrowed(data)],
            asset: MotionAsset {
                layout: PhysicalLayout::AppendedResources,
                format: AssetFormat::Unspecified,
                dialect: VendorDialect::Unknown,
                provenance: Provenance::recovered(
                    "bounded search found one complete BMFF candidate",
                ),
                sources: Vec::new(),
                containers: Vec::new(),
                still: resource(
                    ResourceRole::PrimaryStill,
                    "image/jpeg",
                    0,
                    ByteRange::new(0, video.lower_bound)?,
                ),
                motion: resource(ResourceRole::PrimaryMotionVideo, "video/mp4", 0, video),
                presentation: None,
                pairing: PairingMetadata::default(),
                auxiliary,
            },
            android_asset: None,
            diagnostics,
        })
    }

    pub fn asset(&self) -> &MotionAsset {
        &self.asset
    }
    pub fn format(&self) -> AssetFormat {
        self.asset.format
    }
    pub fn still(&self) -> &Resource {
        &self.asset.still
    }
    pub fn motion_video(&self) -> &Resource {
        &self.asset.motion
    }
    pub fn presentation_time(&self) -> Option<MediaTime> {
        self.asset.presentation.map(|value| value.time)
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub(crate) fn has_auxiliary_directory(&self) -> bool {
        self.android_asset.as_ref().is_some_and(|a| {
            a.items.iter().any(|i| {
                !i.semantic.eq_ignore_ascii_case("Primary")
                    && !i.semantic.eq_ignore_ascii_case("MotionPhoto")
            })
        })
    }
    #[cfg(feature = "xdremux-compat")]
    #[doc(hidden)]
    pub fn compat_android_asset(&self) -> Option<&MotionPhotoAsset> {
        self.android_asset.as_ref()
    }
    pub fn source_bytes(&self, index: usize) -> Option<&[u8]> {
        self.sources.get(index).map(Cow::as_ref)
    }

    pub fn probe(input: Input<'a>, options: ParseOptions) -> ReadResult<MotionAsset> {
        Ok(Self::parse(input, options)?.asset)
    }

    fn complete_model(&mut self) -> ReadResult<()> {
        self.asset.sources = self
            .sources
            .iter()
            .enumerate()
            .map(|(index, data)| Source {
                index,
                byte_length: data.len() as u64,
            })
            .collect();
        // Positive directory items inside the still container are extractable children.
        // Zero-length entries are explicit shared-data views of the preceding item.
        if let Some(android) = &self.android_asset {
            if self.asset.layout == PhysicalLayout::AppendedResources {
                let mut end = self.sources[0].len() as u64;
                let mut ranges = vec![None; android.items.len()];
                for index in (1..android.items.len()).rev() {
                    let start = end
                        .checked_sub(android.items[index].length)
                        .ok_or(MotionPhotoReadError::InvalidResource)?;
                    if start < end {
                        ranges[index] = Some(ByteRange::new(start, end)?);
                    }
                    end = start;
                }
                let mut previous = (self.asset.still.id, self.asset.still.extents.clone());
                for (index, item) in android.items.iter().enumerate().skip(1) {
                    if item.semantic.eq_ignore_ascii_case("MotionPhoto") {
                        continue;
                    }
                    let id = ResourceId(self.asset.auxiliary.len() + 2);
                    let shared = ranges[index].is_none();
                    let extents = ranges[index]
                        .map(|r| vec![r])
                        .unwrap_or_else(|| previous.1.clone());
                    if extents
                        .iter()
                        .any(|r| r.upper_bound > self.asset.still.extents[0].upper_bound)
                    {
                        return Err(MotionPhotoReadError::InvalidResource);
                    }
                    let r = Resource {
                        id,
                        role: if item.mime.starts_with("video/") {
                            ResourceRole::UnknownVideo
                        } else if item.mime.starts_with("image/") {
                            ResourceRole::AuxiliaryImage
                        } else {
                            ResourceRole::Unknown
                        },
                        mime: item.mime.clone(),
                        source_index: 0,
                        extents: extents.clone(),
                        parent: Some(self.asset.still.id),
                        container_item_id: None,
                        relationship: shared.then_some(ResourceRelationship {
                            kind: RelationshipKind::SharedData,
                            target: previous.0,
                        }),
                        vendor_role: Some(item.semantic.clone()),
                        provenance: Provenance::declared("XMP Container:Directory item"),
                    };
                    self.asset.auxiliary.push(r);
                    previous = (id, extents);
                }
            }
        }
        if self.asset.still.mime == "image/heic" {
            let items = crate::heif_io::items(self.still_bytes()?, self.asset.auxiliary.len() + 2)?;
            self.asset.auxiliary.extend(items);
        } else if self.asset.still.mime == "image/jpeg" {
            let items = crate::jpeg_write::metadata_resources(
                self.still_bytes()?,
                self.asset.auxiliary.len() + 2,
            )?;
            self.asset.auxiliary.extend(items);
        }
        // Retain every unclaimed source byte, including mpvd headers and padding.
        for (source_index, source) in self.sources.iter().enumerate() {
            let mut occupied: Vec<_> = self
                .resources()
                .filter(|r| r.source_index == source_index && r.parent.is_none())
                .flat_map(|r| r.extents.iter().copied())
                .collect();
            occupied.sort_by_key(|r| r.lower_bound);
            let mut end = 0;
            for r in occupied.into_iter().chain(std::iter::once(ByteRange::new(
                source.len() as u64,
                source.len() as u64,
            )?)) {
                if end < r.lower_bound {
                    let header = source.get(end as usize..r.lower_bound as usize);
                    let mpvd = header.is_some_and(|h| h.len() == 8 && h.get(4..8) == Some(b"mpvd"));
                    let mut gap = resource(
                        if mpvd {
                            ResourceRole::ContainerMetadata
                        } else {
                            ResourceRole::Unknown
                        },
                        "application/octet-stream",
                        source_index,
                        ByteRange::new(end, r.lower_bound)?,
                    );
                    if mpvd {
                        gap.vendor_role = Some("mpvd header".into());
                    }
                    self.asset.auxiliary.push(gap);
                }
                end = end.max(r.upper_bound);
            }
        }
        for (index, r) in self.asset.auxiliary.iter_mut().enumerate() {
            r.id = ResourceId(index + 2);
            if r.role == ResourceRole::VendorAuxiliaryVideo
                && self.asset.dialect == VendorDialect::Oplus
            {
                r.vendor_role = Some("OPlus concatenated stream 2; purpose unresolved".into());
                r.relationship = Some(ResourceRelationship {
                    kind: RelationshipKind::AuxiliaryTo,
                    target: ResourceId(1),
                });
            }
        }
        let still_kind = if self.asset.still.mime == "image/jpeg" {
            ContainerKind::Jpeg
        } else {
            ContainerKind::Heif
        };
        self.asset.containers.push(crate::MediaContainer {
            resource: self.asset.still.id,
            kind: still_kind,
            movie_timescale: None,
            tracks: Vec::new(),
        });
        let video = crate::tracks::inspect(
            self.motion_video_bytes()?,
            self.asset.motion.id,
            self.asset.dialect == VendorDialect::AndroidStandard,
        )?;
        self.asset.containers.push(video);
        if let Some(time) = self.presentation_time() {
            if !crate::timeline::presentation_in_movie_header(self.motion_video_bytes()?, time)? {
                self.diagnostics
                    .push(Diagnostic::PresentationOutsideMovieHeader);
            }
        }
        let auxiliary: Vec<_> = self
            .asset
            .auxiliary
            .iter()
            .filter(|r| r.mime.starts_with("video/"))
            .cloned()
            .collect();
        for r in auxiliary {
            let raw = self.contiguous_bytes(&r)?;
            let normalized = normalize_embedded_video(raw)?;
            self.asset
                .containers
                .push(crate::tracks::inspect(normalized.data, r.id, false)?);
        }
        Ok(())
    }

    fn contiguous_bytes(&self, resource: &Resource) -> ReadResult<&[u8]> {
        if resource.extents.len() != 1 {
            return Err(MotionPhotoReadError::InvalidResource);
        }
        slice(
            self.source_bytes(resource.source_index)
                .ok_or(MotionPhotoReadError::InvalidResource)?,
            resource.extents[0],
        )
    }

    pub fn still_bytes(&self) -> ReadResult<&[u8]> {
        self.contiguous_bytes(self.still())
    }
    pub fn motion_video_bytes(&self) -> ReadResult<&[u8]> {
        self.contiguous_bytes(self.motion_video())
    }

    /// Copy an asset-owned resource to a caller-controlled sink. No files are
    /// created or overwritten. Original bytes and extent order are preserved.
    pub fn extract(&self, resource: &Resource, mut output: impl Write) -> ReadResult<u64> {
        if !self.resources().any(|owned| owned == resource) {
            return Err(MotionPhotoReadError::InvalidResource);
        }
        let data = self
            .source_bytes(resource.source_index)
            .ok_or(MotionPhotoReadError::InvalidResource)?;
        let mut count = 0u64;
        for range in &resource.extents {
            let bytes = slice(data, *range)?;
            output.write_all(bytes)?;
            count = count
                .checked_add(bytes.len() as u64)
                .ok_or(MotionPhotoReadError::InvalidResource)?;
        }
        Ok(count)
    }

    pub fn resources(&self) -> impl Iterator<Item = &Resource> {
        [&self.asset.still, &self.asset.motion]
            .into_iter()
            .chain(self.asset.auxiliary.iter())
    }

    pub fn validate(&self) -> ReadResult<ValidationReport> {
        let mut ranges = Vec::new();
        for resource in self.resources() {
            let data = self
                .source_bytes(resource.source_index)
                .ok_or(MotionPhotoReadError::InvalidResource)?;
            if resource.extents.is_empty() {
                return Err(MotionPhotoReadError::InvalidResource);
            }
            for range in &resource.extents {
                slice(data, *range)?;
                if let Some(parent_id) = resource.parent {
                    let parent = self
                        .resources()
                        .find(|r| r.id == parent_id)
                        .ok_or(MotionPhotoReadError::InvalidResource)?;
                    if parent.parent.is_some()
                        || parent.source_index != resource.source_index
                        || !parent.extents.iter().any(|p| {
                            p.lower_bound <= range.lower_bound && p.upper_bound >= range.upper_bound
                        })
                    {
                        return Err(MotionPhotoReadError::InvalidResource);
                    }
                } else {
                    ranges.push((resource.source_index, range.lower_bound, range.upper_bound));
                }
            }
        }
        ranges.sort_unstable();
        if ranges
            .windows(2)
            .any(|pair| pair[0].0 == pair[1].0 && pair[0].2 > pair[1].1)
        {
            return Err(MotionPhotoReadError::InvalidResource);
        }
        still_mime(self.still_bytes()?)?;
        let video = self.motion_video_bytes()?;
        if normalize_embedded_video(video)?.removed_vendor_bytes != 0 {
            return Err(MotionPhotoReadError::InvalidResource);
        }
        let mut checks = vec![
            ValidationCheck::ResourceRanges,
            ValidationCheck::StillContainer,
            ValidationCheck::MotionContainer,
            ValidationCheck::Tracks,
        ];
        if let Some(presentation) = self.asset.presentation {
            if presentation.time.timescale == 0 || presentation.time.value < 0 {
                return Err(MotionPhotoReadError::InvalidPresentation);
            }
        }
        if self.asset.format == AssetFormat::AppleLivePhoto {
            let identifier = self
                .asset
                .pairing
                .identifier
                .as_deref()
                .ok_or(MotionPhotoReadError::MissingPairMetadata)?;
            if read_apple_content_identifier(self.still_bytes()?)?.as_deref() != Some(identifier)
                || read_live_photo_content_identifier(video)?.as_deref() != Some(identifier)
            {
                return Err(MotionPhotoReadError::IdentifierMismatch);
            }
            checks.push(ValidationCheck::ApplePairing);
            let time = self
                .presentation_time()
                .ok_or(MotionPhotoReadError::MissingPairMetadata)?;
            validate_live_photo_movie(video, identifier, time.seconds())?;
            checks.push(ValidationCheck::AppleMovieMetadata);
        }
        Ok(ValidationReport {
            checks,
            diagnostics: self.diagnostics.clone(),
        })
    }
}

// Bound candidate allocation before calling the existing scanner. The scanner
// remains the format-specific authority on ftyp plausibility.
fn bounded_candidates(data: &[u8], start: usize, max_candidates: usize) -> ReadResult<Vec<usize>> {
    let mut candidates = Vec::new();
    for (relative, window) in data[start..].windows(8).enumerate() {
        if &window[4..] != b"ftyp" {
            continue;
        }
        let offset = start + relative;
        if crate::is_ftyp_box_start(data, offset as u64, data.len() as u64)? {
            if candidates.len() >= max_candidates {
                return Err(MotionPhotoReadError::ScanLimit);
            }
            candidates.push(offset);
        }
    }
    Ok(candidates)
}

impl MotionPhoto<'static> {
    pub fn open(input: PathInput<'_>, options: ParseOptions) -> ReadResult<Self> {
        fn read(path: &Path, budget: usize) -> ReadResult<Vec<u8>> {
            let file = File::open(path)?;
            if file.metadata()?.len() > budget as u64 {
                return Err(MotionPhotoReadError::InputLimit);
            }
            let mut data = Vec::new();
            file.take((budget as u64).saturating_add(1))
                .read_to_end(&mut data)?;
            if data.len() > budget {
                return Err(MotionPhotoReadError::InputLimit);
            }
            Ok(data)
        }
        let sources = match input {
            PathInput::SingleFile(path) => vec![read(path, options.max_input_bytes)?],
            PathInput::ApplePair { still, movie } => {
                let still = read(still, options.max_input_bytes)?;
                let movie = read(movie, options.max_input_bytes - still.len())?;
                vec![still, movie]
            }
        };
        let input = match sources.as_slice() {
            [data] => Input::SingleFile(data),
            [still, movie] => Input::ApplePair { still, movie },
            _ => return Err(MotionPhotoReadError::Unrecognized),
        };
        let parsed = MotionPhoto::parse(input, options)?;
        let asset = parsed.asset;
        let android_asset = parsed.android_asset;
        let diagnostics = parsed.diagnostics;
        Ok(Self {
            sources: sources.into_iter().map(Cow::Owned).collect(),
            asset,
            android_asset,
            diagnostics,
        })
    }
}
