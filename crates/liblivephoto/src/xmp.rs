//! Standalone raw XMP with a typed Motion Photo projection. Rewriting known
//! properties uses the existing packet splicer and retains unrelated XML.
use crate::{MediaTime, MotionPhotoReadError as Error};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmpDirectoryItem {
    pub mime: String,
    pub semantic: String,
    pub length: u64,
    pub padding: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XmpFields {
    pub camera_values: BTreeMap<String, i64>,
    pub directory: Vec<XmpDirectoryItem>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmpDocument {
    raw: Vec<u8>,
    fields: XmpFields,
}
impl XmpDocument {
    pub fn parse(raw: &[u8]) -> Result<Self, Error> {
        if raw.len() > 4 * 1024 * 1024 {
            return Err(Error::InputLimit);
        }
        let mut reader = quick_xml::reader::NsReader::from_reader(raw);
        let mut roots = 0usize;
        let mut depth = 0usize;
        loop {
            match reader
                .read_event()
                .map_err(|_| Error::MetadataMismatch("XMP document XML"))?
            {
                quick_xml::events::Event::Start(_) => {
                    if depth == 0 {
                        roots += 1;
                    }
                    depth += 1;
                    if depth > 128 {
                        return Err(Error::InputLimit);
                    }
                }
                quick_xml::events::Event::Empty(_) => {
                    if depth == 0 {
                        roots += 1;
                    }
                }
                quick_xml::events::Event::End(_) => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or(Error::MetadataMismatch("XMP document depth"))?;
                }
                quick_xml::events::Event::Text(t) if depth == 0 => {
                    if t.as_ref().bytes().any(|b| !b.is_ascii_whitespace()) {
                        return Err(Error::MetadataMismatch("text outside XMP root"));
                    }
                }
                quick_xml::events::Event::Eof => break,
                _ => {}
            }
        }
        if roots != 1 || depth != 0 {
            return Err(Error::MetadataMismatch("XMP document root"));
        }
        let p = crate::android::parse_xmp(raw)?;
        Ok(Self {
            raw: raw.to_vec(),
            fields: XmpFields {
                camera_values: p
                    .camera_values
                    .into_iter()
                    .map(|(k, v)| (k.trim_start_matches("Camera:").to_owned(), v))
                    .collect(),
                directory: p
                    .items
                    .into_iter()
                    .map(|i| XmpDirectoryItem {
                        mime: i.mime,
                        semantic: i.semantic,
                        length: i.length,
                        padding: i.padding,
                    })
                    .collect(),
            },
        })
    }
    pub fn fields(&self) -> &XmpFields {
        &self.fields
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw
    }
    pub fn photo_time(&self) -> Option<MediaTime> {
        self.fields
            .camera_values
            .get("MotionPhotoPresentationTimestampUs")
            .or_else(|| {
                self.fields
                    .camera_values
                    .get("MicroVideoPresentationTimestampUs")
            })
            .copied()
            .filter(|v| *v >= 0)
            .map(|value| MediaTime {
                value,
                timescale: 1_000_000,
            })
    }
    /// Validate a Motion Photo declaration separately from parsing an ordinary XMP
    /// document. Resource extents cannot be verified without their source asset.
    pub fn validate_motion(&self) -> Result<(), Error> {
        if self.fields.camera_values.get("MotionPhoto") == Some(&1) {
            if self.fields.camera_values.get("MotionPhotoVersion") != Some(&1) {
                return Err(Error::MetadataMismatch("MotionPhotoVersion"));
            }
            crate::android::validate_directory(
                &self
                    .fields
                    .directory
                    .iter()
                    .map(|i| crate::model::MotionPhotoItem {
                        mime: i.mime.clone(),
                        semantic: i.semantic.clone(),
                        length: i.length,
                        padding: i.padding,
                    })
                    .collect::<Vec<_>>(),
            )?;
        } else if self.fields.camera_values.get("MicroVideo") != Some(&1) {
            return Err(Error::Unrecognized);
        }
        Ok(())
    }
    pub fn rewrite(&self, fields: &XmpFields) -> Result<Self, Error> {
        let mut description=String::from("<rdf:Description xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:Camera=\"http://ns.google.com/photos/1.0/camera/\" xmlns:Container=\"http://ns.google.com/photos/1.0/container/\" xmlns:Item=\"http://ns.google.com/photos/1.0/container/item/\"");
        for (key, value) in &fields.camera_values {
            if !matches!(
                key.as_str(),
                "MotionPhoto"
                    | "MotionPhotoVersion"
                    | "MotionPhotoPresentationTimestampUs"
                    | "MicroVideo"
                    | "MicroVideoVersion"
                    | "MicroVideoOffset"
                    | "MicroVideoPresentationTimestampUs"
            ) {
                return Err(Error::MetadataMismatch("unknown writable camera property"));
            }
            description.push_str(&format!(" Camera:{key}=\"{value}\""));
        }
        description.push('>');
        if !fields.directory.is_empty() {
            description.push_str("<Container:Directory><rdf:Seq>");
            for (index, item) in fields.directory.iter().enumerate() {
                if index != 0 && item.padding != 0 {
                    return Err(Error::MetadataMismatch(
                        "padding on a secondary directory item",
                    ));
                }
                // Motion Photo 1.0 permits the Padding attribute only on the
                // primary item, including when a secondary value would be zero.
                let padding = if index == 0 {
                    format!(" Item:Padding=\"{}\"", item.padding)
                } else {
                    String::new()
                };
                description.push_str(&format!("<rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"{}\" Item:Semantic=\"{}\" Item:Length=\"{}\"{padding}/></rdf:li>",quick_xml::escape::escape(&item.mime),quick_xml::escape::escape(&item.semantic),item.length));
            }
            description.push_str("</rdf:Seq></Container:Directory>");
        }
        description.push_str("</rdf:Description>");
        let raw = crate::jpeg_write::rewrite_packet(&self.raw, &description)?;
        Self::parse(&raw)
    }
    pub fn motion(
        still_mime: &str,
        movie_mime: &str,
        movie_length: usize,
        time: Option<MediaTime>,
    ) -> Result<Self, Error> {
        let description = crate::jpeg_write::android_description(
            movie_length,
            still_mime,
            movie_mime,
            time,
            &[],
        )?;
        Self::parse(format!("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">{description}</rdf:RDF></x:xmpmeta>").as_bytes())
    }
}
