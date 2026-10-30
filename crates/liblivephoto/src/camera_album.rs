//! Read the bounded CameraAlbum envelope observed in independent vivo originals.
//! No vendor renderer, pair lookup, frame-rate guess or native writer is implied.
use crate::{ByteRange, MotionPhotoReadError as Error};
use serde_json::Value;
use std::collections::BTreeSet;

const MAGIC: &[u8] = &[
    0x1b, 0x2a, 0x39, 0x48, 0x57, 0x66, 0x75, 0x84, 0x93, 0xa2, 0xb3,
];
const MARKER: &[u8] = b"cameralbum!";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAlbumLayout {
    LengthDelimited43,
    ///55-byte tail/length2=2 is supported for raw inspection from an independent
    /// parser's static contract; no original-device acceptance is claimed.
    Observed55ByteTail,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAlbumPrefix {
    Vivo,
    VivoMediaExtInfo,
    Unspecified,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredIdentifierAgreement {
    Absent,
    Equal,
    Different,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraAlbumFooter<'a> {
    /// Absolute ranges relative to this function's immutable input snapshot.
    pub extent: ByteRange,
    pub json_extent: ByteRange,
    pub identifier_extent: ByteRange,
    pub layout: CameraAlbumLayout,
    pub prefix: CameraAlbumPrefix,
    pub length2: u32,
    pub envelope_identifier: &'a str,
    pub declared_identifier: Option<String>,
    /// Equality is a declaration check, never proof of capture-pair uniqueness.
    pub identifier_agreement: DeclaredIdentifierAgreement,
    pub version: Option<i64>,
    /// Raw integer; the independent parser interprets this as a frame ordinal.
    /// That unit is not promoted to a timestamp without verified sample mapping.
    pub image_time: Option<i64>,
    pub raw_json: &'a [u8],
}

fn mismatch(message: &'static str) -> Error {
    Error::MetadataMismatch(message)
}

// Validate small JSON's structural budget and duplicate keys before Value's map
// can discard a conflicting declaration. String keys are decoded, so Unicode
// escape aliases cannot evade equality. serde still validates the full grammar.
fn json_fields(bytes: &[u8]) -> Result<Value, Error> {
    if bytes.len() > 65536 {
        return Err(Error::InputLimit);
    }
    let mut stack: Vec<Option<BTreeSet<String>>> = Vec::new();
    let (mut pos, mut tokens) = (0usize, 0usize);
    while pos < bytes.len() {
        match bytes[pos] {
            b'{' | b'[' => {
                tokens += 1;
                stack.push((bytes[pos] == b'{').then(BTreeSet::new));
                if stack.len() > 32 {
                    return Err(Error::InputLimit);
                }
            }
            b'}' | b']' => {
                let object = stack
                    .pop()
                    .ok_or_else(|| mismatch("CameraAlbum JSON nesting"))?
                    .is_some();
                if object != (bytes[pos] == b'}') {
                    return Err(mismatch("CameraAlbum JSON nesting"));
                }
            }
            b',' | b':' => tokens += 1,
            b'"' => {
                tokens += 1;
                let start = pos;
                pos += 1;
                let mut escaped = false;
                while pos < bytes.len() {
                    let byte = bytes[pos];
                    if !escaped && byte == b'"' {
                        break;
                    }
                    escaped = !escaped && byte == b'\\';
                    pos += 1;
                }
                if pos == bytes.len() {
                    return Err(mismatch("CameraAlbum JSON string"));
                }
                let next = bytes[pos + 1..].iter().find(|b| !b.is_ascii_whitespace());
                if next == Some(&b':') {
                    let key: String = serde_json::from_slice(&bytes[start..=pos])
                        .map_err(|_| mismatch("CameraAlbum JSON key"))?;
                    let keys = stack
                        .last_mut()
                        .and_then(Option::as_mut)
                        .ok_or_else(|| mismatch("CameraAlbum JSON object key"))?;
                    if !keys.insert(key) {
                        return Err(mismatch("duplicate CameraAlbum JSON key"));
                    }
                }
            }
            _ => {}
        }
        if tokens > 8192 {
            return Err(Error::InputLimit);
        }
        pos += 1;
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| mismatch("CameraAlbum JSON syntax"))?;
    if !value.is_object() {
        return Err(mismatch("CameraAlbum JSON object required"));
    }
    Ok(value)
}
fn integer(fields: &Value, key: &str) -> Result<Option<i64>, Error> {
    fields
        .get(key)
        .map(|value| {
            value
                .as_i64()
                .ok_or_else(|| mismatch("CameraAlbum integer declaration"))
        })
        .transpose()
}

/// Inspect only a terminal, exactly framed envelope. Known layouts are a43-byte
/// identifier/magic tail with length2=47, or the explicitly labelled55-byte
/// static-evidence variant with length2=2 and a12-byte binary prefix. At most
///128KiB of tail is considered, JSON<=64KiB/32 nesting levels/8192 structural
/// tokens, input<=256MiB. Known malformed fields fail; unrelated bytes return None.
/// This establishes neither a real movie nor vendor/native-album compatibility.
pub fn inspect_camera_album_footer(data: &[u8]) -> Result<Option<CameraAlbumFooter<'_>>, Error> {
    if data.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    if !data.ends_with(MAGIC) {
        return Ok(None);
    }
    let candidate = [
        (43usize, CameraAlbumLayout::LengthDelimited43),
        (55, CameraAlbumLayout::Observed55ByteTail),
    ]
    .into_iter()
    .find_map(|(tail, layout)| {
        let marker = data.len().checked_sub(tail + 15)?;
        (data.get(marker..marker + 11) == Some(MARKER)).then_some((marker, tail, layout))
    });
    let Some((marker, tail_length, layout)) = candidate else {
        let tail = &data[data.len().saturating_sub(131072)..];
        return if tail.windows(MARKER.len()).any(|w| w == MARKER) {
            Err(Error::UnsupportedLayout(
                "unrecognized CameraAlbum terminal tail layout",
            ))
        } else {
            Ok(None)
        };
    };
    let length2 = u32::from_be_bytes(
        data[marker + 11..marker + 15]
            .try_into()
            .map_err(|_| Error::InvalidResource)?,
    );
    let start = marker + 15;
    if data.get(data.len() - 15..data.len() - 11) != Some(&[255; 4]) {
        return Err(mismatch("CameraAlbum terminal separator"));
    }
    match layout {
        CameraAlbumLayout::LengthDelimited43 if length2 != 47 => {
            return Err(mismatch("CameraAlbum tail-length declaration"))
        }
        CameraAlbumLayout::Observed55ByteTail
            if length2 != 2
                || data.get(start..start + 11) != Some(&[0; 11])
                || data.get(start + 11) != Some(&b'/') =>
        {
            return Err(Error::UnsupportedLayout(
                "unrecognized CameraAlbum55-byte tail fields",
            ));
        }
        _ => {}
    }
    let id_start = start + tail_length - 43;
    let identifier_bytes = &data[id_start..id_start + 28];
    if !identifier_bytes.iter().all(u8::is_ascii_graphic) {
        return Err(mismatch("CameraAlbum identifier encoding"));
    }
    let envelope_identifier =
        std::str::from_utf8(identifier_bytes).map_err(|_| Error::InvalidResource)?;
    let length_at = marker.checked_sub(4).ok_or(Error::InvalidResource)?;
    let length = u32::from_be_bytes(
        data[length_at..marker]
            .try_into()
            .map_err(|_| Error::InvalidResource)?,
    ) as usize;
    if length > 65536 {
        return Err(Error::InputLimit);
    }
    let json_start = length_at
        .checked_sub(length)
        .ok_or(Error::InvalidResource)?;
    let raw_json = &data[json_start..length_at];
    let fields = json_fields(raw_json)?;
    let (prefix, footer_start) =
        if json_start >= 20 && &data[json_start - 20..json_start] == b"vivoMediaExtInfovivo" {
            (CameraAlbumPrefix::VivoMediaExtInfo, json_start - 20)
        } else if json_start >= 4 && &data[json_start - 4..json_start] == b"vivo" {
            (CameraAlbumPrefix::Vivo, json_start - 4)
        } else {
            (CameraAlbumPrefix::Unspecified, json_start)
        };
    if data.starts_with(&[255, 216])
        && liblivephoto_format::jpeg_image_end(data, 0).map_err(|_| Error::InvalidStill)?
            > footer_start
    {
        return Err(mismatch("CameraAlbum footer overlaps encoded JPEG"));
    }
    let declared_identifier = fields
        .get("com.android.camera.livephoto")
        .map(|value| {
            value
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 4096)
                .map(str::to_owned)
                .ok_or_else(|| mismatch("CameraAlbum live-photo identifier"))
        })
        .transpose()?;
    let identifier_agreement = match declared_identifier.as_deref() {
        None => DeclaredIdentifierAgreement::Absent,
        Some(id) if id == envelope_identifier => DeclaredIdentifierAgreement::Equal,
        Some(_) => DeclaredIdentifierAgreement::Different,
    };
    Ok(Some(CameraAlbumFooter {
        extent: ByteRange::new(footer_start as u64, data.len() as u64)?,
        json_extent: ByteRange::new(json_start as u64, length_at as u64)?,
        identifier_extent: ByteRange::new(id_start as u64, (id_start + 28) as u64)?,
        layout,
        prefix,
        length2,
        envelope_identifier,
        declared_identifier,
        identifier_agreement,
        version: integer(&fields, "version")?,
        image_time: integer(&fields, "com.android.camera.imageTime")?,
        raw_json,
    }))
}
