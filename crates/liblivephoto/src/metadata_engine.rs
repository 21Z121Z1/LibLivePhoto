//! Ordinary-image metadata collections and owned provider registration. Raw bytes
//! and exact source extents remain authoritative; decoded payloads are projections.
use crate::{
    ByteRange, FileRange, HandlerOptions, Input, MotionPhoto, MotionPhotoReadError as Error,
    ParseOptions, XmpDocument,
};
use serde_json::{json, Value};
use std::{fs::File, io::Write, path::Path};
const MAX_METADATA: usize = 16 * 1024 * 1024;
const MAX_JSON_TOKENS: usize = 262_144;

// Bound the generic JSON tree before serde allocates it. Large raw hex strings
// remain permitted; many tiny values, keys or duplicate members do not evade
// this aggregate budget. serde still performs all syntax and depth validation.
fn check_json_budget(bytes: &[u8]) -> Result<(), Error> {
    let (mut quoted, mut escaped, mut tokens) = (false, false, 0usize);
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            if byte == b'"' {
                quoted = true;
            }
            if matches!(byte, b'"' | b'{' | b'[' | b',' | b':') {
                tokens += 1;
                if tokens > MAX_JSON_TOKENS {
                    return Err(Error::InputLimit);
                }
            }
        }
    }
    Ok(())
}
fn range_json(r: ByteRange) -> Value {
    json!({"start":r.lower_bound.to_string(),"end":r.upper_bound.to_string()})
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataBlock {
    pub kind: String,
    pub format_identifier: String,
    pub extents: Vec<ByteRange>,
    pub header_extents: Vec<ByteRange>,
    pub raw_bytes: Vec<u8>,
    pub raw_complete: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderPayload {
    pub value: Value,
    pub motion_declaration: bool,
    pub video_in_block: Option<ByteRange>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedMetadata {
    pub provider: String,
    pub block_index: Option<usize>,
    pub value: Value,
    pub motion_declaration: bool,
    pub video_extents: Vec<ByteRange>,
}
/// Plugins return semantic declarations, not proof of a valid movie. The engine
/// validates relative ranges before translating them through source extents.
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;
    fn decode(
        &self,
        block: &MetadataBlock,
        source: &[u8],
    ) -> Result<Option<ProviderPayload>, Error>;
}
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataCollection {
    pub source_length: u64,
    pub blocks: Vec<MetadataBlock>,
    pub decoded: Vec<DecodedMetadata>,
    pub diagnostics: Vec<String>,
    /// Set only after the existing core validates the actual media in this input.
    pub verified_motion: bool,
}
impl MetadataCollection {
    pub fn declares_motion(&self, options: &HandlerOptions) -> bool {
        self.decoded.iter().any(|d| {
            d.motion_declaration && (d.provider == "google" || options.allows(&d.provider))
        })
    }
    pub fn json(&self) -> Value {
        json!({"schemaVersion":1,"sourceLength":self.source_length.to_string(),"verifiedMotion":self.verified_motion,"blocks":self.blocks.iter().map(|b|json!({"kind":b.kind,"formatIdentifier":b.format_identifier,"extents":b.extents.iter().copied().map(range_json).collect::<Vec<_>>(),"headerExtents":b.header_extents.iter().copied().map(range_json).collect::<Vec<_>>(),"rawBytesHex":encode_hex(&b.raw_bytes),"rawComplete":b.raw_complete})).collect::<Vec<_>>(),"decoded":self.decoded.iter().map(|d|json!({"provider":d.provider,"blockIndex":d.block_index,"value":d.value,"motionDeclaration":d.motion_declaration,"videoExtents":d.video_extents.iter().copied().map(range_json).collect::<Vec<_>>()})).collect::<Vec<_>>(),"diagnostics":self.diagnostics})
    }
    /// Copy the complete block from the caller-retained original source. Large
    /// media/trailer blocks contain only a bounded raw prefix in the manifest.
    /// This checks extents/length, not a cryptographic identity for a new source.
    pub fn extract_block(
        &self,
        index: usize,
        source: &[u8],
        mut output: impl Write,
        max_bytes: u64,
    ) -> Result<u64, Error> {
        if source.len() as u64 != self.source_length {
            return Err(Error::InvalidResource);
        }
        let block = self.blocks.get(index).ok_or(Error::InvalidResource)?;
        // These fields are public and can be constructed without ByteRange::new.
        // Validate the entire block before writing even its first extent.
        if block
            .extents
            .iter()
            .any(|r| r.lower_bound > r.upper_bound || r.upper_bound > source.len() as u64)
        {
            return Err(Error::InvalidResource);
        }
        let size = block
            .extents
            .iter()
            .try_fold(0u64, |n, r| n.checked_add(r.length()))
            .filter(|n| *n <= max_bytes)
            .ok_or(Error::InputLimit)?;
        for range in &block.extents {
            output.write_all(&source[range.lower_bound as usize..range.upper_bound as usize])?;
        }
        Ok(size)
    }
    pub fn write_json(&self, output: impl Write) -> Result<(), Error> {
        serde_json::to_writer(output, &self.json()).map_err(|_| Error::InvalidResource)
    }
    /// A serialized collection is untrusted evidence. Even a persisted true flag
    /// cannot establish current file bytes; verified_motion is cleared on import.
    /// Limited to 64 MiB of JSON, 262144 aggregate structural/string tokens,
    /// 16 MiB of decoded raw bytes and serde's default nesting limit.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(Error::InputLimit);
        }
        check_json_budget(bytes)?;
        let v: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResource)?;
        if v["schemaVersion"] != 1 {
            return Err(Error::UnsupportedLayout("metadata collection schema"));
        }
        let source_length = decimal(&v["sourceLength"])?;
        let array = v["blocks"]
            .as_array()
            .filter(|v| v.len() <= 4096)
            .ok_or(Error::InputLimit)?;
        let mut blocks = Vec::new();
        let mut budget = 0;
        let read_ranges = |v: &Value| -> Result<Vec<ByteRange>, Error> {
            let a = v
                .as_array()
                .filter(|v| v.len() <= 4096)
                .ok_or(Error::InputLimit)?;
            a.iter()
                .map(|v| {
                    let r = ByteRange::new(decimal(&v["start"])?, decimal(&v["end"])?)?;
                    if r.upper_bound > source_length {
                        return Err(Error::InvalidResource);
                    }
                    Ok(r)
                })
                .collect()
        };
        for b in array {
            let encoded = b["rawBytesHex"].as_str().ok_or(Error::InvalidResource)?;
            if encoded.len() % 2 != 0 {
                return Err(Error::InvalidResource);
            }
            budget += encoded.len() / 2;
            if budget > MAX_METADATA {
                return Err(Error::InputLimit);
            }
            let raw = encoded
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| {
                    let digit = |b: u8| match b {
                        b'0'..=b'9' => Some(b - b'0'),
                        b'a'..=b'f' => Some(b - b'a' + 10),
                        b'A'..=b'F' => Some(b - b'A' + 10),
                        _ => None,
                    };
                    Ok(digit(p[0]).ok_or(Error::InvalidResource)? * 16
                        + digit(p[1]).ok_or(Error::InvalidResource)?)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let extents = read_ranges(&b["extents"])?;
            let raw_complete = b["rawComplete"].as_bool().ok_or(Error::InvalidResource)?;
            let size = extents
                .iter()
                .try_fold(0u64, |n, r| n.checked_add(r.length()))
                .ok_or(Error::InvalidResource)?;
            if (raw_complete && size != raw.len() as u64)
                || (!raw_complete && size < raw.len() as u64)
            {
                return Err(Error::InvalidResource);
            }
            blocks.push(MetadataBlock {
                kind: string(&b["kind"])?.into(),
                format_identifier: string(&b["formatIdentifier"])?.into(),
                extents,
                header_extents: read_ranges(&b["headerExtents"])?,
                raw_bytes: raw,
                raw_complete,
            });
        }
        let mut decoded = Vec::new();
        for d in v["decoded"]
            .as_array()
            .filter(|v| v.len() <= 8192)
            .ok_or(Error::InputLimit)?
        {
            let block_index = if d["blockIndex"].is_null() {
                None
            } else {
                Some(
                    d["blockIndex"]
                        .as_u64()
                        .and_then(|n| usize::try_from(n).ok())
                        .filter(|i| *i < blocks.len())
                        .ok_or(Error::InvalidResource)?,
                )
            };
            decoded.push(DecodedMetadata {
                provider: string(&d["provider"])?.into(),
                block_index,
                value: d["value"].clone(),
                motion_declaration: d["motionDeclaration"]
                    .as_bool()
                    .ok_or(Error::InvalidResource)?,
                video_extents: read_ranges(&d["videoExtents"])?,
            });
        }
        Ok(Self {
            source_length,
            blocks,
            decoded,
            diagnostics: vec![
                "serialized semantic payloads are declarations; current media is not verified"
                    .into(),
            ],
            verified_motion: false,
        })
    }
}
fn string(v: &Value) -> Result<&str, Error> {
    v.as_str()
        .filter(|s| s.len() <= 4096)
        .ok_or(Error::InvalidResource)
}
fn decimal(v: &Value) -> Result<u64, Error> {
    string(v)?.parse().map_err(|_| Error::InvalidResource)
}
#[derive(Default)]
pub struct MetadataEngine {
    providers: Vec<Box<dyn MetadataProvider>>,
}
impl MetadataEngine {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, provider: Box<dyn MetadataProvider>) -> Result<(), Error> {
        let n = provider.name();
        if n.is_empty()
            || n.len() > 128
            || [
                "google",
                "legacy_micro_video",
                "samsung_sef",
                "oplus_olive",
                "honor_live_photo",
            ]
            .contains(&n)
            || self.providers.iter().any(|p| p.name() == n)
        {
            return Err(Error::MetadataMismatch("duplicate/reserved provider name"));
        }
        self.providers.push(provider);
        Ok(())
    }
    pub fn parse_path(
        &self,
        path: &Path,
        options: &HandlerOptions,
    ) -> Result<MetadataCollection, Error> {
        let file = File::open(path)?;
        let length = file.metadata()?.len();
        self.parse_range(
            FileRange {
                file: &file,
                offset: 0,
                length,
            },
            options,
        )
    }
    pub fn parse_range(
        &self,
        range: FileRange<'_>,
        options: &HandlerOptions,
    ) -> Result<MetadataCollection, Error> {
        let data = crate::file_input::snapshot(range, 256 * 1024 * 1024)?;
        self.parse(&data, options)
    }
    #[cfg(unix)]
    pub fn parse_fd(
        &self,
        fd: std::os::fd::BorrowedFd<'_>,
        offset: u64,
        length: u64,
        options: &HandlerOptions,
    ) -> Result<MetadataCollection, Error> {
        let file = File::from(fd.try_clone_to_owned()?);
        self.parse_range(
            FileRange {
                file: &file,
                offset,
                length,
            },
            options,
        )
    }
    pub fn parse(
        &self,
        data: &[u8],
        options: &HandlerOptions,
    ) -> Result<MetadataCollection, Error> {
        if data.len() > 256 * 1024 * 1024 {
            return Err(Error::InputLimit);
        }
        let mut blocks = Vec::new();
        let mut diagnostics = Vec::new();
        let mut budget = 0usize;
        let mut push = |kind: &str,
                        identifier: &str,
                        extents: Vec<ByteRange>,
                        header_extents: Vec<ByteRange>|
         -> Result<(), Error> {
            let size = extents
                .iter()
                .try_fold(0usize, |sum, r| {
                    usize::try_from(r.length())
                        .ok()
                        .and_then(|n| sum.checked_add(n))
                })
                .ok_or(Error::InputLimit)?;
            let buffered = if matches!(
                identifier,
                "container.trailer" | "heif.mpvd" | "heic.sefd" | "heif.item"
            ) {
                size.min(65536)
            } else {
                size
            };
            budget = budget
                .checked_add(buffered)
                .filter(|n| *n <= MAX_METADATA)
                .ok_or(Error::InputLimit)?;
            if blocks.len() >= 4096 {
                return Err(Error::InputLimit);
            }
            let mut raw_bytes = Vec::with_capacity(buffered);
            for range in &extents {
                let remaining = buffered - raw_bytes.len();
                if remaining == 0 {
                    break;
                }
                let end = range.upper_bound.min(range.lower_bound + remaining as u64);
                raw_bytes.extend_from_slice(
                    data.get(range.lower_bound as usize..end as usize)
                        .ok_or(Error::InvalidResource)?,
                );
            }
            blocks.push(MetadataBlock {
                kind: kind.into(),
                format_identifier: identifier.into(),
                extents,
                header_extents,
                raw_bytes,
                raw_complete: buffered == size,
            });
            Ok(())
        };
        if data.starts_with(&[255, 216]) {
            let end =
                liblivephoto_format::jpeg_image_end(data, 0).map_err(|_| Error::InvalidStill)?;
            for segment in liblivephoto_format::jpeg_segments::jpeg_metadata_segments(data)
                .map_err(|_| Error::InvalidStill)?
            {
                let raw = &data[segment.payload.clone()];
                let (kind, id, skip) = if raw.starts_with(b"Exif\0\0") {
                    ("EXIF", "standard.exif", 6)
                } else if raw.starts_with(b"http://ns.adobe.com/xap/1.0/\0") {
                    ("XMP", "standard.xmp", 29)
                } else if raw.starts_with(b"ICC_PROFILE\0") && raw.len() >= 14 {
                    ("ICC", "standard.icc", 14)
                } else if raw.starts_with(b"JFIF\0") {
                    ("JFIF", "standard.jfif", 5)
                } else {
                    ("PROPRIETARY", "jpeg.application-or-comment", 0)
                };
                push(
                    kind,
                    id,
                    vec![ByteRange::new(
                        (segment.payload.start + skip) as u64,
                        segment.payload.end as u64,
                    )?],
                    vec![ByteRange::new(
                        segment.range.start as u64,
                        (segment.payload.start + skip) as u64,
                    )?],
                )?;
            }
            if end < data.len() {
                push(
                    "PROPRIETARY",
                    "container.trailer",
                    vec![ByteRange::new(end as u64, data.len() as u64)?],
                    Vec::new(),
                )?;
            }
        } else {
            let top = liblivephoto_format::isobmff::scan_top_level_boxes(data)
                .map_err(|_| Error::InvalidStill)?;
            if top
                .boxes
                .first()
                .is_none_or(|b| b.kind.as_bytes() != b"ftyp")
            {
                return Err(Error::InvalidStill);
            }
            for resource in crate::heif_io::items(data, 0)? {
                let kind = match resource.mime.as_str() {
                    "application/x-exif" => "EXIF",
                    "application/rdf+xml" => "XMP",
                    _ => "PROPRIETARY",
                };
                let identifier = match kind {
                    "EXIF" => "heif.exif-item",
                    "XMP" => "standard.xmp",
                    _ => "heif.item",
                };
                push(kind, identifier, resource.extents, Vec::new())?;
            }
            for b in top.boxes {
                if matches!(b.kind.as_bytes(), b"mpvd" | b"sefd") {
                    push(
                        "PROPRIETARY",
                        if b.kind.as_bytes() == b"mpvd" {
                            "heif.mpvd"
                        } else {
                            "heic.sefd"
                        },
                        vec![ByteRange::new(b.data_start as u64, b.data_end as u64)?],
                        vec![ByteRange::new(b.box_start as u64, b.data_start as u64)?],
                    )?;
                }
            }
        }
        let mut decoded = Vec::new();
        for (index, b) in blocks.iter().enumerate() {
            if b.kind == "XMP" {
                if options.allows("oplus_olive") {
                    match crate::inspect_oplus_xmp_metadata(&b.raw_bytes) {
                        Ok(Some(m)) => decoded.push(DecodedMetadata {
                            provider: "oplus_olive".into(), block_index: Some(index),
                            motion_declaration: m.version.is_some_and(|v| v > 0) || m.declared_video_length.is_some(),
                            value: json!({"version":m.version,"declaredVideoLength":m.declared_video_length.map(|n|n.to_string()),"photoTime":m.photo_time.map(|t|json!({"value":t.value.to_string(),"timescale":t.timescale})),"source":"xmp-declaration"}),
                            video_extents: Vec::new(),
                        }),
                        Ok(None) => {},
                        Err(e) => diagnostics.push(format!("OpCamera XMP block {index}: {e}")),
                    }
                }
                match XmpDocument::parse(&b.raw_bytes) {
                    Ok(document) => {
                        let fields = document.fields();
                        let legacy = fields.camera_values.get("MicroVideo") == Some(&1)
                            && fields.camera_values.get("MotionPhoto") != Some(&1);
                        let provider = if legacy {
                            "legacy_micro_video"
                        } else {
                            "google"
                        };
                        if !legacy || options.allows(provider) {
                            decoded.push(DecodedMetadata{provider:provider.into(),block_index:Some(index),motion_declaration:fields.camera_values.get("MotionPhoto")==Some(&1)||legacy,value:json!({"camera":fields.camera_values.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>(),"directory":fields.directory.iter().map(|i|json!({"mime":i.mime,"semantic":i.semantic,"length":i.length.to_string(),"padding":i.padding.to_string()})).collect::<Vec<_>>()}),video_extents:Vec::new()});
                        }
                    }
                    Err(e) => diagnostics.push(format!("XMP block {index}: {e}")),
                }
            }
            for provider in self.providers.iter().filter(|p| options.allows(p.name())) {
                match provider.decode(b, data) {
                    Ok(Some(payload)) => {
                        let video_extents = payload
                            .video_in_block
                            .map(|r| {
                                subrange(
                                    &b.extents,
                                    r,
                                    usize::try_from(
                                        b.extents.iter().map(|e| e.length()).sum::<u64>(),
                                    )
                                    .map_err(|_| Error::InputLimit)?,
                                )
                            })
                            .transpose()?
                            .unwrap_or_default();
                        decoded.push(DecodedMetadata {
                            provider: provider.name().into(),
                            block_index: Some(index),
                            value: payload.value,
                            motion_declaration: payload.motion_declaration,
                            video_extents,
                        });
                    }
                    Ok(None) => {}
                    Err(e) => {
                        diagnostics.push(format!("provider {} block {index}: {e}", provider.name()))
                    }
                }
            }
        }
        let mut native = |name: &str, value: Value, video: Option<ByteRange>, declaration: bool| {
            decoded.push(DecodedMetadata {
                provider: name.into(),
                block_index: None,
                value,
                motion_declaration: declaration,
                video_extents: video.into_iter().collect(),
            })
        };
        if options.allows("oplus_olive") {
            match crate::inspect_oplus_metadata(data) {
                Ok(Some(m)) => native(
                    "oplus_olive",
                    json!({"version":m.version,"declaredVideoLength":m.declared_video_length.map(|v|v.to_string()),"photoTime":m.photo_time.map(|t|json!({"value":t.value.to_string(),"timescale":t.timescale})),"jxrsFooterField":m.jxrs_footer_field,"checksumBytes":m.checksum_bytes,"checksumVerified":false,"records":m.records.iter().map(|r|json!({"name":r.name,"range":range_json(r.range),"raw":r.raw})).collect::<Vec<_>>()}),
                    m.video,
                    m.video.is_some() || m.version.is_some_and(|v| v > 0),
                ),
                Ok(None) => {}
                Err(e) => diagnostics.push(format!("oplus_olive: {e}")),
            }
        }
        if options.allows("honor_live_photo") {
            match crate::inspect_honor_metadata(data) {
                Ok(Some(m)) => native(
                    "honor_live_photo",
                    json!({"variant":format!("{:?}",m.variant),"sceneMode":m.scene_mode,"photoTime":m.presentation.map(|t|json!({"value":t.value.to_string(),"timescale":t.timescale})),"records":m.records.iter().map(|r|json!({"tag":r.tag,"range":range_json(r.payload)})).collect::<Vec<_>>()}),
                    m.video,
                    m.video.is_some(),
                ),
                Ok(None) => {}
                Err(e) => diagnostics.push(format!("honor_live_photo: {e}")),
            }
        }
        if options.allows("samsung_sef") {
            match crate::inspect_samsung_metadata(data) {
                Ok(Some(m)) => native(
                    "samsung_sef",
                    json!({"version":m.version,"motionPhotoDataMagic":m.motion_photo_data_magic.map(|m|String::from_utf8_lossy(&m).into_owned()),"records":m.records.iter().map(|r|json!({"name":r.name,"kind":r.kind,"payload":range_json(r.payload),"raw":range_json(r.raw)})).collect::<Vec<_>>()}),
                    m.video,
                    m.video.is_some(),
                ),
                Ok(None) => {}
                Err(e) => diagnostics.push(format!("samsung_sef: {e}")),
            }
        }
        let verified_motion = match MotionPhoto::parse_with_handlers(
            Input::SingleFile(data),
            ParseOptions::compatible(),
            options,
        ) {
            Ok(_) => true,
            Err(e) => {
                if decoded.iter().any(|d| d.motion_declaration) {
                    diagnostics.push(format!("motion declaration could not be validated: {e}"));
                }
                false
            }
        };
        Ok(MetadataCollection {
            source_length: data.len() as u64,
            blocks,
            decoded,
            diagnostics,
            verified_motion,
        })
    }
}
fn subrange(
    extents: &[ByteRange],
    range: ByteRange,
    length: usize,
) -> Result<Vec<ByteRange>, Error> {
    if range.lower_bound > range.upper_bound || range.upper_bound > length as u64 {
        return Err(Error::InvalidResource);
    }
    let mut cursor = 0;
    let mut output = Vec::new();
    for e in extents {
        let next = cursor + e.length();
        let start = cursor.max(range.lower_bound);
        let end = next.min(range.upper_bound);
        if start < end {
            output.push(ByteRange::new(
                e.lower_bound + start - cursor,
                e.lower_bound + end - cursor,
            )?);
        }
        cursor = next;
    }
    if output.iter().map(|r| r.length()).sum::<u64>() != range.length() {
        return Err(Error::InvalidResource);
    }
    Ok(output)
}

fn encode_hex(data: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for b in data {
        out.push(HEX[usize::from(b >> 4)] as char);
        out.push(HEX[usize::from(b & 15)] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opcamera_xmp_in_real_heic_is_a_declaration_without_a_fabricated_movie() {
        let still = include_bytes!("../../../web/public/acceptance/still.heic");
        let declaration = r#"<rdf:Description xmlns:o="http://ns.oplus.com/photo/1.0/camera/" o:OLivePhotoVersion="2" o:VideoLength="19000" o:PrimaryPresentationTimestampUs="812500"/>"#;
        let image = crate::heif_io::replace_xmp(still, declaration).unwrap();
        let collection = MetadataEngine::new()
            .parse(&image, &HandlerOptions::default())
            .unwrap();
        let decoded = collection
            .decoded
            .iter()
            .find(|p| p.provider == "oplus_olive")
            .unwrap();
        assert!(decoded.motion_declaration);
        assert!(decoded.block_index.is_some());
        assert_eq!(decoded.value["photoTime"]["value"], "812500");
        assert!(decoded.video_extents.is_empty());
        assert!(!collection.verified_motion);
        let disabled = HandlerOptions {
            disable_third_party: true,
            ..HandlerOptions::default()
        };
        assert!(!MetadataEngine::new()
            .parse(&image, &disabled)
            .unwrap()
            .declares_motion(&disabled));
    }
}
