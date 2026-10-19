//! Source-scoped declaration checks; never native pairing or imageTime inference.
use crate::{
    ByteRange, CameraAlbumFooter, DeclaredIdentifierAgreement, MotionPhoto,
    MotionPhotoReadError as Error, ResourceId, XmpDocument,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VivoFooterOrigin {
    TerminalSource,
    MotionVideoUuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VivoFooterObservation<'a> {
    pub source_index: usize,
    /// All ranges refer to source_bytes(source_index), including UUID observations.
    pub extents: Vec<ByteRange>,
    pub json_extents: Vec<ByteRange>,
    pub identifier_extents: Vec<ByteRange>,
    /// Two origins can be aliases of one physical footer, not independent evidence.
    pub origins: Vec<VivoFooterOrigin>,
    pub envelope_identifier: &'a str,
    pub declared_identifier: Option<String>,
    pub raw_json: &'a [u8],
    pub version: Option<i64>,
    /// Opaque integer with an unverified unit; not compared with XMP timestamps.
    pub image_time: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VivoXmpSpanStatus {
    Matched,
    Mismatch,
    OutOfRange,
    InvalidDeclaration,
    NotComparable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VivoXmpSpanCheck {
    pub resource: ResourceId,
    pub source_index: usize,
    pub xmp_extents: Vec<ByteRange>,
    /// "directory" or "microvideo-offset". Raw XMP stays in the resource.
    pub declaration: &'static str,
    pub declared_length: Option<u64>,
    pub declared_extent: Option<ByteRange>,
    pub status: VivoXmpSpanStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VivoDeclarationIssue {
    pub code: &'static str,
    pub message: String,
    pub footer_indices: Vec<usize>,
    pub xmp_resource: Option<ResourceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VivoDeclarationReport<'a> {
    pub footers: Vec<VivoFooterObservation<'a>>,
    pub xmp_spans: Vec<VivoXmpSpanCheck>,
    pub issues: Vec<VivoDeclarationIssue>,
    /// Counts like-field comparisons, not independent originals or capture pairs.
    pub identifier_comparisons: usize,
}

fn ranges_json(ranges: &[ByteRange]) -> Value {
    json!(ranges
        .iter()
        .map(|r| json!({"start":r.lower_bound.to_string(),"end":r.upper_bound.to_string()}))
        .collect::<Vec<_>>())
}
impl VivoDeclarationReport<'_> {
    /// JSON projections do not change the validity of the core media container.
    pub fn json(&self) -> Value {
        json!({"footers": self.footers.iter().map(|f|json!({
            "sourceIndex":f.source_index,"extents":ranges_json(&f.extents),
            "jsonExtents":ranges_json(&f.json_extents),"identifierExtents":ranges_json(&f.identifier_extents),
            "origins":f.origins.iter().map(|o|format!("{o:?}")).collect::<Vec<_>>(),
            "envelopeIdentifier":f.envelope_identifier,"declaredIdentifier":f.declared_identifier,
            "rawJson":std::str::from_utf8(f.raw_json).ok(),
            "version":f.version,"imageTime":f.image_time.map(|n|n.to_string()),"imageTimeUnit":"unverified"
        })).collect::<Vec<_>>(),
        "xmpSpans": self.xmp_spans.iter().map(|x|json!({"resourceId":x.resource.0,"sourceIndex":x.source_index,
            "xmpExtents":ranges_json(&x.xmp_extents),"declaration":x.declaration,
            "declaredLength":x.declared_length.map(|n|n.to_string()),
            "declaredExtents":x.declared_extent.map(|r|ranges_json(&[r])),"status":format!("{:?}",x.status)
        })).collect::<Vec<_>>(),
        "issues":self.issues.iter().map(|i|json!({"code":i.code,"message":i.message,
            "footerIndices":i.footer_indices,"xmpResourceId":i.xmp_resource.map(|r|r.0)})).collect::<Vec<_>>(),
        "identifierComparisons":self.identifier_comparisons,"pairingVerified":false,
        "imageTimeUnit":"unverified","streamSemanticsInterpreted":false})
    }

    fn issue(
        &mut self,
        code: &'static str,
        message: String,
        indices: Vec<usize>,
        xmp: Option<ResourceId>,
    ) {
        self.issues.push(VivoDeclarationIssue {
            code,
            message,
            footer_indices: indices,
            xmp_resource: xmp,
        });
    }
}

impl MotionPhoto<'_> {
    /// Audit observed CameraAlbum/UUID declarations and standard XMP media spans.
    /// Same-source physical aliases are merged; equal identifiers never prove
    /// capture pairing. XMP spans are compared only for a single appended JPEG
    /// source, not HEIF or caller-associated Parts/Apple pairs. Limits: the existing
    /// footer/UUID limits, at most64 XMP resources /8MiB total XML. No mutation.
    /// None means there is no observed footer/UUID contract to audit.
    pub fn vivo_declaration_report(&self) -> Result<Option<VivoDeclarationReport<'_>>, Error> {
        self.vivo_declaration_report_with_terminal(true)
    }

    pub(crate) fn vivo_declaration_report_with_terminal(
        &self,
        terminal: bool,
    ) -> Result<Option<VivoDeclarationReport<'_>>, Error> {
        let mut report = VivoDeclarationReport {
            footers: Vec::new(),
            xmp_spans: Vec::new(),
            issues: Vec::new(),
            identifier_comparisons: 0,
        };
        if terminal {
            for source in &self.asset().sources {
                let data = self
                    .source_bytes(source.index)
                    .ok_or(Error::InvalidResource)?;
                if let Some(footer) = crate::inspect_camera_album_footer(data)? {
                    observe(
                        &mut report,
                        footer,
                        source.index,
                        VivoFooterOrigin::TerminalSource,
                        |r| Ok(vec![r]),
                    )?;
                }
            }
        }
        let movie = self.motion_video_bytes()?;
        // This is only a bounded preflight; actual association remains top-level UUID parsing.
        let extension = if movie
            .windows(16)
            .any(|w| w == b"vivoMediaEStream" || w == b"vivoMediaExtInfo")
        {
            crate::inspect_vivo_media_extensions(movie)?
        } else {
            None
        };
        if let Some(footer) = extension.as_ref().and_then(|e| e.footer.clone()) {
            let resource = self.motion_video();
            let source = self
                .source_bytes(resource.source_index)
                .ok_or(Error::InvalidResource)?;
            observe(
                &mut report,
                footer,
                resource.source_index,
                VivoFooterOrigin::MotionVideoUuid,
                |r| {
                    let mapped =
                        crate::metadata_engine::subrange(&resource.extents, r, movie.len())?;
                    if mapped.iter().any(|r| r.upper_bound > source.len() as u64) {
                        return Err(Error::InvalidResource);
                    }
                    Ok(mapped)
                },
            )?;
        }
        if report.footers.is_empty() && extension.is_none() {
            return Ok(None);
        }
        for a in 0..report.footers.len() {
            for b in a + 1..report.footers.len() {
                report.identifier_comparisons += 1;
                if report.footers[a].envelope_identifier != report.footers[b].envelope_identifier {
                    report.issue(
                        "envelope-identifiers-differ",
                        "Distinct physical footer envelopes declare different identifiers".into(),
                        vec![a, b],
                        None,
                    );
                }
                if let (Some(x), Some(y)) = (
                    &report.footers[a].declared_identifier,
                    &report.footers[b].declared_identifier,
                ) {
                    report.identifier_comparisons += 1;
                    if x != y {
                        report.issue(
                            "json-identifiers-differ",
                            "Distinct physical footers declare different JSON identifiers".into(),
                            vec![a, b],
                            None,
                        );
                    }
                }
            }
        }
        let mut count = 0usize;
        let mut total = 0u64;
        for resource in self.resources().filter(|r| {
            r.mime == "application/rdf+xml" || r.vendor_role.as_deref() == Some("JPEG XMP")
        }) {
            count += 1;
            total = resource.extents.iter().try_fold(total, |n, r| {
                n.checked_add(r.length()).ok_or(Error::InputLimit)
            })?;
            if count > 64 || total > 8 * 1024 * 1024 {
                return Err(Error::InputLimit);
            }
            let mut xml = Vec::new();
            self.extract(resource, &mut xml)?;
            let prefix = if resource.vendor_role.as_deref() == Some("JPEG XMP") {
                b"http://ns.adobe.com/xap/1.0/\0".len()
            } else {
                0
            };
            let xmp_extents = crate::metadata_engine::subrange(
                &resource.extents,
                ByteRange::new(prefix as u64, xml.len() as u64)?,
                xml.len(),
            )?;
            let xml = xml.get(prefix..).ok_or(Error::InvalidResource)?;
            let document = match XmpDocument::parse(xml) {
                Ok(d) => d,
                Err(e) => {
                    report.issue("xmp-invalid", e.to_string(), Vec::new(), Some(resource.id));
                    continue;
                }
            };
            let fields = document.fields();
            let comparable = self.asset().sources.len() == 1
                && self.still().mime == "image/jpeg"
                && resource.source_index == self.motion_video().source_index;
            let source = self
                .source_bytes(resource.source_index)
                .ok_or(Error::InvalidResource)?;
            let mut check = |name, length: Option<u64>, valid: bool| {
                let declared_extent = if comparable && valid {
                    length
                        .filter(|n| *n > 0)
                        .and_then(|n| (source.len() as u64).checked_sub(n))
                        .map(|start| ByteRange {
                            lower_bound: start,
                            upper_bound: source.len() as u64,
                        })
                } else {
                    None
                };
                let status = if !valid {
                    VivoXmpSpanStatus::InvalidDeclaration
                } else if !comparable {
                    VivoXmpSpanStatus::NotComparable
                } else if declared_extent.is_none() {
                    VivoXmpSpanStatus::OutOfRange
                } else if self.motion_video().extents.as_slice() == declared_extent.as_slice() {
                    VivoXmpSpanStatus::Matched
                } else {
                    VivoXmpSpanStatus::Mismatch
                };
                if matches!(
                    status,
                    VivoXmpSpanStatus::InvalidDeclaration
                        | VivoXmpSpanStatus::OutOfRange
                        | VivoXmpSpanStatus::Mismatch
                ) {
                    report.issue("xmp-media-span",format!("{name} declaration: {status:?}; no proprietary timestamp unit inferred"),Vec::new(),Some(resource.id));
                }
                report.xmp_spans.push(VivoXmpSpanCheck {
                    resource: resource.id,
                    source_index: resource.source_index,
                    xmp_extents: xmp_extents.clone(),
                    declaration: name,
                    declared_length: length,
                    declared_extent,
                    status,
                });
            };
            if fields.camera_values.get("MotionPhoto") == Some(&1) {
                check(
                    "directory",
                    fields.directory.last().map(|i| i.length),
                    document.validate_motion().is_ok(),
                );
            }
            if fields.camera_values.get("MicroVideo") == Some(&1) {
                let length = fields
                    .camera_values
                    .get("MicroVideoOffset")
                    .and_then(|n| u64::try_from(*n).ok());
                check("microvideo-offset", length, length.is_some_and(|n| n > 0));
            }
        }
        Ok(Some(report))
    }
}

fn observe<'a>(
    report: &mut VivoDeclarationReport<'a>,
    footer: CameraAlbumFooter<'a>,
    source_index: usize,
    origin: VivoFooterOrigin,
    map: impl Fn(ByteRange) -> Result<Vec<ByteRange>, Error>,
) -> Result<(), Error> {
    let extents = map(footer.extent)?;
    if let Some(existing) = report
        .footers
        .iter_mut()
        .find(|f| f.source_index == source_index && f.extents == extents)
    {
        if !existing.origins.contains(&origin) {
            existing.origins.push(origin);
        }
        return Ok(());
    }
    let index = report.footers.len();
    report.identifier_comparisons +=
        usize::from(footer.identifier_agreement != DeclaredIdentifierAgreement::Absent);
    if footer.identifier_agreement == DeclaredIdentifierAgreement::Different {
        report.issue(
            "footer-internal-identifiers-differ",
            "JSON and binary envelope identifiers differ within one footer".into(),
            vec![index],
            None,
        );
    }
    report.footers.push(VivoFooterObservation {
        source_index,
        extents,
        json_extents: map(footer.json_extent)?,
        identifier_extents: map(footer.identifier_extent)?,
        origins: vec![origin],
        envelope_identifier: footer.envelope_identifier,
        declared_identifier: footer.declared_identifier,
        raw_json: footer.raw_json,
        version: footer.version,
        image_time: footer.image_time,
    });
    Ok(())
}
