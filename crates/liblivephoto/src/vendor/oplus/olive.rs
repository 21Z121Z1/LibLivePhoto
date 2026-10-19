//! Bounded OpCamera / JXRS provider. Raw footer fields and directory records are
//! retained; a digest is not asserted valid without a specified algorithm.
use crate::{ByteRange, MediaTime, MotionPhotoReadError as Error};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader, XmlVersion};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct OplusRecord {
    pub name: String,
    pub range: ByteRange,
    pub raw: Value,
}
#[derive(Debug, Clone, PartialEq)]
pub struct OplusMetadata {
    pub version: Option<i64>,
    pub declared_video_length: Option<u64>,
    pub photo_time: Option<MediaTime>,
    pub video: Option<ByteRange>,
    pub jxrs_footer_field: Option<u32>,
    pub checksum_bytes: Option<Vec<u8>>,
    pub directory: Option<ByteRange>,
    pub records: Vec<OplusRecord>,
}
fn numeric(xml: &[u8]) -> Result<Option<BTreeMap<String, i64>>, Error> {
    if xml.len() > 4 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    let mut reader = NsReader::from_reader(xml);
    let mut fields = BTreeMap::new();
    let mut current = None;
    let mut text = String::new();
    let mut marker = false;
    let mut depth = 0usize;
    fn key(
        reader: &NsReader<&[u8]>,
        name: quick_xml::name::QName<'_>,
        attr: bool,
    ) -> Option<(String, bool)> {
        let (ns, local) = if attr {
            reader.resolver().resolve_attribute(name)
        } else {
            reader.resolver().resolve_element(name)
        };
        let ResolveResult::Bound(ns) = ns else {
            return None;
        };
        let oplus = matches!(
            ns.as_ref(),
            "http://ns.oplus.com/photo/1.0/camera/" | "http://ns.oppo.com/photo/1.0/camera/"
        );
        if !oplus && ns.as_ref() != "http://ns.google.com/photos/1.0/camera/" {
            return None;
        }
        let name = local.as_ref();
        matches!(
            name,
            "OLivePhotoVersion"
                | "VideoLength"
                | "MotionPhotoPrimaryPresentationTimestampUs"
                | "MotionPhotoPresentationTimestampUs"
                | "PrimaryPresentationTimestampUs"
        )
        .then(|| (name.into(), oplus))
    }
    fn put(fields: &mut BTreeMap<String, i64>, key: String, text: &str) -> Result<(), Error> {
        let value = text
            .trim()
            .parse::<i64>()
            .map_err(|_| Error::MetadataMismatch("OpCamera numeric field"))?;
        if fields.insert(key, value).is_some_and(|old| old != value) {
            return Err(Error::MetadataMismatch("conflicting OpCamera fields"));
        }
        Ok(())
    }
    loop {
        let event = reader
            .read_event()
            .map_err(|_| Error::MetadataMismatch("OpCamera XML"))?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                if current.is_some() {
                    return Err(Error::MetadataMismatch("nested OpCamera scalar"));
                }
                if let Some((name, is_oplus)) = key(&reader, e.name(), false) {
                    marker |= is_oplus;
                    if current.is_some() {
                        return Err(Error::MetadataMismatch("nested OpCamera scalar"));
                    }
                    current = Some(name);
                    text.clear();
                }
                for (index, a) in e.attributes().with_checks(true).enumerate() {
                    if index >= 256 {
                        return Err(Error::InputLimit);
                    }
                    let a = a.map_err(|_| Error::MetadataMismatch("OpCamera attribute"))?;
                    if let Some((name, is_oplus)) = key(&reader, a.key, true) {
                        marker |= is_oplus;
                        let value = a
                            .normalized_value(XmlVersion::Implicit1_0)
                            .map_err(|_| Error::MetadataMismatch("OpCamera value"))?;
                        put(&mut fields, name, &value)?;
                    }
                }
                if !empty {
                    depth += 1;
                } else if let Some(key) = current.take() {
                    put(&mut fields, key, "")?;
                }
                if depth > 512 {
                    return Err(Error::InputLimit);
                }
            }
            Event::Text(e) => {
                if current.is_some() {
                    let part = e.xml_content(XmlVersion::Implicit1_0);
                    if text.len() + part.len() > 128 {
                        return Err(Error::InputLimit);
                    }
                    text.push_str(&part);
                }
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if let Some(key) = current.take() {
                    put(&mut fields, key, &text)?;
                }
            }
            Event::DocType(_) => return Err(Error::MetadataMismatch("OpCamera DTD")),
            Event::Eof => {
                if depth != 0 || current.is_some() {
                    return Err(Error::MetadataMismatch("unclosed OpCamera XML"));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(marker.then_some(fields))
}
/// Inspect the complete JPEG asset. Returned extents are absolute source offsets.
/// This identifies metadata-only OpCamera declarations separately from a movie.
pub fn inspect_oplus_metadata(data: &[u8]) -> Result<Option<OplusMetadata>, Error> {
    if !data.starts_with(&[255, 216]) {
        return Ok(None);
    }
    let end = liblivephoto_format::jpeg_image_end(data, 0).map_err(|_| Error::InvalidStill)?;
    let xml = crate::jpeg_write::read_xmp(data).map_err(|_| Error::InvalidStill)?;
    let fields = xml.map(numeric).transpose()?.flatten();
    let tail = &data[end..];
    let footer = if tail.len() >= 8 && &tail[tail.len() - 8..tail.len() - 4] == b"jxrs" {
        Some(u32::from_le_bytes(
            tail[tail.len() - 4..]
                .try_into()
                .map_err(|_| Error::InvalidResource)?,
        ))
    } else {
        None
    };
    if fields.is_none() && footer.is_none() {
        return Ok(None);
    }
    let fields = fields.unwrap_or_default();
    let mut result = OplusMetadata {
        version: fields.get("OLivePhotoVersion").copied(),
        declared_video_length: fields
            .get("VideoLength")
            .map(|v| {
                u64::try_from(*v)
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or(Error::MetadataMismatch("OpCamera video length"))
            })
            .transpose()?,
        photo_time: None,
        video: None,
        jxrs_footer_field: footer,
        checksum_bytes: None,
        directory: None,
        records: Vec::new(),
    };
    for key in [
        "MotionPhotoPrimaryPresentationTimestampUs",
        "MotionPhotoPresentationTimestampUs",
        "PrimaryPresentationTimestampUs",
    ] {
        if let Some(value) = fields.get(key) {
            if *value < -1 {
                return Err(Error::InvalidPresentation);
            }
            if *value >= 0 {
                let t = MediaTime {
                    value: *value,
                    timescale: 1_000_000,
                };
                if result.photo_time.replace(t).is_some_and(|old| old != t) {
                    return Err(Error::MetadataMismatch("OpCamera timestamps"));
                }
            }
        }
    }
    if tail.is_empty() {
        return Ok(Some(result));
    }
    let mut video_end = None;
    if let Some(raw) = footer {
        let n = raw as usize;
        if (2..=65536).contains(&n) {
            let candidates = [
                tail.len().checked_sub(n),
                tail.len().checked_sub(8).and_then(|v| v.checked_sub(n)),
            ];
            for start in candidates.into_iter().flatten() {
                if start >= tail.len() - 8 || tail[start] != b'[' {
                    continue;
                }
                let bytes = &tail[start..tail.len() - 8];
                let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
                let value: Value = serde_json::from_slice(bytes)
                    .map_err(|_| Error::MetadataMismatch("JXRS directory JSON"))?;
                let list = value
                    .as_array()
                    .filter(|a| a.len() <= 1024)
                    .ok_or(Error::InputLimit)?;
                let mut max_back = 0usize;
                for raw in list {
                    let offset = usize::try_from(
                        raw.get("offset")
                            .and_then(Value::as_u64)
                            .ok_or(Error::InvalidResource)?,
                    )
                    .map_err(|_| Error::InvalidResource)?;
                    let len = usize::try_from(
                        raw.get("length")
                            .and_then(Value::as_u64)
                            .ok_or(Error::InvalidResource)?,
                    )
                    .map_err(|_| Error::InvalidResource)?;
                    let lower = start.checked_sub(offset).ok_or(Error::InvalidResource)?;
                    let upper = lower
                        .checked_add(len)
                        .filter(|u| *u <= start)
                        .ok_or(Error::InvalidResource)?;
                    if len == 0 || offset == 0 {
                        return Err(Error::InvalidResource);
                    }
                    max_back = max_back.max(offset);
                    result.records.push(OplusRecord {
                        name: raw
                            .get("name")
                            .and_then(Value::as_str)
                            .filter(|s| s.len() <= 4096)
                            .ok_or(Error::InvalidResource)?
                            .into(),
                        range: ByteRange::new((end + lower) as u64, (end + upper) as u64)?,
                        raw: raw.clone(),
                    });
                }
                let boundary = start.checked_sub(max_back).ok_or(Error::InvalidResource)?;
                let mut spans = result.records.iter().map(|r| r.range).collect::<Vec<_>>();
                spans.sort_by_key(|r| r.lower_bound);
                if spans
                    .windows(2)
                    .any(|s| s[0].upper_bound > s[1].lower_bound)
                {
                    return Err(Error::InvalidResource);
                }
                video_end = Some(boundary);
                result.directory = Some(ByteRange::new(
                    (end + start) as u64,
                    (data.len() - 8) as u64,
                )?);
                break;
            }
        }
        if video_end.is_none() {
            if let Some(length) = result.declared_video_length {
                video_end = Some(usize::try_from(length).map_err(|_| Error::InvalidResource)?);
            } else if tail.len() > 68 {
                let boundary = tail.len() - 68;
                // Treat this as a fixed footer only if its candidate preceding
                // bytes are a complete BMFF, avoiding blind deletion of 60 bytes.
                if liblivephoto_format::isobmff::parse_boxes(tail, 0..boundary).is_ok() {
                    video_end = Some(boundary);
                    result.checksum_bytes = Some(tail[boundary..boundary + 60].to_vec());
                }
            }
            if video_end.is_none() {
                video_end = tail.len().checked_sub(8);
            }
        }
    } else if let Some(length) = result.declared_video_length {
        video_end = Some(usize::try_from(length).map_err(|_| Error::InvalidResource)?);
    } else {
        video_end = Some(tail.len());
    }
    if let Some(boundary) = video_end {
        if let Some(declared) = result.declared_video_length {
            if declared != boundary as u64 {
                return Err(Error::MetadataMismatch(
                    "OpCamera length and JXRS directory",
                ));
            }
        }
        if boundary == 0
            || boundary
                > tail
                    .len()
                    .saturating_sub(if footer.is_some() { 8 } else { 0 })
        {
            return Err(Error::InvalidResource);
        }
        let boxes = liblivephoto_format::isobmff::parse_boxes(tail, 0..boundary)
            .map_err(|_| Error::InvalidResource)?;
        if boxes.first().is_none_or(|b| {
            !matches!(
                b.kind.as_bytes(),
                b"ftyp" | b"moov" | b"mdat" | b"free" | b"skip" | b"wide"
            )
        }) {
            return Err(Error::InvalidResource);
        }
        result.video = Some(ByteRange::new(end as u64, (end + boundary) as u64)?);
    }
    Ok(Some(result))
}
