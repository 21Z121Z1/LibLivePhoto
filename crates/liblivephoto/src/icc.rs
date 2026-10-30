//! ICC.1:2022 v4.4 envelope checks, without tag interpretation or colour rendering.
use crate::{
    ByteRange, FormatConformanceReport, FormatIssue, FormatIssueKind, MotionPhotoReadError as Error,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IccTag<'a> {
    pub signature: [u8; 4],
    pub data_type: [u8; 4],
    pub extent: ByteRange,
    /// Exact borrowed data, including unknown/private tag types. Shared extents
    /// remain shared references; tag payloads are never copied or normalized.
    pub raw: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IccStructure<'a> {
    pub raw: &'a [u8],
    pub declared_size: u32,
    pub version: [u8; 4],
    /// Only4.4.0.0 is certified against the pinned envelope rules. A different
    /// version may fit that envelope but is reported unsupported, not conforming.
    pub version_supported: bool,
    pub tags: Vec<IccTag<'a>>,
    /// Additional tag references reusing an identical earlier data extent.
    pub shared_references: usize,
}

fn bad(message: &'static str) -> Error {
    Error::MetadataMismatch(message)
}
fn word(p: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(
        p.get(at..at + 4)
            .ok_or_else(|| bad("truncated ICC field"))?
            .try_into()
            .map_err(|_| bad("ICC field width"))?,
    ))
}

/// Validate the fixed header, exact profile size, tag-table and data topology
/// from ICC.1:2022 §§7.1–7.4. Unknown tag contents are retained, not decoded.
/// This does not check date/class/PCS semantics, required tags, profile ID hashes, transforms,
/// registry membership or colour accuracy. Limits:16MiB input and4096 tags.
/// Other profile versions that fit this envelope are explicitly unsupported.
pub fn inspect_icc_structure(raw: &[u8]) -> Result<IccStructure<'_>, Error> {
    if raw.len() > 16 * 1024 * 1024 {
        return Err(Error::InputLimit);
    }
    if raw.len() < 132 {
        return Err(bad("truncated ICC header or tag count"));
    }
    let declared_size = word(raw, 0)?;
    if declared_size as usize != raw.len() || !raw.len().is_multiple_of(4) {
        return Err(bad("ICC declared size, input extent or final alignment"));
    }
    if &raw[36..40] != b"acsp" {
        return Err(bad("ICC profile signature"));
    }
    let version: [u8; 4] = raw[8..12].try_into().map_err(|_| bad("ICC version"))?;
    let supported = version == [4, 0x40, 0, 0];
    if raw[10..12] != [0; 2] || raw[100..128] != [0; 28] {
        return Err(bad("ICC reserved header bytes"));
    }
    if word(raw, 64)? > 3 {
        return Err(bad("ICC rendering-intent encoding"));
    }
    let count = word(raw, 128)? as usize;
    if count > 4096 {
        return Err(Error::InputLimit);
    }
    let table_end = 132usize
        .checked_add(count.checked_mul(12).ok_or(Error::InputLimit)?)
        .filter(|end| *end <= raw.len())
        .ok_or_else(|| bad("truncated ICC tag table"))?;
    let mut signatures = BTreeSet::new();
    let mut unique = BTreeMap::new();
    let mut tags = Vec::with_capacity(count);
    for i in 0..count {
        let row = 132 + i * 12;
        let signature: [u8; 4] = raw[row..row + 4]
            .try_into()
            .map_err(|_| bad("ICC tag signature"))?;
        if !signatures.insert(signature) {
            return Err(bad("duplicate ICC tag signature"));
        }
        let offset = word(raw, row + 4)? as usize;
        let size = word(raw, row + 8)? as usize;
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= raw.len())
            .ok_or_else(|| bad("ICC tag extent exceeds profile"))?;
        if offset < table_end || !offset.is_multiple_of(4) || size < 8 {
            return Err(bad(
                "ICC tag alignment, header overlap or truncated type header",
            ));
        }
        if raw[offset + 4..offset + 8] != [0; 4] {
            return Err(bad("ICC tag type reserved bytes"));
        }
        if let Some(previous) = unique.insert(offset, end) {
            if previous != end {
                return Err(bad("ICC shared offsets have inconsistent lengths"));
            }
        }
        tags.push(IccTag {
            signature,
            data_type: raw[offset..offset + 4]
                .try_into()
                .map_err(|_| bad("ICC tag type"))?,
            extent: ByteRange::new(offset as u64, end as u64)?,
            raw: &raw[offset..end],
        });
    }
    let mut next = table_end;
    for (&offset, &end) in &unique {
        if offset != next {
            return Err(bad("ICC tag data overlaps or contains unreferenced gaps"));
        }
        let padded = end
            .checked_add(3)
            .map(|v| v & !3)
            .filter(|v| *v <= raw.len())
            .ok_or_else(|| bad("ICC final tag padding exceeds profile"))?;
        if raw[end..padded].iter().any(|b| *b != 0) {
            return Err(bad("ICC nonzero tag padding"));
        }
        next = padded;
    }
    if next != raw.len() {
        return Err(bad("ICC trailing unreferenced bytes"));
    }
    Ok(IccStructure {
        raw,
        declared_size,
        version,
        version_supported: supported,
        shared_references: tags.len() - unique.len(),
        tags,
    })
}

/// Scoped structural report. Success never certifies tag semantics or rendering.
pub fn validate_icc_structure(raw: &[u8]) -> Result<FormatConformanceReport, Error> {
    let profile = inspect_icc_structure(raw)?;
    let mut report = FormatConformanceReport {
        profile: "icc.1-2022-v4.4-structure",
        checked_rules: vec![
            "ICC7.header-size",
            "ICC7.tag-table",
            "ICC7.extents-sharing-padding",
        ],
        issues: Vec::new(),
    };
    if !profile.version_supported {
        report.issue(FormatIssue { rule: "ICC7.version", kind: FormatIssueKind::Unsupported,
            item_id: None, track_id: None,
            message: "Envelope fits selected ICC4.4 structure rules; this profile version is unsupported. Tag semantics and colour rendering are not certified.".into() })?;
    }
    Ok(report)
}
