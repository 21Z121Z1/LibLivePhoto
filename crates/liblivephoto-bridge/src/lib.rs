#![forbid(unsafe_code)]
//! Owned adapter boundary. Keeps parsers, identity rules and timing in the core.
//! Browser clients run this work in a disposable worker; native callers can
//! cancel at the documented phase boundaries using the progress callback.
use liblivephoto::{
    ColorDescription, Input, MediaTime, MotionPhoto, MotionPhotoReadError, ParseOptions, TrackKind,
};
use serde_json::{json, Value};
use std::ops::ControlFlow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Parse,
    Validate,
    Extract,
    Complete,
}
#[derive(Debug)]
pub enum InspectError {
    Cancelled,
    Core(MotionPhotoReadError),
}
impl From<MotionPhotoReadError> for InspectError {
    fn from(value: MotionPhotoReadError) -> Self {
        Self::Core(value)
    }
}
impl std::fmt::Display for InspectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("operation cancelled"),
            Self::Core(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for InspectError {}
impl InspectError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Core(MotionPhotoReadError::IdentifierMismatch) => "identifierMismatch",
            Self::Core(MotionPhotoReadError::MissingPairMetadata) => "missingPairMetadata",
            Self::Core(MotionPhotoReadError::InputLimit) => "inputLimit",
            Self::Core(MotionPhotoReadError::UnsupportedLayout(_)) => "unsupportedLayout",
            _ => "invalidInput",
        }
    }
}

pub fn exact_time(t: MediaTime) -> Value {
    json!({"value":t.value.to_string(),"timescale":t.timescale})
}
fn color(c: &ColorDescription) -> Value {
    match c {
        ColorDescription::Cicp {
            primaries,
            transfer,
            matrix,
            full_range,
        } => {
            json!({"kind":"cicp","primaries":primaries,"transfer":transfer,"matrix":matrix,"fullRange":full_range,"hdrTransfer":c.hdr_transfer().map(|v|format!("{v:?}"))})
        }
        ColorDescription::Icc { kind, profile } => {
            json!({"kind":"icc","type":String::from_utf8_lossy(kind),"bytes":profile.len(),"interpretation":"retained, not rendered"})
        }
        ColorDescription::Uninterpreted(raw) => json!({"kind":"uninterpreted","bytes":raw.len()}),
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Inspection {
    manifest: String,
    still: Vec<u8>,
    movie: Vec<u8>,
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
impl Inspection {
    pub fn manifest(&self) -> String {
        self.manifest.clone()
    }
    pub fn still_bytes(&self) -> Vec<u8> {
        self.still.clone()
    }
    pub fn movie_bytes(&self) -> Vec<u8> {
        self.movie.clone()
    }
}

/// The callback is called only at real phase boundaries; no artificial percent
/// is supplied for parsing. Core parsing is synchronous and not interruptible
/// within a phase. Browser cancellation terminates the dedicated worker.
pub fn inspect(
    input: Input<'_>,
    progress: impl FnMut(Phase) -> ControlFlow<()>,
) -> Result<Inspection, InspectError> {
    inspect_with_handlers(input, &liblivephoto::HandlerOptions::default(), progress)
}
pub fn inspect_with_handlers(
    input: Input<'_>,
    handlers: &liblivephoto::HandlerOptions,
    mut progress: impl FnMut(Phase) -> ControlFlow<()>,
) -> Result<Inspection, InspectError> {
    let mut phase = |p| {
        if progress(p).is_break() {
            Err(InspectError::Cancelled)
        } else {
            Ok(())
        }
    };
    phase(Phase::Parse)?;
    let mut options = ParseOptions::compatible();
    options.max_input_bytes = 128 * 1024 * 1024;
    let photo = MotionPhoto::parse_with_handlers(input, options, handlers)?;
    phase(Phase::Validate)?;
    let report = photo.validate()?;
    let mut diagnostics = photo
        .diagnostics()
        .iter()
        .chain(report.diagnostics.iter())
        .map(|d| format!("{d:?}"))
        .collect::<Vec<_>>();
    let still_metadata = match photo.still_metadata() {
        Ok(m) => {
            json!({"status":"observed","exifOrientation":m.exif_orientation,"exifTransform":m.exif_transform.map(|t|t.matrix),"heifTransform":m.heif_transform.map(|t|t.matrix),"color":m.color.iter().map(color).collect::<Vec<_>>(),"properties":m.primary_properties.iter().map(|p|json!({"type":String::from_utf8_lossy(&p.kind),"bytes":p.bytes.len()})).collect::<Vec<_>>()})
        }
        Err(e) => {
            diagnostics.push(format!("still display facts unavailable: {e}"));
            json!({"status":"unavailable","reason":e.to_string()})
        }
    };
    let descriptions=match photo.sample_descriptions() {
        Ok(list)=>list.iter().map(|s|json!({"trackId":s.track_id,"index":s.index,"codec":String::from_utf8_lossy(&s.format),"matrix":s.track_matrix,"color":s.color.iter().map(color).collect::<Vec<_>>(),"audio":s.audio.as_ref().map(|a|json!({"channels":a.sample_entry_channels,"rate16_16":a.sample_entry_rate_16_16,"aac":a.aac.as_ref().map(|c|json!({"objectType":c.object_type,"sampleRate":c.sample_rate,"channels":c.channels}))}))})).collect::<Vec<_>>(),
        Err(e)=>{diagnostics.push(format!("sample descriptions unavailable: {e}"));Vec::new()}
    };
    let tracks=photo.asset().containers.iter().filter(|c|c.resource==photo.motion_video().id).flat_map(|c|&c.tracks).map(|t| {
        let frames=if t.kind==TrackKind::Video {match photo.frame_times(t.id,100_000) {
            Ok(frames)=>json!({"status":"available","frames":frames.iter().map(|f|json!({"sampleIndex":f.sample_index,"time":exact_time(f.time)})).collect::<Vec<_>>()}),
            Err(e)=>{diagnostics.push(format!("track {} frame timing unavailable: {e}",t.id));json!({"status":"unavailable","reason":e.to_string()})}
        }} else {Value::Null};
        json!({"id":t.id,"kind":format!("{:?}",t.kind),"timescale":t.timescale,"duration":t.duration.to_string(),"codecs":t.sample_entries.iter().map(|v|String::from_utf8_lossy(v).into_owned()).collect::<Vec<_>>(),"frameTiming":frames})
    }).collect::<Vec<_>>();
    let manifest = json!({"schemaVersion":1,"libraryVersion":env!("CARGO_PKG_VERSION"),"format":format!("{:?}",photo.format()),"dialect":format!("{:?}",photo.asset().dialect),"layout":format!("{:?}",photo.asset().layout),"pairIdentifier":photo.asset().pairing.identifier,"association":photo.asset().provenance.evidence,"photoTime":photo.presentation_time().map(exact_time),"timeSource":photo.asset().presentation.as_ref().map(|p|format!("{:?}",p.source)),"stillMime":photo.still().mime,"movieMime":photo.motion_video().mime,"resources":photo.resources().map(|r|json!({"id":r.id.0,"role":format!("{:?}",r.role),"mime":r.mime,"sourceIndex":r.source_index,"parent":r.parent.map(|id|id.0),"vendorRole":r.vendor_role,"extents":r.extents.iter().map(|e|json!({"start":e.lower_bound.to_string(),"end":e.upper_bound.to_string()})).collect::<Vec<_>>(),"evidence":r.provenance.evidence})).collect::<Vec<_>>(),"tracks":tracks,"sampleDescriptions":descriptions,"stillMetadata":still_metadata,"diagnostics":diagnostics,"validation":format!("{:?}",report.checks),"capabilities":{"inspect":true,"extract":true,"renderAgtm":false,"photoKitImport":"native-adapter-required","edit":"native-adapter-required","capture":"host-capture-session-required"}});
    phase(Phase::Extract)?;
    let mut still = Vec::new();
    photo.extract(photo.still(), &mut still)?;
    let mut movie = Vec::new();
    photo.extract(photo.motion_video(), &mut movie)?;
    phase(Phase::Complete)?;
    Ok(Inspection {
        manifest: manifest.to_string(),
        still,
        movie,
    })
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;
    fn js_error(e: InspectError) -> JsValue {
        JsValue::from_str(&json!({"code":e.code(),"message":e.to_string()}).to_string())
    }
    #[wasm_bindgen]
    pub fn inspect_motion(bytes: &[u8]) -> Result<Inspection, JsValue> {
        inspect(Input::SingleFile(bytes), |_| ControlFlow::Continue(())).map_err(js_error)
    }
    #[wasm_bindgen]
    pub fn inspect_pair(
        still: &[u8],
        movie: &[u8],
        declared: bool,
        time_value: &str,
        timescale: u32,
    ) -> Result<Inspection, JsValue> {
        let input = if declared {
            let presentation = if time_value.is_empty() {
                None
            } else {
                Some(
                    MediaTime::new(
                        time_value.parse().map_err(|_| {
                            JsValue::from_str(
                                "{\"code\":\"invalidTime\",\"message\":\"invalid i64 time\"}",
                            )
                        })?,
                        timescale,
                    )
                    .ok_or_else(|| JsValue::from_str("invalid timescale"))?,
                )
            };
            Input::Parts {
                still,
                movie,
                presentation,
            }
        } else {
            Input::ApplePair { still, movie }
        };
        inspect(input, |_| ControlFlow::Continue(())).map_err(js_error)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_precedes_parsing_and_reports_real_phase() {
        let mut seen = Vec::new();
        let e = inspect(Input::SingleFile(&[]), |p| {
            seen.push(p);
            ControlFlow::Break(())
        })
        .err()
        .unwrap();
        assert_eq!(e.code(), "cancelled");
        assert_eq!(seen, vec![Phase::Parse]);
    }
    #[test]
    fn rational_json_never_rounds_large_values() {
        assert_eq!(
            exact_time(MediaTime {
                value: 9_007_199_254_740_993,
                timescale: 3
            })["value"],
            "9007199254740993"
        );
    }
    #[test]
    fn invalid_pair_is_not_silently_redeclared() {
        let e = inspect(
            Input::ApplePair {
                still: &[],
                movie: &[],
            },
            |_| ControlFlow::Continue(()),
        )
        .err()
        .unwrap();
        assert_ne!(e.code(), "cancelled");
    }
}
