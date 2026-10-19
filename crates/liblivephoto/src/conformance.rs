//! Optional, explicitly scoped format rules. Preservation and device acceptance
//! are separate contracts; these checks never rewrite source metadata.
use crate::{MotionPhoto, MotionPhotoReadError as Error};
use liblivephoto_format::isobmff::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatIssueKind {
    Violation,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatIssue {
    pub rule: &'static str,
    pub kind: FormatIssueKind,
    pub item_id: Option<u32>,
    pub track_id: Option<u32>,
    pub message: String,
}

/// Evidence for only the listed rules, not a complete container/codec or native
/// player certification. Unsupported declarations never produce a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatConformanceReport {
    pub profile: &'static str,
    pub checked_rules: Vec<&'static str>,
    pub issues: Vec<FormatIssue>,
}
impl FormatConformanceReport {
    pub fn checked_rules_satisfied(&self) -> bool {
        self.issues.is_empty()
    }
    pub(crate) fn issue(&mut self, issue: FormatIssue) -> Result<(), Error> {
        if self.issues.len() >= 4096 {
            return Err(Error::InputLimit);
        }
        self.issues.push(issue);
        Ok(())
    }
}
fn fmt<T>(value: liblivephoto_format::Result<T>) -> Result<T, Error> {
    value.map_err(|_| Error::InvalidResource)
}
fn one<'a>(boxes: &'a [BoxHeader], kind: &[u8; 4]) -> Result<&'a BoxHeader, Error> {
    let mut found = boxes.iter().filter(|b| b.kind.as_bytes() == kind);
    let first = found.next().ok_or(Error::InvalidResource)?;
    if found.next().is_some() {
        return Err(Error::UnsupportedLayout(
            "ambiguous HEIF property containers",
        ));
    }
    Ok(first)
}
fn count(data: &[u8], b: &BoxHeader, short: bool) -> Result<usize, Error> {
    let p = &data[b.payload_range()];
    if short {
        Ok(usize::from(u16::from_be_bytes(
            p.get(4..6)
                .ok_or(Error::InvalidResource)?
                .try_into()
                .map_err(|_| Error::InvalidResource)?,
        )))
    } else {
        usize::try_from(u32::from_be_bytes(
            p.get(4..8)
                .ok_or(Error::InvalidResource)?
                .try_into()
                .map_err(|_| Error::InvalidResource)?,
        ))
        .map_err(|_| Error::InputLimit)
    }
}

/// Check the count, ICC/nclx pairing and paired nclx primaries/transfer rules of
/// ISO/IEC23008-12:2025 §6.5.5. All supported image-item associations, including
/// multiple ipma boxes, are examined. ICC profile contents, generic BMFF color
/// syntax, decoding and visual-track/image conversion are not certified.
/// Bounds:256MiB source,4096 items/properties/association rows/issues,
/// 65536 associations and16MiB referenced colr bytes, including repeated ranges.
pub fn validate_heif_color_information(data: &[u8]) -> Result<FormatConformanceReport, Error> {
    if data.len() > 256 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    if crate::facade::still_mime(data)? == "image/jpeg" {
        return Err(Error::UnsupportedLayout(
            "HEIF color rules require HEIC/AVIF",
        ));
    }
    let top = fmt(parse_boxes(data, 0..data.len()))?;
    let meta = one(&top, b"meta")?;
    if data.get(meta.data_start) != Some(&0) {
        return Err(Error::UnsupportedLayout(
            "HEIF meta version for color rules",
        ));
    }
    let children = fmt(parse_boxes(data, meta.data_start + 4..meta.data_end))?;
    let iinf = one(&children, b"iinf")?;
    let version = *data.get(iinf.data_start).ok_or(Error::InvalidResource)?;
    if version > 1 {
        return Err(Error::UnsupportedLayout(
            "HEIF iinf version for color rules",
        ));
    }
    if count(data, iinf, version == 0)? > 4096 {
        return Err(Error::InputLimit);
    }
    let info = fmt(parse_iinf(data, iinf))?;
    let mut item_types = BTreeMap::new();
    for item in &info.entries {
        if item_types.insert(item.item_id, item.item_type).is_some() {
            return Err(Error::UnsupportedLayout("duplicate HEIF item declarations"));
        }
    }
    let iprp = one(&children, b"iprp")?;
    let iprp_children = fmt(parse_boxes(data, iprp.payload_range()))?;
    let ipco = one(&iprp_children, b"ipco")?;
    let properties = fmt(parse_boxes(data, ipco.payload_range()))?;
    if properties.len() > 4096 {
        return Err(Error::InputLimit);
    }
    let ipmas: Vec<_> = iprp_children
        .iter()
        .filter(|b| b.kind.as_bytes() == b"ipma")
        .collect();
    if ipmas.is_empty() {
        return Err(Error::UnsupportedLayout(
            "HEIF color associations unavailable",
        ));
    }
    let mut groups = BTreeMap::<u32, Vec<(u16, &[u8])>>::new();
    let (mut rows, mut associations, mut color_bytes) = (0usize, 0usize, 0usize);
    for ipma in ipmas {
        let p = &data[ipma.payload_range()];
        if p.first().is_some_and(|v| *v > 1)
            || p.get(1..4)
                .is_some_and(|v| v[0] != 0 || v[1] != 0 || v[2] & !1 != 0)
        {
            return Err(Error::UnsupportedLayout(
                "HEIF ipma version/flags for color rules",
            ));
        }
        rows = rows
            .checked_add(count(data, ipma, false)?)
            .filter(|v| *v <= 4096)
            .ok_or(Error::InputLimit)?;
        // With the row/association bounds below, a supported encoded table is
        // <256KiB. Preflight this before the shared parser allocates its rows.
        if p.len() > 256 * 1024 {
            return Err(Error::InputLimit);
        }
        for row in fmt(parse_ipma(data, ipma))?.entries {
            associations = associations
                .checked_add(row.associations.len())
                .filter(|v| *v <= 65536)
                .ok_or(Error::InputLimit)?;
            for association in row.associations {
                if association.property_index == 0 {
                    continue;
                }
                let property = properties
                    .get(usize::from(association.property_index) - 1)
                    .ok_or(Error::InvalidResource)?;
                if property.kind.as_bytes() != b"colr" {
                    continue;
                }
                let payload = &data[property.payload_range()];
                color_bytes = color_bytes
                    .checked_add(payload.len())
                    .filter(|v| *v <= 16 * 1024 * 1024)
                    .ok_or(Error::InputLimit)?;
                groups
                    .entry(row.item_id)
                    .or_default()
                    .push((association.property_index, payload));
            }
        }
    }
    let mut report = FormatConformanceReport {
        profile: "heif-color-2025",
        checked_rules: vec![
            "H05.colr-count",
            "H05.colr-pair",
            "H05.paired-nclx-unspecified",
        ],
        issues: Vec::new(),
    };
    for (item_id, declarations) in groups {
        let mut issue = |rule, kind, message: String| {
            report.issue(FormatIssue {
                rule,
                kind,
                item_id: Some(item_id),
                track_id: None,
                message,
            })
        };
        let image = item_types
            .get(&item_id)
            .copied()
            .flatten()
            .is_some_and(|t| {
                matches!(
                    t.as_bytes(),
                    b"hvc1" | b"av01" | b"jpeg" | b"grid" | b"iden"
                )
            });
        if !image {
            issue("H05.item-kind",FormatIssueKind::Unsupported,"colr applicability to undeclared or unsupported image-item type is not established".into())?;
            continue;
        }
        let mut seen = BTreeSet::new();
        let mut colors = Vec::new();
        for (index, payload) in declarations {
            if seen.insert(index) {
                colors.push(payload);
            } else {
                issue(
                    "H05.association-layout",
                    FormatIssueKind::Unsupported,
                    "repeated association of the same colr property is not certified".into(),
                )?;
            }
        }
        if colors.len() > 2 {
            issue(
                "H05.colr-count",
                FormatIssueKind::Violation,
                format!(
                    "{} distinct colr properties; at most2 are allowed",
                    colors.len()
                ),
            )?;
            continue;
        }
        for payload in &colors {
            match payload.get(..4) {
                Some(b"nclx") if payload.len() == 11 => {}
                Some(b"rICC" | b"prof") => {}
                _ => issue(
                    "H05.color-declaration",
                    FormatIssueKind::Unsupported,
                    "color_type or its field layout is outside the checked color subset".into(),
                )?,
            }
        }
        if colors.len() == 2 {
            let icc = colors
                .iter()
                .filter(|p| matches!(p.get(..4), Some(b"rICC" | b"prof")))
                .count();
            let nclx = colors.iter().find(|p| p.get(..4) == Some(b"nclx"));
            if icc != 1 || nclx.is_none() {
                issue(
                    "H05.colr-pair",
                    FormatIssueKind::Violation,
                    "two colr properties require exactly one ICC (rICC/prof) and one nclx".into(),
                )?;
            } else if let Some(p) = nclx.filter(|p| p.len() == 11) {
                let primaries = u16::from_be_bytes([p[4], p[5]]);
                let transfer = u16::from_be_bytes([p[6], p[7]]);
                if primaries != 2 || transfer != 2 {
                    issue("H05.paired-nclx-unspecified",FormatIssueKind::Violation,format!("paired nclx requires primaries=2,transfer=2; observed {primaries},{transfer}"))?;
                }
            }
        }
    }
    Ok(report)
}
impl MotionPhoto<'_> {
    pub fn heif_color_conformance(&self) -> Result<FormatConformanceReport, Error> {
        validate_heif_color_information(self.still_bytes()?)
    }
}
