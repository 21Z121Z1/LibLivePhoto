//! Metadata splices retain JPEG scan bytes and remap MPF references.
use crate::{
    ByteRange, MediaTime, MotionPhotoReadError as Error, Provenance, Resource, ResourceId,
    ResourceRole,
};
use quick_xml::{
    events::{BytesStart, Event},
    name::ResolveResult,
    reader::NsReader,
    Writer,
};
use std::ops::Range;
type Result<T> = std::result::Result<T, Error>;
const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const CAMERA: &str = "http://ns.google.com/photos/1.0/camera/";
const CONTAINER: &str = "http://ns.google.com/photos/1.0/container/";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
fn motion_property(local: &str) -> bool {
    matches!(
        local,
        "MotionPhoto"
            | "MotionPhotoVersion"
            | "MotionPhotoPresentationTimestampUs"
            | "MicroVideo"
            | "MicroVideoVersion"
            | "MicroVideoOffset"
            | "MicroVideoPresentationTimestampUs"
    )
}

type JpegSegment = (u8, Range<usize>, Range<usize>);
fn segments(jpeg: &[u8]) -> Result<Vec<JpegSegment>> {
    if !jpeg.starts_with(&[255, 216]) {
        return Err(Error::InvalidStill);
    }
    let mut out = Vec::new();
    let mut pos = 2usize;
    while pos < jpeg.len() {
        let start = pos;
        if jpeg[pos] != 255 {
            return Err(Error::InvalidStill);
        }
        while jpeg.get(pos) == Some(&255) {
            pos += 1;
        }
        let marker = *jpeg.get(pos).ok_or(Error::InvalidStill)?;
        pos += 1;
        if marker == 218 || marker == 217 {
            break;
        }
        if marker == 1 || (208..=215).contains(&marker) {
            continue;
        }
        let len = usize::from(u16::from_be_bytes(
            jpeg.get(pos..pos + 2)
                .and_then(|v| v.try_into().ok())
                .ok_or(Error::InvalidStill)?,
        ));
        if len < 2 {
            return Err(Error::InvalidStill);
        }
        let end = pos
            .checked_add(len)
            .filter(|v| *v <= jpeg.len())
            .ok_or(Error::InvalidStill)?;
        if out.len() >= 4096 {
            return Err(Error::InvalidStill);
        }
        out.push((marker, start..end, pos + 2..end));
        pos = end;
    }
    Ok(out)
}
fn app1(payload: &[u8]) -> Result<Vec<u8>> {
    let len = u16::try_from(payload.len() + 2).map_err(|_| Error::InvalidStill)?;
    Ok([vec![255, 225], len.to_be_bytes().to_vec(), payload.to_vec()].concat())
}
fn is_ns(
    reader: &NsReader<&[u8]>,
    name: quick_xml::name::QName<'_>,
    uri: &str,
    local: &str,
) -> bool {
    let (ns, l) = reader.resolver().resolve_element(name);
    matches!(ns,ResolveResult::Bound(n) if n.as_ref()==uri) && l.as_ref() == local
}
fn retain_attributes(e: &BytesStart<'_>, reader: &NsReader<&[u8]>) -> Result<BytesStart<'static>> {
    let mut out = e.to_owned();
    out.clear_attributes();
    for a in e.attributes() {
        let a = a.map_err(|_| Error::InvalidStill)?;
        let (ns, local) = reader.resolver().resolve_attribute(a.key);
        if matches!(ns,ResolveResult::Bound(n) if n.as_ref()==CAMERA)
            && motion_property(local.as_ref())
        {
            continue;
        }
        out.push_attribute(a);
    }
    Ok(out)
}

pub(crate) fn rewrite_packet(packet: &[u8], description: &str) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(packet).map_err(|_| Error::InvalidStill)?;
    let mut reader = NsReader::from_str(text);
    let mut writer = Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut skip = 0usize;
    let mut inserted = false;
    loop {
        let e = reader.read_event().map_err(|_| Error::InvalidStill)?;
        match e {
            Event::Start(ref s) => {
                depth += 1;
                if depth > 128 {
                    return Err(Error::InvalidStill);
                }
                if skip > 0 {
                    skip += 1;
                    continue;
                }
                let (ns, local) = reader.resolver().resolve_element(s.name());
                if is_ns(&reader, s.name(), CONTAINER, "Directory")
                    || (matches!(ns,ResolveResult::Bound(n) if n.as_ref()==CAMERA)
                        && motion_property(local.as_ref()))
                {
                    skip = 1;
                    continue;
                }
                writer.write_event(Event::Start(retain_attributes(s, &reader)?))?;
            }
            Event::Empty(ref s) => {
                if skip > 0 {
                    continue;
                }
                let (ns, local) = reader.resolver().resolve_element(s.name());
                if is_ns(&reader, s.name(), CONTAINER, "Directory")
                    || (matches!(ns,ResolveResult::Bound(n) if n.as_ref()==CAMERA)
                        && motion_property(local.as_ref()))
                {
                    continue;
                }
                writer.write_event(Event::Empty(retain_attributes(s, &reader)?))?;
            }
            Event::End(ref s) => {
                depth = depth.checked_sub(1).ok_or(Error::InvalidStill)?;
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                if is_ns(&reader, s.name(), RDF, "RDF") {
                    if inserted {
                        return Err(Error::InvalidStill);
                    }
                    writer.get_mut().extend_from_slice(description.as_bytes());
                    inserted = true;
                }
                writer.write_event(e)?;
            }
            Event::DocType(_) => return Err(Error::InvalidStill),
            Event::Eof => break,
            _ => {
                if skip == 0 {
                    writer.write_event(e)?;
                }
            }
        }
    }
    if !inserted || depth != 0 {
        return Err(Error::InvalidStill);
    }
    Ok(writer.into_inner())
}

fn be_or_le_u32(bytes: &[u8], offset: usize, le: bool) -> Result<u32> {
    let a = bytes
        .get(offset..offset + 4)
        .and_then(|v| v.try_into().ok())
        .ok_or(Error::InvalidStill)?;
    Ok(if le {
        u32::from_le_bytes(a)
    } else {
        u32::from_be_bytes(a)
    })
}
fn put_u32(bytes: &mut [u8], offset: usize, value: u32, le: bool) -> Result<()> {
    let target = bytes
        .get_mut(offset..offset + 4)
        .ok_or(Error::InvalidStill)?;
    target.copy_from_slice(&if le {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    });
    Ok(())
}
fn map_position(pos: usize, change: &Range<usize>, new_len: usize) -> Result<usize> {
    if pos < change.start {
        Ok(pos)
    } else if pos >= change.end {
        pos.checked_sub(change.len())
            .and_then(|v| v.checked_add(new_len))
            .ok_or(Error::InvalidStill)
    } else {
        Err(Error::InvalidStill)
    }
}
fn splice(jpeg: &[u8], change: Range<usize>, bytes: &[u8]) -> Result<Vec<u8>> {
    let mut output = [
        jpeg[..change.start].to_vec(),
        bytes.to_vec(),
        jpeg[change.end..].to_vec(),
    ]
    .concat();
    // MPF offsets are relative to the MPF TIFF header, not to file start.
    for (marker, _, p) in segments(jpeg)? {
        if marker != 226 || !jpeg[p.clone()].starts_with(b"MPF\0") {
            continue;
        }
        let old_tiff = p.start + 4;
        let new_tiff = map_position(old_tiff, &change, bytes.len())?;
        let tiff = &jpeg[old_tiff..p.end];
        let le = match tiff.get(..4) {
            Some(b"II*\0") => true,
            Some(b"MM\0*") => false,
            _ => return Err(Error::InvalidStill),
        };
        let ifd = be_or_le_u32(tiff, 4, le)? as usize;
        let raw = tiff
            .get(ifd..ifd + 2)
            .and_then(|v| v.try_into().ok())
            .ok_or(Error::InvalidStill)?;
        let count = usize::from(if le {
            u16::from_le_bytes(raw)
        } else {
            u16::from_be_bytes(raw)
        });
        if count > 256 {
            return Err(Error::InvalidStill);
        }
        for n in 0..count {
            let entry = ifd + 2 + n * 12;
            let raw = tiff
                .get(entry..entry + 2)
                .and_then(|v| v.try_into().ok())
                .ok_or(Error::InvalidStill)?;
            let tag = if le {
                u16::from_le_bytes(raw)
            } else {
                u16::from_be_bytes(raw)
            };
            if tag != 0xb002 {
                continue;
            }
            let len = be_or_le_u32(tiff, entry + 4, le)? as usize;
            let start = be_or_le_u32(tiff, entry + 8, le)? as usize;
            if len == 0
                || !len.is_multiple_of(16)
                || len > 65536
                || start.checked_add(len).is_none_or(|end| end > tiff.len())
            {
                return Err(Error::InvalidStill);
            }
            for mp in (start..start + len).step_by(16) {
                let offset = be_or_le_u32(tiff, mp + 8, le)? as usize;
                if offset != 0 {
                    let absolute = old_tiff
                        .checked_add(offset)
                        .filter(|v| *v < jpeg.len())
                        .ok_or(Error::InvalidStill)?;
                    let mapped = map_position(absolute, &change, bytes.len())?
                        .checked_sub(new_tiff)
                        .ok_or(Error::InvalidStill)?;
                    put_u32(
                        &mut output,
                        new_tiff + mp + 8,
                        u32::try_from(mapped).map_err(|_| Error::InvalidStill)?,
                        le,
                    )?;
                } else {
                    let size = be_or_le_u32(tiff, mp + 4, le)? as usize;
                    if size != 0 {
                        let end = map_position(size, &change, bytes.len())?;
                        put_u32(
                            &mut output,
                            new_tiff + mp + 4,
                            u32::try_from(end).map_err(|_| Error::InvalidStill)?,
                            le,
                        )?;
                    }
                }
            }
        }
    }
    Ok(output)
}

pub(crate) fn write_xmp(jpeg: &[u8], description: &str) -> Result<Vec<u8>> {
    let found: Vec<_> = segments(jpeg)?
        .into_iter()
        .filter(|(m, _, p)| *m == 225 && jpeg[p.clone()].starts_with(XMP))
        .collect();
    if found.len() > 1 {
        return Err(Error::InvalidStill);
    }
    let (change, packet) = if let Some((_, s, p)) = found.first() {
        (
            s.clone(),
            rewrite_packet(&jpeg[p.start + XMP.len()..p.end], description)?,
        )
    } else {
        (2..2,format!("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"{RDF}\">{description}</rdf:RDF></x:xmpmeta>").into_bytes())
    };
    let packet = app1(&[XMP, packet.as_slice()].concat())?;
    splice(jpeg, change, &packet)
}

pub(crate) fn read_xmp(jpeg: &[u8]) -> Result<Option<&[u8]>> {
    let mut selected = None;
    for (m, _, p) in segments(jpeg)? {
        if m == 225 && jpeg[p.clone()].starts_with(XMP) {
            if selected.is_some() {
                return Err(Error::InvalidStill);
            }
            selected = Some(&jpeg[p.start + XMP.len()..p.end]);
        }
    }
    Ok(selected)
}

pub(crate) fn metadata_resources(jpeg: &[u8], start_id: usize) -> Result<Vec<Resource>> {
    let mut resources = Vec::new();
    for (m, _, p) in segments(jpeg)? {
        if !(224..=239).contains(&m) {
            continue;
        }
        let name = if m == 225 && jpeg[p.clone()].starts_with(b"Exif\0\0") {
            "JPEG EXIF".to_owned()
        } else if m == 225 && jpeg[p.clone()].starts_with(XMP) {
            "JPEG XMP".to_owned()
        } else if m == 226 && jpeg[p.clone()].starts_with(b"MPF\0") {
            "JPEG MPF".to_owned()
        } else {
            format!("JPEG APP{}", m - 224)
        };
        resources.push(Resource {
            id: ResourceId(start_id + resources.len()),
            role: ResourceRole::ContainerMetadata,
            mime: "application/octet-stream".into(),
            source_index: 0,
            extents: vec![ByteRange::new(p.start as u64, p.end as u64)?],
            parent: Some(ResourceId(0)),
            container_item_id: None,
            relationship: None,
            vendor_role: Some(name),
            provenance: Provenance::observed("JPEG application segment"),
        });
    }
    Ok(resources)
}

pub(crate) fn encoded_payload(jpeg: &[u8]) -> Result<Vec<u8>> {
    let end = liblivephoto_format::jpeg_image_end(jpeg, 0).map_err(|_| Error::InvalidStill)?;
    let headers = segments(jpeg)?;
    let mut bytes = Vec::new();
    let mut pos = 2;
    for (m, s, _) in headers {
        if !(224..=239).contains(&m) {
            bytes.extend_from_slice(&jpeg[s.clone()]);
        }
        pos = s.end;
    }
    bytes.extend_from_slice(&jpeg[pos..end]);
    bytes.extend_from_slice(&jpeg[end..]);
    Ok(bytes)
}

pub(crate) fn android_description(
    video_len: usize,
    mime: &str,
    time: Option<MediaTime>,
    items: &[(String, String, u64)],
) -> Result<String> {
    let us = time
        .map(|t| t.rescale_exact(1_000_000).ok_or(Error::InvalidPresentation))
        .transpose()?
        .map_or(-1, |t| t.value);
    let esc = |s: &str| quick_xml::escape::escape(s).into_owned();
    let padding = if mime == "image/jpeg" { 0 } else { 8 };
    let mut d=format!("<rdf:Description xmlns:rdf=\"{RDF}\" xmlns:Camera=\"{CAMERA}\" xmlns:Container=\"{CONTAINER}\" xmlns:Item=\"http://ns.google.com/photos/1.0/container/item/\" Camera:MotionPhoto=\"1\" Camera:MotionPhotoVersion=\"1\" Camera:MotionPhotoPresentationTimestampUs=\"{us}\"><Container:Directory><rdf:Seq><rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"{}\" Item:Semantic=\"Primary\" Item:Length=\"0\" Item:Padding=\"{padding}\"/></rdf:li>",esc(mime));
    for (mime, semantic, len) in items {
        d.push_str(&format!("<rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"{}\" Item:Semantic=\"{}\" Item:Length=\"{len}\"/></rdf:li>",esc(mime),esc(semantic)));
    }
    d.push_str(&format!("<rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"video/mp4\" Item:Semantic=\"MotionPhoto\" Item:Length=\"{video_len}\"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description>"));
    Ok(d)
}

pub(crate) fn apple_still(
    jpeg: &[u8],
    identifier: &str,
    externalize_exif: bool,
) -> Result<Vec<u8>> {
    let mut tiff = crate::live_photo_still::build_live_photo_jpeg_exif(jpeg, identifier)?;
    if tiff.len() + 6 > 65533 {
        if !externalize_exif {
            return Err(Error::ConversionBlocked(vec!["Apple pairing exceeds the JPEG EXIF APP1 capacity; preserve original EXIF in a sidecar".into()]));
        }
        let original = liblivephoto_format::jpeg_exif_tiff(jpeg)
            .map_err(|_| Error::InvalidStill)?
            .ok_or(Error::InvalidStill)?;
        // Preserve the raw SHORT value, including invalid vendor values such as 0.
        // A conversion must not invent a rotation to repair unrelated metadata.
        let le = original.starts_with(b"II");
        let ifd = be_or_le_u32(&original, 4, le)? as usize;
        let count_bytes = original
            .get(ifd..ifd + 2)
            .and_then(|b| b.try_into().ok())
            .ok_or(Error::InvalidStill)?;
        let count = if le {
            u16::from_le_bytes(count_bytes)
        } else {
            u16::from_be_bytes(count_bytes)
        };
        let mut orientation = None;
        for n in 0..usize::from(count) {
            let e = original
                .get(ifd + 2 + n * 12..ifd + 14 + n * 12)
                .ok_or(Error::InvalidStill)?;
            let word = |p: usize| {
                if le {
                    u16::from_le_bytes([e[p], e[p + 1]])
                } else {
                    u16::from_be_bytes([e[p], e[p + 1]])
                }
            };
            if word(0) == 0x0112 {
                if word(2) != 3 || be_or_le_u32(e, 4, le)? != 1 || orientation.is_some() {
                    return Err(Error::InvalidStill);
                }
                orientation = Some(word(8));
            }
        }
        let maker = crate::live_photo_still::build_apple_makernote(identifier)?;
        tiff = b"II*\0\x08\0\0\0".to_vec();
        let exif_offset = if orientation.is_some() { 38 } else { 26 };
        tiff.extend_from_slice(&(if orientation.is_some() { 2u16 } else { 1 }).to_le_bytes());
        let entries = orientation
            .map(|o| (0x0112u16, 3u16, 1u32, u32::from(o)))
            .into_iter()
            .chain(std::iter::once((0x8769, 4, 1, exif_offset)));
        for (tag, kind, count, value) in entries {
            tiff.extend_from_slice(&tag.to_le_bytes());
            tiff.extend_from_slice(&kind.to_le_bytes());
            tiff.extend_from_slice(&count.to_le_bytes());
            tiff.extend_from_slice(&value.to_le_bytes());
        }
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x927cu16.to_le_bytes());
        tiff.extend_from_slice(&7u16.to_le_bytes());
        tiff.extend_from_slice(&(maker.len() as u32).to_le_bytes());
        tiff.extend_from_slice(&(exif_offset + 18).to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(&maker);
    }
    let payload = [b"Exif\0\0".as_slice(), tiff.as_slice()].concat();
    let found: Vec<_> = segments(jpeg)?
        .into_iter()
        .filter(|(m, _, p)| *m == 225 && jpeg[p.clone()].starts_with(b"Exif\0\0"))
        .collect();
    if found.len() > 1 {
        return Err(Error::InvalidStill);
    }
    let change = found.first().map(|(_, s, _)| s.clone()).unwrap_or(2..2);
    let output = splice(jpeg, change, &app1(&payload)?)?;
    let d = format!(
        "<rdf:Description xmlns:rdf=\"{RDF}\" xmlns:Camera=\"{CAMERA}\" Camera:MotionPhoto=\"0\"/>"
    );
    write_xmp(&output, &d)
}
