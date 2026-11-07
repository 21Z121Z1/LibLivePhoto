//! Apple Live Photo `.pvt` directory contract.
//!
//! A package is a validated still/MOV pair plus one XML property list. Packaging
//! does not transcode, remux, or choose a new content identifier. Callers publish
//! the directory. This module does not claim Photos import.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::{Input, MediaTime, MotionPhoto, ParseOptions};

/// Observed complement plist. `PFVideoComplementMetadataVersionKey` is the string `"1"`.
pub const METADATA_PLIST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>PFVideoComplementMetadataVersionKey</key><string>1</string></dict></plist>
"#;

const MAX_METADATA_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub struct Error {
    detail: String,
}

impl Error {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub still_name: String,
    pub movie_name: String,
    pub metadata_name: &'static str,
    pub still: Vec<u8>,
    pub movie: Vec<u8>,
    pub metadata: &'static [u8],
    pub content_identifier: String,
    pub presentation: MediaTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub still_name: String,
    pub movie_name: String,
    pub still: Vec<u8>,
    pub movie: Vec<u8>,
}

pub fn metadata_plist() -> &'static [u8] {
    METADATA_PLIST
}

fn invalid_metadata() -> Error {
    Error::new("PVT metadata.plist must be the XML version-1 video complement document")
}

fn invalid_directory() -> Error {
    Error::new(
        "unsupported or ambiguous PVT directory; expected one regular still, one regular MOV, and metadata.plist",
    )
}

/// Accept only the observed complement document. This is not a general property-list
/// parser, and it does not fetch external entities.
pub fn validate_metadata(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err(invalid_metadata());
    }
    let expected: [&[u8]; 10] = [
        b"plist",
        b"dict",
        b"key",
        b"PFVideoComplementMetadataVersionKey",
        b"/key",
        b"string",
        b"1",
        b"/string",
        b"/dict",
        b"/plist",
    ];
    let mut position = 0usize;
    let mut declaration = false;
    let mut doctype = false;
    let mut reader = Reader::from_reader(bytes);
    loop {
        let token = match reader
            .read_event()
            .map_err(|error| Error::new(format!("PVT metadata XML: {error}")))?
        {
            Event::Start(value) => {
                let name = value.name();
                let attributes = value
                    .attributes()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| Error::new(format!("PVT metadata XML: {error}")))?;
                if name.as_ref() == "plist" {
                    if attributes.len() != 1
                        || attributes[0].key.as_ref() != "version"
                        || attributes[0].value.as_ref() != "1.0"
                    {
                        return Err(invalid_metadata());
                    }
                } else if !attributes.is_empty() {
                    return Err(invalid_metadata());
                }
                if !matches!(position, 0 | 1 | 2 | 5) {
                    return Err(invalid_metadata());
                }
                name.as_ref().as_bytes().to_vec()
            }
            Event::End(value) => {
                if !matches!(position, 4 | 7 | 8 | 9) {
                    return Err(invalid_metadata());
                }
                [b"/".as_slice(), value.name().as_ref().as_bytes()].concat()
            }
            Event::Text(value) => {
                if value
                    .as_ref()
                    .bytes()
                    .all(|byte| byte.is_ascii_whitespace())
                {
                    continue;
                }
                if !matches!(position, 3 | 6) {
                    return Err(invalid_metadata());
                }
                value.as_ref().as_bytes().to_vec()
            }
            Event::Comment(_) => continue,
            Event::Decl(_) if position == 0 && !declaration && !doctype => {
                declaration = true;
                continue;
            }
            Event::DocType(_) if position == 0 && !doctype => {
                doctype = true;
                continue;
            }
            Event::Eof => break,
            _ => return Err(invalid_metadata()),
        };
        if expected.get(position).copied() != Some(token.as_slice()) {
            return Err(invalid_metadata());
        }
        position += 1;
    }
    if position != expected.len() {
        return Err(invalid_metadata());
    }
    Ok(())
}

pub fn validate_stem(stem: &str) -> Result<(), Error> {
    if stem.is_empty()
        || stem == "."
        || stem == ".."
        || stem.contains(['/', '\\', '\0'])
        || stem.chars().any(char::is_control)
    {
        return Err(Error::new("PVT package name must be a single file stem"));
    }
    Ok(())
}

fn still_extension(mime: &str, bytes: &[u8]) -> Result<&'static str, Error> {
    let (extension, matches) = match mime {
        "image/jpeg" => ("jpg", bytes.starts_with(&[0xff, 0xd8, 0xff])),
        "image/heic" => ("heic", bytes.get(4..8) == Some(b"ftyp")),
        "image/heif" => ("heif", bytes.get(4..8) == Some(b"ftyp")),
        _ => {
            return Err(Error::new(
                "PVT writer accepts a JPEG or HEIC/HEIF still; this library does not transcode",
            ))
        }
    };
    if !matches {
        return Err(Error::new(
            "PVT still bytes do not match the declared image type",
        ));
    }
    Ok(extension)
}

/// Assemble package members from an already validated Apple pair.
///
/// The still and movie bytes are stored unchanged. Identifier and still-image-time
/// checks use the existing Apple pair parser.
pub fn package_pair(
    stem: &str,
    still_mime: &str,
    still: Vec<u8>,
    movie: Vec<u8>,
) -> Result<Package, Error> {
    validate_stem(stem)?;
    let extension = still_extension(still_mime, &still)?;
    let photo = MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .map_err(|error| Error::new(format!("PVT requires a validated Apple pair: {error}")))?;
    let content_identifier = photo
        .asset()
        .pairing
        .identifier
        .clone()
        .ok_or_else(|| Error::new("PVT requires an Apple content identifier"))?;
    let presentation = photo
        .presentation_time()
        .ok_or_else(|| Error::new("PVT requires still-image-time metadata"))?;
    Ok(Package {
        still_name: format!("{stem}.{extension}"),
        movie_name: format!("{stem}.mov"),
        metadata_name: "metadata.plist",
        still,
        movie,
        metadata: METADATA_PLIST,
        content_identifier,
        presentation,
    })
}

fn read_regular(path: &Path, remaining: &mut u64) -> Result<Vec<u8>, Error> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::new(format!("PVT member read: {error}")))?;
    if !metadata.file_type().is_file() {
        return Err(invalid_directory());
    }
    let len = metadata.len();
    if len > *remaining {
        return Err(Error::new("PVT member exceeds the input budget"));
    }
    *remaining -= len;
    let file = File::open(path).map_err(|error| Error::new(format!("PVT member read: {error}")))?;
    let mut bytes = Vec::new();
    file.take(len.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| Error::new(format!("PVT member read: {error}")))?;
    if bytes.len() as u64 != len {
        return Err(Error::new("PVT member changed during read"));
    }
    Ok(bytes)
}

/// Read one package directory. Member names do not establish pairing.
pub fn read_directory(path: &Path, max_member_bytes: u64) -> Result<Directory, Error> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::new(format!("PVT directory read: {error}")))?;
    if !metadata.is_dir() {
        return Err(invalid_directory());
    }
    let mut image: Option<(String, PathBuf)> = None;
    let mut video: Option<(String, PathBuf)> = None;
    let mut plist = None;
    let mut count = 0usize;
    let entries =
        fs::read_dir(path).map_err(|error| Error::new(format!("PVT directory read: {error}")))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::new(format!("PVT directory read: {error}")))?;
        count += 1;
        if count > 3 {
            return Err(invalid_directory());
        }
        let file_type = entry
            .file_type()
            .map_err(|error| Error::new(format!("PVT member type: {error}")))?;
        if !file_type.is_file() {
            return Err(invalid_directory());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid_directory())?;
        let member = entry.path();
        if name == "metadata.plist" {
            if plist.replace(member).is_some() {
                return Err(invalid_directory());
            }
        } else {
            let extension = Path::new(&name)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if ["heic", "heif", "jpg", "jpeg"]
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            {
                if image.replace((name, member)).is_some() {
                    return Err(invalid_directory());
                }
            } else if extension.eq_ignore_ascii_case("mov") {
                if video.replace((name, member)).is_some() {
                    return Err(invalid_directory());
                }
            } else {
                return Err(invalid_directory());
            }
        }
    }
    let (Some((still_name, still_path)), Some((movie_name, movie_path)), Some(plist)) =
        (image, video, plist)
    else {
        return Err(invalid_directory());
    };
    let mut remaining = max_member_bytes;
    let plist_bytes = read_regular(&plist, &mut remaining)?;
    if plist_bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err(invalid_directory());
    }
    validate_metadata(&plist_bytes)?;
    let still = read_regular(&still_path, &mut remaining)?;
    let movie = read_regular(&movie_path, &mut remaining)?;
    Ok(Directory {
        still_name,
        movie_name,
        still,
        movie,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_requires_the_observed_string_version() {
        validate_metadata(METADATA_PLIST).unwrap();
        let text = std::str::from_utf8(METADATA_PLIST).unwrap();
        for malformed in [
            text.replace("<string>1</string>", "<integer>1</integer>"),
            text.replace("<string>1</string>", "<true/>"),
            text.replace("<string>1</string>", "<string>2</string>"),
            text.replace("PFVideoComplementMetadataVersionKey", "UnknownKey"),
            text.replace(
                "</dict>",
                "<key>PFVideoComplementMetadataVersionKey</key><string>1</string></dict>",
            ),
            format!("{text}{text}"),
            format!("<?xml version=\"1.0\"?>{text}"),
            text.replace("</plist>", ""),
        ] {
            assert!(
                validate_metadata(malformed.as_bytes()).is_err(),
                "{malformed}"
            );
        }
    }

    #[test]
    fn stem_rejects_path_components() {
        assert!(validate_stem("IMG_0010").is_ok());
        for stem in ["", ".", "..", "a/b", "a\\b", "a\0b", "bad\n"] {
            assert!(validate_stem(stem).is_err(), "{stem:?}");
        }
    }
}
