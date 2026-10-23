#![forbid(unsafe_code)]
//! One operation implementation shared by maintained platform wrappers. All
//! original inputs are immutable. Publication belongs to the caller/CLI.
use liblivephoto::*;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::ops::ControlFlow;
pub const MAX_INPUT: usize = 128 * 1024 * 1024;
const MAX_RESPONSE: usize = 320 * 1024 * 1024;
const MAX_RESPONSE_METADATA: usize = 64 * 1024 * 1024;
const MAX_RESPONSE_FILES: usize = 4096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationFile {
    pub name: String,
    pub mime: String,
    pub resource_id: Option<usize>,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct OperationResult {
    pub success: bool,
    pub metadata: Value,
    pub files: Vec<OperationFile>,
}
impl OperationResult {
    pub fn failure(code: &str, message: impl std::fmt::Display) -> Self {
        Self {
            success: false,
            metadata: json!({"success":false,"error":{"code":code,"message":message.to_string()}}),
            files: Vec::new(),
        }
    }
    /// LLP1: magic, u32 BE JSON length + UTF-8 JSON, u32 file count; each file has
    /// u32 MIME length/string, u32 name length/string, i64 resource ID (-1=none),
    /// u32 data length + raw data. No base64 media copies or lossy numeric time.
    pub fn into_envelope(self) -> std::result::Result<Vec<u8>, String> {
        let metadata = envelope_metadata(self.success, &self.metadata)?;
        let total = checked_response_len(
            metadata.len(),
            self.files.iter().map(|file| ResponseFile {
                name: &file.name,
                mime: &file.mime,
                resource_id: file.resource_id,
                byte_len: file.bytes.len(),
            }),
        )?;
        let mut output = Vec::with_capacity(total);
        output.extend(b"LLP1");
        put(&mut output, &metadata)?;
        output.extend((self.files.len() as u32).to_be_bytes());
        for file in self.files {
            put(&mut output, file.mime.as_bytes())?;
            put(&mut output, file.name.as_bytes())?;
            output.extend(
                file.resource_id
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| "resource ID exceeds protocol range")?
                    .unwrap_or(-1)
                    .to_be_bytes(),
            );
            put(&mut output, &file.bytes)?;
        }
        Ok(output)
    }
}
fn envelope_metadata(success: bool, metadata: &Value) -> std::result::Result<Vec<u8>, String> {
    serde_json::to_vec(&json!({"success":success,"metadata":metadata})).map_err(|e| e.to_string())
}
struct ResponseFile<'a> {
    name: &'a str,
    mime: &'a str,
    resource_id: Option<usize>,
    byte_len: usize,
}
fn checked_response_len<'a>(
    metadata_len: usize,
    files: impl ExactSizeIterator<Item = ResponseFile<'a>>,
) -> std::result::Result<usize, String> {
    if metadata_len > MAX_RESPONSE_METADATA || files.len() > MAX_RESPONSE_FILES {
        return Err("response budget exceeded".into());
    }
    let mut total = metadata_len
        .checked_add(12)
        .ok_or("response budget exceeded")?;
    for file in files {
        for size in [file.name.len(), file.mime.len(), file.byte_len] {
            u32::try_from(size).map_err(|_| "response field too large")?;
            total = total.checked_add(size).ok_or("response budget exceeded")?;
        }
        file.resource_id
            .map(i64::try_from)
            .transpose()
            .map_err(|_| "resource ID exceeds protocol range")?;
        total = total.checked_add(20).ok_or("response budget exceeded")?;
    }
    (total <= MAX_RESPONSE)
        .then_some(total)
        .ok_or_else(|| "response budget exceeded".into())
}
fn put(out: &mut Vec<u8>, value: &[u8]) -> std::result::Result<(), String> {
    let n = u32::try_from(value.len()).map_err(|_| "response field too large")?;
    out.extend(n.to_be_bytes());
    out.extend(value);
    Ok(())
}
fn ok(metadata: Value, files: Vec<OperationFile>) -> OperationResult {
    OperationResult {
        success: true,
        metadata,
        files,
    }
}
fn conformance_result(
    result: std::result::Result<FormatConformanceReport, MotionPhotoReadError>,
) -> OperationResult {
    match result {
        Ok(report) => OperationResult {
            success: report.checked_rules_satisfied(),
            metadata: json!({"profile":report.profile,"checkedRequiredRulesSatisfied":report.checked_rules_satisfied(),
                "checkedRules":report.checked_rules,"issues":report.issues.iter().map(|i|json!({"rule":i.rule,"kind":format!("{:?}",i.kind),"itemId":i.item_id,"trackId":i.track_id,"message":i.message})).collect::<Vec<_>>(),
                "scope":"listed declarations and supported timelines only; no codec decoding, full format certification or native Gallery/Photos acceptance"}),
            files: Vec::new(),
        },
        Err(error) => OperationResult::failure(code(&error), error),
    }
}
fn code(error: &MotionPhotoReadError) -> &'static str {
    match error {
        MotionPhotoReadError::InputLimit => "inputLimit",
        MotionPhotoReadError::IdentifierMismatch => "identifierMismatch",
        MotionPhotoReadError::MissingPairMetadata => "missingPairMetadata",
        MotionPhotoReadError::InvalidPresentation => "invalidTime",
        MotionPhotoReadError::UnsupportedLayout(_) => "unsupportedLayout",
        MotionPhotoReadError::MetadataMismatch(_) => "metadataMismatch",
        MotionPhotoReadError::ConversionBlocked(_) => "conversionBlocked",
        MotionPhotoReadError::Io(_) => "ioError",
        _ => "invalidInput",
    }
}
fn time(v: &Value, key: &str) -> std::result::Result<Option<MediaTime>, String> {
    let t = &v[key];
    if t.is_null() {
        return Ok(None);
    }
    let value = t["value"]
        .as_str()
        .ok_or("time.value must be a decimal string")?
        .parse::<i64>()
        .map_err(|_| "invalid i64 time")?;
    let scale = t["timescale"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("invalid timescale")?;
    MediaTime::new(value, scale)
        .map(Some)
        .ok_or("zero timescale".into())
}
fn handlers(v: &Value) -> std::result::Result<HandlerOptions, String> {
    let mut options = HandlerOptions {
        disable_third_party: v["disableThirdParty"].as_bool().unwrap_or(false),
        ..HandlerOptions::default()
    };
    if let Some(a) = v.get("enabledProviders") {
        for name in a
            .as_array()
            .filter(|v| v.len() <= 128)
            .ok_or("invalid enabledProviders")?
        {
            options.enabled_third_party.push(
                name.as_str()
                    .filter(|s| s.len() <= 128)
                    .ok_or("invalid provider name")?
                    .into(),
            );
        }
    }
    Ok(options)
}
fn ids(v: &Value) -> std::result::Result<Vec<u32>, String> {
    v["trackIds"]
        .as_array()
        .filter(|v| v.len() <= 4096)
        .ok_or("trackIds array required")?
        .iter()
        .map(|n| {
            n.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n > 0)
                .ok_or("invalid track ID".into())
        })
        .collect()
}
fn ext(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/heic" => "heic",
        "image/avif" => "avif",
        "video/quicktime" => "mov",
        "video/mp4" => "mp4",
        "application/rdf+xml" => "xmp",
        _ => "bin",
    }
}
enum ExtractFile<'p> {
    Resource {
        resource: &'p Resource,
        name: String,
        byte_len: usize,
    },
    Xmp(Cow<'p, [u8]>),
}
impl<'p> ExtractFile<'p> {
    fn resource(
        photo: &MotionPhoto<'_>,
        resource: &'p Resource,
    ) -> std::result::Result<Self, String> {
        let byte_len = usize::try_from(
            photo
                .extract(resource, std::io::sink())
                .map_err(|e| e.to_string())?,
        )
        .map_err(|_| "response field too large")?;
        Ok(Self::Resource {
            resource,
            name: format!("resource-{}.{}", resource.id.0, ext(&resource.mime)),
            byte_len,
        })
    }
    fn fields(&self) -> ResponseFile<'_> {
        match self {
            Self::Resource {
                resource,
                name,
                byte_len,
            } => ResponseFile {
                name,
                mime: &resource.mime,
                resource_id: Some(resource.id.0),
                byte_len: *byte_len,
            },
            Self::Xmp(bytes) => ResponseFile {
                name: "metadata.xmp",
                mime: "application/rdf+xml",
                resource_id: None,
                byte_len: bytes.len(),
            },
        }
    }
}
struct CheckedExtraction<'p, 'i> {
    photo: &'p MotionPhoto<'i>,
    files: Vec<ExtractFile<'p>>,
    metadata: Value,
}
impl<'p, 'i> CheckedExtraction<'p, 'i> {
    fn from_request(
        photo: &'p MotionPhoto<'i>,
        request: &Value,
    ) -> std::result::Result<Self, String> {
        let mut files = Vec::new();
        let mut unavailable = Vec::new();
        if let Some(selected) = request.get("resourceIds") {
            let selected = selected
                .as_array()
                .filter(|a| a.len() <= MAX_RESPONSE_FILES)
                .ok_or("invalid resourceIds")?;
            let mut seen = std::collections::BTreeSet::new();
            for id in selected {
                let id = id
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|n| seen.insert(*n))
                    .ok_or("invalid/duplicate resource ID")?;
                let resource = photo
                    .resources()
                    .find(|r| r.id.0 == id)
                    .ok_or("resource ID missing")?;
                files.push(ExtractFile::resource(photo, resource)?);
            }
        } else {
            if request["still"].as_bool().unwrap_or(true) {
                files.push(ExtractFile::resource(photo, photo.still())?);
            }
            if request["video"].as_bool().unwrap_or(true) {
                files.push(ExtractFile::resource(photo, photo.motion_video())?);
            }
            if request["xmp"].as_bool().unwrap_or(true) {
                if let Some(bytes) = photo.xmp_bytes().map_err(|e| e.to_string())? {
                    files.push(ExtractFile::Xmp(bytes));
                } else {
                    unavailable.push("xmp");
                }
            }
        }
        let metadata = json!({"files":files.iter().map(|file| {
            let file = file.fields();
            json!({"name":file.name,"mime":file.mime,"resourceId":file.resource_id,"bytes":file.byte_len.to_string()})
        }).collect::<Vec<_>>(),"unavailable":unavailable});
        checked_response_len(
            envelope_metadata(true, &metadata)?.len(),
            files.iter().map(ExtractFile::fields),
        )?;
        Ok(Self {
            photo,
            files,
            metadata,
        })
    }
    fn materialize(self) -> std::result::Result<OperationResult, String> {
        let mut files = Vec::with_capacity(self.files.len());
        for file in self.files {
            files.push(match file {
                ExtractFile::Resource {
                    resource,
                    name,
                    byte_len,
                } => {
                    let mut bytes = Vec::with_capacity(byte_len);
                    self.photo
                        .extract(resource, &mut bytes)
                        .map_err(|e| e.to_string())?;
                    OperationFile {
                        name,
                        mime: resource.mime.clone(),
                        resource_id: Some(resource.id.0),
                        bytes,
                    }
                }
                ExtractFile::Xmp(bytes) => OperationFile {
                    name: "metadata.xmp".into(),
                    mime: "application/rdf+xml".into(),
                    resource_id: None,
                    bytes: bytes.into_owned(),
                },
            });
        }
        Ok(ok(self.metadata, files))
    }
}
/// Request JSON is capped independently of immutable source bytes. Invalid input
/// is an explicit response; no path, process, network or publication occurs here.
pub fn process(request: &str, primary: &[u8], movie: &[u8]) -> OperationResult {
    if request.len() > 1024 * 1024
        || primary
            .len()
            .checked_add(movie.len())
            .is_none_or(|n| n > MAX_INPUT)
    {
        return OperationResult::failure("inputLimit", "operation input budget exceeded");
    }
    let request: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return OperationResult::failure("invalidRequest", e),
    };
    match execute(&request, primary, movie) {
        Ok(v) => v,
        Err(e) => OperationResult::failure("invalidRequest", e),
    }
}

/// The same bounded operation protocol as C/JNI/CLI. The caller owns the LLP1
/// bytes and chooses publication; no browser-specific format implementation.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn process_operation(
    request: &str,
    primary: &[u8],
    movie: &[u8],
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
    process(request, primary, movie)
        .into_envelope()
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}
fn execute(
    v: &Value,
    primary: &[u8],
    movie: &[u8],
) -> std::result::Result<OperationResult, String> {
    for key in [
        "action",
        "inputKind",
        "target",
        "unmapped",
        "pairId",
        "profile",
        "samsungRecordIndex",
    ] {
        if v.get(key).is_some_and(|value| !value.is_string()) {
            return Err(format!("{key} must be a string"));
        }
    }
    for key in [
        "strict",
        "disableThirdParty",
        "compact",
        "dryRun",
        "still",
        "video",
        "xmp",
        "videoInput",
    ] {
        if v.get(key).is_some_and(|value| !value.is_boolean()) {
            return Err(format!("{key} must be a boolean"));
        }
    }
    let action = v["action"].as_str().ok_or("action required")?;
    if let Some(value) = v.get("samsungRecordIndex") {
        if action != "extract"
            || !movie.is_empty()
            || v.get("inputKind")
                .is_some_and(|k| k.as_str() != Some("single"))
            || v.get("resourceIds").is_some()
            || v["still"].as_bool() == Some(true)
            || v["xmp"].as_bool() == Some(true)
            || v["video"].as_bool() == Some(false)
            || v.get("time").is_some()
            || v["strict"].as_bool() == Some(true)
        {
            return Err("samsungRecordIndex is an exclusive single-input original-video extraction selector".into());
        }
        let index = value
            .as_str()
            .ok_or("samsungRecordIndex must be a decimal string")?;
        if index.is_empty() || index.len() > 20 || !index.bytes().all(|b| b.is_ascii_digit()) {
            return Err("samsungRecordIndex must be a nonnegative decimal string".into());
        }
        let index = index
            .parse::<usize>()
            .map_err(|_| "samsungRecordIndex overflows the supported index")?;
        let options = handlers(v)?;
        if !options.allows("samsung_sef") && !options.allows("samsung_video_inventory") {
            return Err("Samsung record inspection is disabled".into());
        }
        let selected = (|| -> std::result::Result<_, MotionPhotoReadError> {
            let inventory =
                inspect_samsung_videos(primary)?.ok_or(MotionPhotoReadError::Unrecognized)?;
            let video = inventory.select(Some(index))?;
            let record = &inventory.records[index];
            let mime = movie_container_mime(video.bytes)?;
            Ok(ok(
                json!({"samsungRecordIndex":index,"wireName":record.name,"wireKind":record.kind,
                "extent":{"start":video.extent.lower_bound.to_string(),"end":video.extent.upper_bound.to_string()},
                "declaredPrimaryRecords":inventory.declared_primary_records,"role":"uninterpreted",
                "encodedBytesUnchanged":true,"selection":"explicit-record-index"}),
                vec![OperationFile {
                    name: format!("samsung-record-{index}.{}", ext(mime)),
                    mime: mime.into(),
                    resource_id: None,
                    bytes: video.bytes.to_vec(),
                }],
            ))
        })();
        return Ok(match selected {
            Ok(result) => result,
            Err(error) => OperationResult::failure(code(&error), error),
        });
    }
    if !movie.is_empty()
        && matches!(
            action,
            "version"
                | "metadata"
                | "extract-metadata"
                | "classify-metadata"
                | "xmp"
                | "track-summary"
                | "optimize"
                | "remove-tracks"
                | "conformance"
        )
    {
        return Err("this operation accepts one input".into());
    }
    let options = handlers(v)?;
    let requested_time = time(v, "time")?;
    if action == "version" {
        return Ok(ok(
            json!({"version":version(),"protocol":"LLP1"}),
            Vec::new(),
        ));
    }
    if action == "conformance" {
        return Ok(
            match v["profile"]
                .as_str()
                .ok_or("conformance profile required")?
            {
                "heif-color-2025" => conformance_result(validate_heif_color_information(primary)),
                "icc-structure-v4.4" => conformance_result(validate_icc_structure(primary)),
                "google-video-container-1.0" => {
                    let tracks = ids(v)?;
                    if !(1..=2).contains(&tracks.len())
                        || tracks.len() == 2 && tracks[0] == tracks[1]
                    {
                        return Err("conformance requires one primary track ID and at most one distinct secondary ID".into());
                    }
                    conformance_result(validate_google_video_container(
                        primary,
                        GoogleVideoTracks {
                            primary: tracks[0],
                            secondary: tracks.get(1).copied(),
                        },
                    ))
                }
                _ => return Err("unknown conformance profile".into()),
            },
        );
    }
    if action == "classify-metadata" {
        return Ok(match MetadataCollection::from_json(primary) {
            Ok(collection) => ok(
                json!({"declaresMotion":collection.declares_motion(&options),"verifiedMotion":false,"diagnostics":collection.diagnostics}),
                Vec::new(),
            ),
            Err(e) => OperationResult::failure(code(&e), e),
        });
    }
    if action == "metadata" {
        return Ok(match MetadataEngine::new().parse(primary, &options) {
            Ok(c) => ok(c.json(), Vec::new()),
            Err(e) => OperationResult::failure(code(&e), e),
        });
    }
    if action == "extract-metadata" {
        let collection = match MetadataEngine::new().parse(primary, &options) {
            Ok(c) => c,
            Err(e) => return Ok(OperationResult::failure(code(&e), e)),
        };
        let selected = match v.get("blockIds") {
            None => (0..collection.blocks.len()).collect::<Vec<_>>(),
            Some(ids) => {
                let mut seen = std::collections::BTreeSet::new();
                ids.as_array()
                    .filter(|a| a.len() <= 4096)
                    .ok_or("invalid blockIds")?
                    .iter()
                    .map(|id| {
                        id.as_u64()
                            .and_then(|n| usize::try_from(n).ok())
                            .filter(|n| *n < collection.blocks.len() && seen.insert(*n))
                            .ok_or("missing/duplicate metadata block ID")
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?
            }
        };
        let mut total = 0u64;
        let mut files = Vec::new();
        for &index in &selected {
            let block = &collection.blocks[index];
            let mut bytes = Vec::new();
            match collection.extract_block(index, primary, &mut bytes, MAX_INPUT as u64 - total) {
                Ok(size) => total += size,
                Err(e) => return Ok(OperationResult::failure(code(&e), e)),
            }
            let mime = match block.kind.as_str() {
                "XMP" => "application/rdf+xml",
                "EXIF" => "application/x-exif",
                "ICC" => "application/vnd.iccprofile",
                _ => "application/octet-stream",
            };
            files.push(OperationFile {
                name: format!("metadata-block-{index}.{}", ext(mime)),
                mime: mime.into(),
                resource_id: None,
                bytes,
            });
        }
        return Ok(ok(
            json!({"collection":collection.json(),"selectedBlockIds":selected,"totalBytes":total.to_string(),"rawBlocksMayContainCodedItemsOrVendorVideo":true}),
            files,
        ));
    }
    if action == "xmp" {
        return Ok(match XmpDocument::parse(primary) {
            Ok(doc) => {
                let fields = doc.fields();
                ok(
                    json!({"camera":fields.camera_values.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>(),"directory":fields.directory.iter().map(|i|json!({"mime":i.mime,"semantic":i.semantic,"length":i.length.to_string(),"padding":i.padding.to_string()})).collect::<Vec<_>>(),"motionValidation":doc.validate_motion().err().map(|e|e.to_string())}),
                    Vec::new(),
                )
            }
            Err(e) => OperationResult::failure(code(&e), e),
        });
    }
    if matches!(
        action,
        "optimize" | "remove-tracks" | "import-tracks" | "track-summary"
    ) {
        if action == "track-summary" {
            return Ok(match remux::track_summary(primary) {
                Ok(c) => ok(
                    json!({"movieTimescale":c.movie_timescale,"tracks":c.tracks.iter().map(|t|json!({"id":t.id,"kind":format!("{:?}",t.kind),"timescale":t.timescale,"duration":t.duration.to_string(),"sampleEntries":t.sample_entries.iter().map(|v|String::from_utf8_lossy(v).into_owned()).collect::<Vec<_>>()})).collect::<Vec<_>>()}),
                    Vec::new(),
                ),
                Err(e) => OperationResult::failure(code(&e), e),
            });
        }
        let settings = remux::OptimizeOptions {
            compact_unreferenced_media: v["compact"].as_bool().unwrap_or(false),
        };
        let result = match action {
            "optimize" => remux::optimize(primary, settings),
            "remove-tracks" => remux::remove_tracks(primary, &ids(v)?, settings),
            _ => remux::import_tracks(primary, movie, &ids(v)?, settings),
        };
        return Ok(match result {
            Ok(result) => {
                let mime = movie_container_mime(&result.bytes).map_err(|e| e.to_string())?;
                let mut files = vec![OperationFile {
                    name: format!("movie.{}", ext(mime)),
                    mime: mime.into(),
                    resource_id: None,
                    bytes: result.bytes,
                }];
                for (i, s) in result.sidecars.into_iter().enumerate() {
                    files.push(OperationFile {
                        name: format!("source-{}-metadata-{i}.bin", s.source),
                        mime: "application/octet-stream".into(),
                        resource_id: None,
                        bytes: s.bytes,
                    });
                }
                ok(
                    json!({"trackMappings":result.report.track_mappings.iter().map(|m|json!({"source":m.source,"originalId":m.original_id,"outputId":m.output_id})).collect::<Vec<_>>(),"removedTrackIds":result.report.removed_track_ids,"discardedMediaBytes":result.report.discarded_unreferenced_media_bytes.to_string(),"retainedUnknownMediaBytes":result.report.retained_unreferenced_media_bytes.to_string(),"discardedPaddingBytes":result.report.discarded_padding_bytes.to_string(),"fastStart":result.report.fast_start,"encodedSamplesUnchanged":result.report.encoded_samples_unchanged}),
                    files,
                )
            }
            Err(e) => OperationResult::failure(code(&e), e),
        });
    }
    if action == "agtm" && v["videoInput"].as_bool() == Some(true) {
        let time = requested_time.ok_or("standalone video AGTM requires an exact time")?;
        return Ok(agtm(primary, time));
    }
    let input =
        match v["inputKind"]
            .as_str()
            .unwrap_or(if movie.is_empty() { "single" } else { "apple" })
        {
            "single" if movie.is_empty() => Input::SingleFile(primary),
            "apple" if !movie.is_empty() => Input::ApplePair {
                still: primary,
                movie,
            },
            "parts" if !movie.is_empty() => Input::Parts {
                still: primary,
                movie,
                presentation: requested_time,
            },
            _ => return Err("invalid inputKind / movie association".into()),
        };
    let parse_options = if v["strict"].as_bool() == Some(true) {
        ParseOptions {
            max_input_bytes: MAX_INPUT,
            ..ParseOptions::strict()
        }
    } else {
        ParseOptions {
            max_input_bytes: MAX_INPUT,
            ..ParseOptions::compatible()
        }
    };
    let photo = match MotionPhoto::parse_with_handlers(input, parse_options, &options) {
        Ok(p) => p,
        Err(e) => return Ok(OperationResult::failure(code(&e), e)),
    };
    match action {
        "parse" => {
            let inspection = liblivephoto_bridge::inspect_with_handlers(input, &options, |_| {
                ControlFlow::Continue(())
            })
            .map_err(|e| e.to_string())?;
            Ok(ok(
                serde_json::from_str(&inspection.manifest()).map_err(|e| e.to_string())?,
                Vec::new(),
            ))
        }
        "check" => {
            let report = photo.validate().map_err(|e| e.to_string())?;
            let metadata = photo.scoring_metadata();
            let issues = metadata.as_ref().map(|v| {
                v.iter()
                    .flat_map(|m| {
                        m.issues.iter().map(|i| {
                            format!("track {} sample {}: {i:?}", m.track_id, m.sample_index)
                        })
                    })
                    .collect::<Vec<_>>()
            });
            let valid = issues.as_ref().is_ok_and(|v| v.is_empty());
            Ok(OperationResult {
                success: valid,
                metadata: json!({"valid":valid,"structuralChecks":format!("{:?}",report.checks),"diagnostics":photo.diagnostics().iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),"metadataIssues":issues.as_ref().ok(),"metadataError":metadata.err().map(|e|e.to_string())}),
                files: Vec::new(),
            })
        }
        "extract" => CheckedExtraction::from_request(&photo, v)?.materialize(),
        "agtm" => Ok(agtm(
            photo.motion_video_bytes().map_err(|e| e.to_string())?,
            requested_time
                .or(photo.presentation_time())
                .ok_or("AGTM extraction requires known or explicit photo time")?,
        )),
        "build" => {
            let target = match v["target"]
                .as_str()
                .unwrap_or(match photo.still().mime.as_str() {
                    "image/heic" => "google-heif",
                    "image/avif" => "google-avif",
                    _ => "google-jpeg",
                }) {
                "google-jpeg" => TargetProfile::AndroidJpeg,
                "google-heif" => TargetProfile::AndroidHeif,
                "google-avif" => TargetProfile::AndroidAvif,
                "apple" => TargetProfile::ApplePair,
                "samsung-jpeg-sef" => TargetProfile::SamsungJpegSef,
                "preserve" => TargetProfile::PreserveSource,
                _ => return Err("unknown target".into()),
            };
            let policy = WritePolicy {
                unmapped: match v["unmapped"].as_str().unwrap_or("reject") {
                    "reject" => UnmappedPolicy::Reject,
                    "sidecars" => UnmappedPolicy::PreserveSidecars,
                    "drop" => UnmappedPolicy::DropExplicitly,
                    _ => return Err("unknown unmapped policy".into()),
                },
                pairing: v["pairId"]
                    .as_str()
                    .map_or(PairingPolicy::Preserve, |s| PairingPolicy::Set(s.into())),
            };
            let plan = photo.plan(target, policy);
            let metadata = json!({"target":format!("{target:?}"),"canExecute":plan.can_execute(),"blockers":plan.compatibility().blockers,"consumerAcceptance":format!("{:?}",plan.compatibility().target_consumer),"resourceReport":plan.report().resources.iter().map(|r|json!({"resourceId":r.resource.map(|id|id.0),"disposition":format!("{:?}",r.disposition),"reason":r.reason})).collect::<Vec<_>>(),"pairIdentifier":plan.report().pairing.identifier});
            if !plan.can_execute() {
                return Ok(OperationResult {
                    success: false,
                    metadata,
                    files: Vec::new(),
                });
            }
            if v["dryRun"].as_bool() == Some(true) {
                return Ok(ok(metadata, Vec::new()));
            }
            let output = plan.execute().map_err(|e| e.to_string())?;
            Ok(ok(
                metadata,
                output
                    .files
                    .into_iter()
                    .enumerate()
                    .map(|(index, f)| OperationFile {
                        name: format!(
                            "{}-{index}.{}",
                            f.sidecar_for.map_or_else(
                                || "asset".to_string(),
                                |id| format!("sidecar-{}", id.0)
                            ),
                            ext(&f.mime)
                        ),
                        mime: f.mime,
                        resource_id: f.sidecar_for.map(|id| id.0),
                        bytes: f.bytes,
                    })
                    .collect(),
            ))
        }
        _ => Err("unknown action".into()),
    }
}
fn agtm(video: &[u8], time: MediaTime) -> OperationResult {
    let samples = match extract_agtm_from_video(video, time) {
        Ok(s) => s,
        Err(e) => return OperationResult::failure(code(&e), e),
    };
    let mut files = Vec::new();
    let mut entries = Vec::new();
    for sample in samples {
        let metadata = match sample.metadata() {
            Ok(m) => m,
            Err(e) => return OperationResult::failure(code(&e), e),
        };
        files.push(OperationFile {
            name: format!(
                "agtm-track-{}-sample-{}.bin",
                sample.track_id, sample.sample_index
            ),
            mime: "application/octet-stream".into(),
            resource_id: None,
            bytes: sample.bytes.to_vec(),
        });
        entries.push(json!({"trackId":sample.track_id,"sampleIndex":sample.sample_index,"time":{"value":sample.presentation.value.to_string(),"timescale":sample.presentation.timescale},"metadata":metadata.details_json()}));
    }
    ok(json!({"samples":entries,"renderedPixels":false}), files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_file(byte_len: usize) -> ResponseFile<'static> {
        ResponseFile {
            name: "ab",
            mime: "xyz",
            resource_id: None,
            byte_len,
        }
    }

    #[test]
    fn response_limit_includes_wrapper_fields_and_framing() {
        let payload = MAX_RESPONSE - 3 - 12 - 20 - 2 - 3;
        assert_eq!(
            checked_response_len(3, [response_file(payload)].into_iter()).unwrap(),
            MAX_RESPONSE
        );
        assert_eq!(
            checked_response_len(3, [response_file(payload + 1)].into_iter()).unwrap_err(),
            "response budget exceeded"
        );
        assert!(checked_response_len(MAX_RESPONSE_METADATA, std::iter::empty()).is_ok());
        assert!(checked_response_len(MAX_RESPONSE_METADATA + 1, std::iter::empty()).is_err());
        assert!(checked_response_len(0, (0..MAX_RESPONSE_FILES).map(|_| response_file(0))).is_ok());
        assert!(
            checked_response_len(0, (0..MAX_RESPONSE_FILES + 1).map(|_| response_file(0))).is_err()
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn response_fields_and_public_results_retain_protocol_range_checks() {
        assert_eq!(
            checked_response_len(0, [response_file(u32::MAX as usize + 1)].into_iter())
                .unwrap_err(),
            "response field too large"
        );
        let result = ok(
            json!({}),
            vec![OperationFile {
                name: "x".into(),
                mime: "x".into(),
                resource_id: Some(i64::MAX as usize + 1),
                bytes: Vec::new(),
            }],
        );
        assert_eq!(
            result.into_envelope().unwrap_err(),
            "resource ID exceeds protocol range"
        );
    }
}
