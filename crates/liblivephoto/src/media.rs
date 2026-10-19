//! Observed display, color and codec facts. Missing metadata stays unknown.
use crate::{MotionPhoto, MotionPhotoReadError as Error, TrackKind};
use liblivephoto_format::{isobmff::*, FourCC};
type Result<T> = std::result::Result<T, Error>;
fn fmt<T>(value: liblivephoto_format::Result<T>) -> Result<T> {
    value.map_err(|_| Error::InvalidResource)
}
fn u16_at(p: &[u8], i: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(
        p.get(i..i + 2)
            .ok_or(Error::InvalidResource)?
            .try_into()
            .map_err(|_| Error::InvalidResource)?,
    ))
}
fn u32_at(p: &[u8], i: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        p.get(i..i + 4)
            .ok_or(Error::InvalidResource)?
            .try_into()
            .map_err(|_| Error::InvalidResource)?,
    ))
}

/// Linear part of an orthogonal display transform in x-right/y-down coordinates.
/// Translation and crop are not implied. Apply EXIF and HEIF declarations under
/// their respective container rules; they are not blindly composed together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrthogonalTransform {
    pub matrix: [i8; 4],
}
impl OrthogonalTransform {
    pub fn from_exif(value: u16) -> Option<Self> {
        Some(Self {
            matrix: match value {
                1 => [1, 0, 0, 1],
                2 => [-1, 0, 0, 1],
                3 => [-1, 0, 0, -1],
                4 => [1, 0, 0, -1],
                5 => [0, 1, 1, 0],
                6 => [0, -1, 1, 0],
                7 => [0, -1, -1, 0],
                8 => [0, 1, -1, 0],
                _ => return None,
            },
        })
    }
    fn then(self, next: Self) -> Self {
        let [a, b, c, d] = self.matrix;
        let [e, f, g, h] = next.matrix;
        Self {
            matrix: [e * a + f * c, e * b + f * d, g * a + h * c, g * b + h * d],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProperty {
    pub kind: [u8; 4],
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorDescription {
    Cicp {
        primaries: u16,
        transfer: u16,
        matrix: u16,
        full_range: Option<bool>,
    },
    Icc {
        kind: [u8; 4],
        profile: Vec<u8>,
    },
    Uninterpreted(Vec<u8>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdrTransfer {
    Pq,
    Hlg,
}
impl ColorDescription {
    /// Only an explicit transfer characteristic identifies PQ or HLG. No tag
    /// does not mean SDR. ICC profiles are retained, not converted to CICP.
    pub fn hdr_transfer(&self) -> Option<HdrTransfer> {
        match self {
            Self::Cicp { transfer: 16, .. } => Some(HdrTransfer::Pq),
            Self::Cicp { transfer: 18, .. } => Some(HdrTransfer::Hlg),
            _ => None,
        }
    }
}
fn color(p: &[u8]) -> Result<ColorDescription> {
    match p.get(..4) {
        Some(b"nclx") if p.len() == 11 => Ok(ColorDescription::Cicp {
            primaries: u16_at(p, 4)?,
            transfer: u16_at(p, 6)?,
            matrix: u16_at(p, 8)?,
            full_range: Some(p[10] & 0x80 != 0),
        }),
        Some(b"nclc") if p.len() == 10 => Ok(ColorDescription::Cicp {
            primaries: u16_at(p, 4)?,
            transfer: u16_at(p, 6)?,
            matrix: u16_at(p, 8)?,
            full_range: None,
        }),
        Some(b"rICC" | b"prof") => Ok(ColorDescription::Icc {
            kind: p[..4].try_into().map_err(|_| Error::InvalidResource)?,
            profile: p[4..].to_vec(),
        }),
        Some(b"nclx" | b"nclc") => {
            Err(Error::UnsupportedLayout("malformed CICP color declaration"))
        }
        _ => Ok(ColorDescription::Uninterpreted(p.to_vec())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StillMetadata {
    pub exif_orientation: Option<u16>,
    pub exif_transform: Option<OrthogonalTransform>,
    pub heif_transform: Option<OrthogonalTransform>,
    /// Primary-item property association order, including unrecognized geometry,
    /// codec configuration and HDR boxes. These bytes are not a rendered result.
    pub primary_properties: Vec<RawProperty>,
    pub color: Vec<ColorDescription>,
}

impl MotionPhoto<'_> {
    pub fn still_metadata(&self) -> Result<StillMetadata> {
        inspect_still(self.still_bytes()?, &self.still().mime)
    }
}

pub(crate) fn inspect_still(data: &[u8], mime: &str) -> Result<StillMetadata> {
    let mut out = StillMetadata::default();
    let tiff = if mime == "image/jpeg" {
        fmt(liblivephoto_format::jpeg_exif_tiff(data))?
    } else {
        fmt(liblivephoto_format::heif_exif_tiff(data))?
    };
    out.exif_orientation = tiff
        .as_ref()
        .map(|t| fmt(liblivephoto_format::exif::read_tiff_orientation_value(t)))
        .transpose()?
        .flatten();
    out.exif_transform = out
        .exif_orientation
        .and_then(OrthogonalTransform::from_exif);
    if mime == "image/jpeg" {
        if let Some(profile) = fmt(liblivephoto_format::jpeg_icc_profile(data))? {
            out.color.push(ColorDescription::Icc {
                kind: *b"prof",
                profile,
            });
        }
        return Ok(out);
    }
    let top = fmt(parse_boxes(data, 0..data.len()))?;
    let meta = top
        .iter()
        .find(|b| b.kind.as_bytes() == b"meta")
        .ok_or(Error::InvalidStill)?;
    let parsed = fmt(parse_meta_box(data, meta))?;
    let mut transform = OrthogonalTransform {
        matrix: [1, 0, 0, 1],
    };
    let mut transformed = false;
    let mut property_bytes = 0usize;
    for item in parsed
        .ipma
        .entries
        .iter()
        .filter(|e| e.item_id == parsed.primary_item_id)
    {
        for a in &item.associations {
            if a.property_index == 0 {
                continue;
            }
            let property = parsed
                .properties
                .iter()
                .find(|p| p.index == u32::from(a.property_index))
                .ok_or(Error::InvalidResource)?;
            let b = fmt(parse_boxes(data, property.box_range.clone()))?
                .into_iter()
                .next()
                .ok_or(Error::InvalidResource)?;
            let p = &data[b.payload_range()];
            let kind = *b.kind.as_bytes();
            property_bytes = property_bytes
                .checked_add(p.len())
                .filter(|n| *n <= 16 * 1024 * 1024)
                .ok_or(Error::InputLimit)?;
            if out.primary_properties.len() >= 4096 {
                return Err(Error::InputLimit);
            }
            out.primary_properties.push(RawProperty {
                kind,
                bytes: p.to_vec(),
            });
            match &kind {
                b"irot" if p.len() == 1 && p[0] & !3 == 0 => {
                    for _ in 0..p[0] {
                        transform = transform.then(OrthogonalTransform {
                            matrix: [0, 1, -1, 0],
                        });
                    }
                    transformed = true;
                }
                b"imir" if p.len() == 1 && p[0] & !1 == 0 => {
                    transform = transform.then(OrthogonalTransform {
                        matrix: if p[0] == 0 {
                            [1, 0, 0, -1]
                        } else {
                            [-1, 0, 0, 1]
                        },
                    });
                    transformed = true;
                }
                b"irot" | b"imir" => {
                    return Err(Error::UnsupportedLayout("invalid HEIF transform property"))
                }
                b"colr" => out.color.push(color(p)?),
                _ => {}
            }
        }
    }
    out.heif_transform = transformed.then_some(transform);
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AacConfiguration {
    pub object_type: u8,
    pub sample_rate: u32,
    /// Channel configuration 0 requires a program-config element. It is unknown.
    pub channels: Option<u16>,
    pub raw: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioParameters {
    /// AudioSampleEntry version; QuickTime v2 layouts are retained, not decoded.
    pub sample_entry_version: u16,
    pub sample_entry_channels: u16,
    pub sample_entry_sample_size: u16,
    /// Exact 16.16 fixed-point field. It is not rounded to an integer rate.
    pub sample_entry_rate_16_16: u32,
    pub aac: Option<AacConfiguration>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoDimensions {
    /// Coded sample-entry dimensions, before tkhd matrix, crop or pixel aspect.
    pub width: u16,
    pub height: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleDescription {
    pub track_id: u32,
    pub index: usize,
    pub kind: TrackKind,
    pub format: [u8; 4],
    pub dimensions: Option<VideoDimensions>,
    pub audio: Option<AudioParameters>,
    pub color: Vec<ColorDescription>,
    /// Video codec configuration, color, HDR and unknown child boxes, in order.
    pub properties: Vec<RawProperty>,
    /// Exact tkhd matrix: a,b,c,d,x,y use 16.16; u,v,w use 2.30.
    /// Array order is a,b,u,c,d,v,x,y,w. No floating-point conversion is used.
    pub track_matrix: [i32; 9],
}

fn descriptor(p: &[u8]) -> Result<(u8, &[u8], &[u8])> {
    let tag = *p.first().ok_or(Error::InvalidResource)?;
    let mut len = 0usize;
    for i in 1..=4 {
        let v = *p.get(i).ok_or(Error::InvalidResource)?;
        len = len
            .checked_mul(128)
            .and_then(|n| n.checked_add(usize::from(v & 127)))
            .ok_or(Error::InvalidResource)?;
        if v & 128 == 0 {
            let end = (i + 1)
                .checked_add(len)
                .filter(|e| *e <= p.len())
                .ok_or(Error::InvalidResource)?;
            return Ok((tag, &p[i + 1..end], &p[end..]));
        }
    }
    Err(Error::InvalidResource)
}
fn decoder_config(esds: &[u8]) -> Result<Option<&[u8]>> {
    let (tag, es, trailing) = descriptor(esds.get(4..).ok_or(Error::InvalidResource)?)?;
    if !trailing.is_empty() {
        return Err(Error::UnsupportedLayout("multiple esds root descriptors"));
    }
    if tag != 3 || es.len() < 3 {
        return Err(Error::InvalidResource);
    }
    let flags = es[2];
    let mut pos = 3;
    if flags & 128 != 0 {
        pos += 2;
    }
    if flags & 64 != 0 {
        pos += 1 + usize::from(*es.get(pos).ok_or(Error::InvalidResource)?);
    }
    if flags & 32 != 0 {
        pos += 2;
    }
    let (tag, config, _) = descriptor(es.get(pos..).ok_or(Error::InvalidResource)?)?;
    if tag != 4 || config.len() < 13 {
        return Err(Error::InvalidResource);
    }
    if config[0] != 0x40 {
        return Ok(None);
    }
    let mut rest = &config[13..];
    let mut specific = None;
    while !rest.is_empty() {
        let (tag, value, next) = descriptor(rest)?;
        if tag == 5 && specific.replace(value).is_some() {
            return Err(Error::UnsupportedLayout(
                "duplicate AAC DecoderSpecificInfo",
            ));
        }
        rest = next;
    }
    Ok(specific)
}
struct Bits<'a> {
    data: &'a [u8],
    bit: usize,
}
impl Bits<'_> {
    fn take(&mut self, n: usize) -> Result<u32> {
        let end = self
            .bit
            .checked_add(n)
            .filter(|v| *v <= self.data.len().saturating_mul(8))
            .ok_or(Error::InvalidResource)?;
        let mut value = 0;
        for i in self.bit..end {
            value = value * 2 + u32::from((self.data[i / 8] >> (7 - i % 8)) & 1);
        }
        self.bit = end;
        Ok(value)
    }
    fn rate(&mut self) -> Result<u32> {
        let index = self.take(4)? as usize;
        if index == 15 {
            return self.take(24).and_then(|v| {
                if v > 0 {
                    Ok(v)
                } else {
                    Err(Error::InvalidResource)
                }
            });
        }
        [
            96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
        ]
        .get(index)
        .copied()
        .ok_or(Error::InvalidResource)
    }
}
fn aac(raw: &[u8]) -> Result<AacConfiguration> {
    let mut bits = Bits { data: raw, bit: 0 };
    let object = bits.take(5)?;
    let object = if object == 31 {
        32 + bits.take(6)?
    } else {
        object
    };
    let mut sample_rate = bits.rate()?;
    let channel = bits.take(4)?;
    let channels = match channel {
        0 => None,
        1..=6 => Some(channel as u16),
        7 => Some(8),
        _ => {
            return Err(Error::UnsupportedLayout(
                "unsupported AAC channel configuration",
            ))
        }
    };
    if object == 5 || object == 29 {
        sample_rate = bits.rate()?;
        let _base_object_type = bits.take(5)?;
    }
    Ok(AacConfiguration {
        object_type: object as u8,
        sample_rate,
        channels,
        raw: raw.to_vec(),
    })
}

impl MotionPhoto<'_> {
    /// Read sample descriptions and declared display/color facts. Codec decoding,
    /// AAC PCE parsing, QuickTime v2 audio descriptions and codec-level VUI are
    /// outside this contract. Missing configuration does not imply compatibility.
    pub fn sample_descriptions(&self) -> Result<Vec<SampleDescription>> {
        inspect_video_sample_descriptions(self.motion_video_bytes()?)
    }
}

/// Declared sample-entry facts for a standalone MP4/MOV. No frame/audio decoding
/// or generic bitstream conformance is implied. Bounded to256MiB input,4096
/// descriptions,65536 properties and16MiB aggregate referenced property bytes.
pub fn inspect_video_sample_descriptions(data: &[u8]) -> Result<Vec<SampleDescription>> {
    if data.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    let container = crate::tracks::inspect(data, crate::ResourceId(0), false)?;
    let top = fmt(parse_boxes(data, 0..data.len()))?;
    let moov = top
        .iter()
        .find(|b| b.kind.as_bytes() == b"moov")
        .ok_or(Error::InvalidResource)?;
    let children = fmt(parse_boxes(data, moov.payload_range()))?;
    let mut out = Vec::new();
    let mut property_bytes = 0usize;
    let mut property_count = 0usize;
    for (track, trak) in container
        .tracks
        .iter()
        .zip(children.iter().filter(|b| b.kind.as_bytes() == b"trak"))
    {
        let tc = fmt(parse_boxes(data, trak.payload_range()))?;
        let tkhd = tc
            .iter()
            .find(|b| b.kind.as_bytes() == b"tkhd")
            .ok_or(Error::InvalidResource)?;
        let p = &data[tkhd.payload_range()];
        let matrix_start = match p.first() {
            Some(0) => 40,
            Some(1) => 52,
            _ => return Err(Error::InvalidResource),
        };
        let mut matrix = [0; 9];
        for (i, v) in matrix.iter_mut().enumerate() {
            *v = u32_at(p, matrix_start + i * 4)? as i32;
        }
        let mut parent = trak.clone();
        for kind in [b"mdia", b"minf", b"stbl", b"stsd"] {
            parent = fmt(parse_boxes(data, parent.payload_range()))?
                .into_iter()
                .find(|b| b.kind.as_bytes() == kind)
                .ok_or(Error::InvalidResource)?;
        }
        for (index, entry) in fmt(parse_boxes(data, parent.data_start + 8..parent.data_end))?
            .iter()
            .enumerate()
        {
            if out.len() >= 4096 {
                return Err(Error::InputLimit);
            }
            let p = &data[entry.payload_range()];
            let mut desc = SampleDescription {
                track_id: track.id,
                index,
                kind: track.kind,
                format: *entry.kind.as_bytes(),
                dimensions: None,
                audio: None,
                color: Vec::new(),
                properties: Vec::new(),
                track_matrix: matrix,
            };
            let child_start = match track.kind {
                TrackKind::Video => {
                    desc.dimensions = Some(VideoDimensions {
                        width: u16_at(p, 24)?,
                        height: u16_at(p, 26)?,
                    });
                    Some(78)
                }
                TrackKind::Audio if p.len() >= 28 => {
                    desc.audio = Some(AudioParameters {
                        sample_entry_version: u16_at(p, 8)?,
                        sample_entry_channels: u16_at(p, 16)?,
                        sample_entry_sample_size: u16_at(p, 18)?,
                        sample_entry_rate_16_16: u32_at(p, 24)?,
                        aac: None,
                    });
                    match u16_at(p, 8)? {
                        0 => Some(28),
                        1 => Some(44),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(start) = child_start {
                let mut esds_seen = false;
                for b in fmt(parse_boxes(data, entry.data_start + start..entry.data_end))? {
                    let p = &data[b.payload_range()];
                    property_bytes = property_bytes
                        .checked_add(p.len())
                        .filter(|n| *n <= 16 * 1024 * 1024)
                        .ok_or(Error::InputLimit)?;
                    property_count += 1;
                    if property_count > 65536 {
                        return Err(Error::InputLimit);
                    }
                    desc.properties.push(RawProperty {
                        kind: *b.kind.as_bytes(),
                        bytes: p.to_vec(),
                    });
                    if b.kind == FourCC::new(*b"colr") {
                        desc.color.push(color(p)?);
                    }
                    if b.kind == FourCC::new(*b"esds") && desc.format == *b"mp4a" {
                        if esds_seen {
                            return Err(Error::UnsupportedLayout(
                                "duplicate audio esds declarations",
                            ));
                        }
                        esds_seen = true;
                        if let Some(raw) = decoder_config(p)? {
                            if let Some(audio) = &mut desc.audio {
                                audio.aac = Some(aac(raw)?);
                            }
                        }
                    }
                }
            }
            out.push(desc);
        }
    }
    Ok(out)
}

/// A caller-supplied target contract. These values are not inferred from a vendor
/// name and do not prove device acceptance. Every sample description is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCapabilities {
    pub video_sample_entries: Vec<[u8; 4]>,
    pub audio_sample_entries: Vec<[u8; 4]>,
    pub aac_object_types: Vec<u8>,
    pub max_audio_channels: u16,
    pub audio_sample_rates: Vec<u32>,
}

/// Inspect a standalone JPEG/HEIC/AVIF still without inventing a motion partner.
pub fn inspect_image_metadata(data: &[u8]) -> Result<StillMetadata> {
    if data.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    inspect_still(data, crate::facade::still_mime(data)?)
}
