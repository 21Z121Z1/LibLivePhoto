//! HEIF item locations and append-only XMP replacement.
use crate::{
    ByteRange, MotionPhotoReadError as Error, Provenance, Resource, ResourceId, ResourceRole,
};
use liblivephoto_format::{exif::read_item_payload, isobmff::*, FourCC};
type Result<T> = std::result::Result<T, Error>;
fn fmt<T>(r: liblivephoto_format::Result<T>) -> Result<T> {
    r.map_err(|_| Error::InvalidStill)
}
fn meta(data: &[u8]) -> Result<BoxHeader> {
    let top = fmt(scan_top_level_boxes(data))?;
    let metas: Vec<_> = top
        .boxes
        .into_iter()
        .filter(|b| b.kind.as_bytes() == b"meta")
        .collect();
    if metas.len() != 1 {
        return Err(Error::InvalidStill);
    }
    Ok(metas[0].clone())
}
fn mime_xmp(data: &[u8], i: &ItemInfo) -> bool {
    i.item_type == Some(FourCC::new(*b"mime"))
        && data[i.box_range.clone()]
            .windows(b"application/rdf+xml\0".len())
            .any(|w| w == b"application/rdf+xml\0")
}
fn primary_xmp<'a>(
    data: &[u8],
    info: &'a IinfBox,
    primary: u32,
    iref: Option<&IrefBox>,
) -> Result<Option<&'a ItemInfo>> {
    let all: Vec<_> = info.entries.iter().filter(|i| mime_xmp(data, i)).collect();
    let related: Vec<_> = all
        .iter()
        .copied()
        .filter(|i| {
            iref.is_some_and(|r| {
                r.entries.iter().any(|e| {
                    e.kind.as_bytes() == b"cdsc"
                        && e.from_item_id == i.item_id
                        && e.to_item_ids.contains(&primary)
                })
            })
        })
        .collect();
    if related.len() == 1 {
        return Ok(Some(related[0]));
    }
    if related.len() > 1 || all.len() > 1 {
        return Err(Error::InvalidStill);
    }
    Ok(all.first().copied())
}
pub(crate) fn read_xmp(data: &[u8]) -> Result<Option<Vec<u8>>> {
    let m = meta(data)?;
    let parsed = fmt(parse_meta_box(data, &m))?;
    let Some(item) = primary_xmp(
        data,
        &parsed.iinf,
        parsed.primary_item_id,
        parsed.iref.as_ref(),
    )?
    else {
        return Ok(None);
    };
    let entry = parsed
        .iloc
        .entries
        .iter()
        .find(|e| e.item_id == item.item_id)
        .ok_or(Error::InvalidStill)?;
    let mut size = 0u64;
    for e in &entry.extents {
        size = size.checked_add(e.length).ok_or(Error::InvalidStill)?;
    }
    if size > 4 * 1024 * 1024 {
        return Err(Error::InvalidStill);
    }
    fmt(read_item_payload(data, entry, parsed.idat.as_ref())).map(Some)
}

pub(crate) fn items(data: &[u8], start_id: usize) -> Result<Vec<Resource>> {
    let m = meta(data)?;
    let parsed = fmt(parse_meta_box(data, &m))?;
    let mut resources = Vec::new();
    let xmp_id = primary_xmp(
        data,
        &parsed.iinf,
        parsed.primary_item_id,
        parsed.iref.as_ref(),
    )?
    .map(|i| i.item_id);
    for info in &parsed.iinf.entries {
        let Some(entry) = parsed
            .iloc
            .entries
            .iter()
            .find(|e| e.item_id == info.item_id)
        else {
            continue;
        };
        if entry.data_reference_index != 0 {
            return Err(Error::InvalidResource);
        }
        let base = match entry.construction_method {
            0 => 0,
            1 => {
                parsed
                    .idat
                    .as_ref()
                    .ok_or(Error::InvalidResource)?
                    .data_start as u64
            }
            _ => return Err(Error::InvalidResource),
        };
        let mut extents = Vec::new();
        for e in &entry.extents {
            if e.length == 0 {
                continue;
            }
            let start = base
                .checked_add(
                    entry
                        .resolved_extent_offset(e)
                        .map_err(|_| Error::InvalidResource)?,
                )
                .ok_or(Error::InvalidResource)?;
            let end = start
                .checked_add(e.length)
                .filter(|v| *v <= data.len() as u64)
                .ok_or(Error::InvalidResource)?;
            extents.push(ByteRange::new(start, end)?);
        }
        if extents.is_empty() {
            continue;
        }
        let kind = info
            .item_type
            .map(|k| String::from_utf8_lossy(k.as_bytes()).into_owned())
            .unwrap_or_else(|| "unknown".into());
        let image = matches!(kind.as_str(), "hvc1" | "av01" | "jpeg" | "grid" | "iden");
        resources.push(Resource {
            id: ResourceId(start_id + resources.len()),
            role: if image {
                ResourceRole::AuxiliaryImage
            } else {
                ResourceRole::Unknown
            },
            mime: if image {
                format!("image/heif-item;type={kind}")
            } else if kind == "Exif" {
                "application/x-exif".into()
            } else if mime_xmp(data, info) {
                "application/rdf+xml".into()
            } else {
                "application/octet-stream".into()
            },
            source_index: 0,
            extents,
            parent: Some(ResourceId(0)),
            container_item_id: Some(info.item_id),
            relationship: None,
            vendor_role: Some(format!("HEIF item {} ({kind})", info.item_id)),
            provenance: Provenance::declared(if xmp_id == Some(info.item_id) {
                "HEIF primary XMP item"
            } else {
                "HEIF iinf/iloc item location"
            }),
        });
    }
    Ok(resources)
}

pub(crate) fn replace_xmp(data: &[u8], description: &str) -> Result<Vec<u8>> {
    let packet=match read_xmp(data)? {
        Some(old)=>crate::jpeg_write::rewrite_packet(&old,description)?,
        None=>format!("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">{description}</rdf:RDF></x:xmpmeta>").into_bytes(),
    };
    replace_metadata(data, &packet, false)
}

pub(crate) fn apple_still(data: &[u8], identifier: &str) -> Result<Vec<u8>> {
    let old = fmt(liblivephoto_format::heif_exif_tiff(data))?;
    let maker = crate::live_photo_still::pairing_makernote(old.as_deref(), identifier)?;
    let tiff = fmt(liblivephoto_format::replace_exif_makernote(
        old.as_deref(),
        &maker,
    ))?;
    let output = replace_metadata(data, &[vec![0; 4], tiff].concat(), true)?;
    let description="<rdf:Description xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:Camera=\"http://ns.google.com/photos/1.0/camera/\" Camera:MotionPhoto=\"0\"/>";
    replace_xmp(&output, description)
}

fn replace_metadata(data: &[u8], packet: &[u8], exif: bool) -> Result<Vec<u8>> {
    let m = meta(data)?;
    let children = fmt(parse_boxes(data, m.data_start + 4..m.data_end))?;
    let iloc_header = children
        .iter()
        .find(|b| b.kind.as_bytes() == b"iloc")
        .ok_or(Error::InvalidStill)?;
    let iinf_header = children
        .iter()
        .find(|b| b.kind.as_bytes() == b"iinf")
        .ok_or(Error::InvalidStill)?;
    let mut locations = fmt(parse_iloc(data, iloc_header))?;
    let info = fmt(parse_iinf(data, iinf_header))?;
    let parsed = fmt(parse_meta_box(data, &m))?;
    let exifs: Vec<_> = info
        .entries
        .iter()
        .filter(|i| i.item_type == Some(FourCC::new(*b"Exif")))
        .collect();
    if exif && exifs.len() > 1 {
        return Err(Error::InvalidStill);
    }
    let existing = if exif {
        exifs.first().copied()
    } else {
        primary_xmp(data, &info, parsed.primary_item_id, parsed.iref.as_ref())?
    };
    let id = match existing {
        Some(i) => i.item_id,
        None => info
            .entries
            .iter()
            .map(|i| i.item_id)
            .chain(locations.entries.iter().map(|i| i.item_id))
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::InvalidStill)?,
    };
    let mut infe: Vec<_> = info
        .entries
        .iter()
        .map(|i| data[i.box_range.clone()].to_vec())
        .collect();
    if existing.is_none() {
        let mut payload = id.to_be_bytes().to_vec();
        payload.extend_from_slice(&[0, 0]);
        payload.extend_from_slice(if exif {
            b"Exif\0".as_slice()
        } else {
            b"mime\0application/rdf+xml\0\0"
        });
        infe.push(fmt(make_full_box(FourCC::new(*b"infe"), 3, 0, &payload))?);
    }
    locations.entries.retain(|e| e.item_id != id);
    locations.entries.push(IlocEntry {
        item_id: id,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: (data.len() + 8) as u64,
            length: packet.len() as u64,
        }],
    });
    let new_iloc = fmt(make_iloc_box(
        2,
        8,
        8,
        8,
        locations.index_size,
        &locations.entries,
    ))?;
    let new_iinf = fmt(make_iinf_box(1, &infe))?;
    let mut references = parsed
        .iref
        .as_ref()
        .map_or_else(Vec::new, |r| r.entries.clone());
    if existing.is_none() {
        references.push(IrefEntry {
            kind: FourCC::new(*b"cdsc"),
            from_item_id: id,
            to_item_ids: vec![parsed.primary_item_id],
        });
    }
    let new_iref = fmt(make_iref_box(1, &references))?;
    let mut wrote_iref = false;
    let mut payload = data[m.data_start..m.data_start + 4].to_vec();
    for child in children {
        if child.kind.as_bytes() == b"iloc" {
            payload.extend_from_slice(&new_iloc);
        } else if child.kind.as_bytes() == b"iinf" {
            payload.extend_from_slice(&new_iinf);
        } else if child.kind.as_bytes() == b"iref" {
            payload.extend_from_slice(&new_iref);
            wrote_iref = true;
        } else {
            payload.extend_from_slice(&data[child.box_range()]);
        }
    }
    if !wrote_iref {
        payload.extend_from_slice(&new_iref);
    }
    let new_meta = fmt(make_box(FourCC::new(*b"meta"), &payload))?;
    let mut output = data.to_vec();
    // Replace meta with an equal-size free box. All absolute source item offsets remain valid.
    output[m.box_start + 4..m.box_start + 8].copy_from_slice(b"free");
    output[m.data_start..m.data_end].fill(0);
    // A size-zero final box must become explicit before bytes are appended.
    for b in fmt(parse_boxes(data, 0..data.len()))? {
        if data[b.box_start..b.box_start + 4] == [0, 0, 0, 0] {
            let size = u32::try_from(b.size).map_err(|_| Error::InvalidStill)?;
            output[b.box_start..b.box_start + 4].copy_from_slice(&size.to_be_bytes());
        }
    }
    output.extend_from_slice(&fmt(make_box(FourCC::new(*b"mdat"), packet))?);
    output.extend_from_slice(&new_meta);
    Ok(output)
}
