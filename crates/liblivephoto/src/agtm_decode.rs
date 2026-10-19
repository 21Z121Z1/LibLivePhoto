//! Complete parsing of the version-0 fields consumed by the pinned Google AGTM
//! implementation. Unknown component-mix scaling remains encoded data; it is not
//! replaced with the zero-valued mix returned by that implementation.
use crate::{AgtmHeader, MotionPhotoReadError as Error};
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgtmComponentMix {
    pub component_type: u8,
    /// Six optional 16-bit values, in bitstream order. Google consumes these
    /// values without decoding them. No weight scaling or channel role is inferred.
    pub encoded_coefficients: [Option<u16>; 6],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgtmControlPoint {
    pub x: f32,
    pub y: f32,
    /// Absent when the curve requests PCHIP-derived slopes.
    pub slope: Option<f32>,
    /// Original angle value, before the tangent conversion.
    pub encoded_slope_angle: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgtmToneMappingRule {
    pub alternate_hdr_headroom_log2: f32,
    pub component_mix: AgtmComponentMix,
    pub use_pchip_slope: bool,
    pub curve: Vec<AgtmControlPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgtmMetadata {
    pub header: AgtmHeader,
    /// Preset selectors are retained as selectors; their color-space semantics
    /// are not invented from Google's default BT.2020-valued struct initializer.
    pub color_space_mode: Option<u8>,
    pub custom_chromaticities: Option<[f32; 8]>,
    pub common_component_mix: bool,
    pub common_curve: bool,
    pub rules: Vec<AgtmToneMappingRule>,
    /// Relative to the unwrapped raw ST 2094-50 payload.
    pub consumed_bits: usize,
    /// Exact input, including its T.35 prefix and any unparsed extension bits.
    pub raw_bytes: Vec<u8>,
}

struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl Bits<'_> {
    fn take(&mut self, count: usize) -> Result<u32> {
        let end = self
            .position
            .checked_add(count)
            .filter(|p| *p <= self.bytes.len().saturating_mul(8))
            .ok_or(Error::UnsupportedLayout(
                "truncated AGTM curve/mix bitstream",
            ))?;
        let mut value = 0;
        for i in self.position..end {
            value = (value << 1) | u32::from((self.bytes[i / 8] >> (7 - i % 8)) & 1);
        }
        self.position = end;
        Ok(value)
    }
    fn bit(&mut self) -> Result<bool> {
        Ok(self.take(1)? != 0)
    }
    fn word(&mut self) -> Result<u16> {
        Ok(self.take(16)? as u16)
    }
    fn headroom(&mut self) -> Result<f32> {
        let value = self.word()?;
        if value > 60_000 {
            return Err(Error::UnsupportedLayout(
                "AGTM headroom is outside its supported range",
            ));
        }
        Ok(f32::from(value) / 10_000.0)
    }
    fn mix(&mut self) -> Result<AgtmComponentMix> {
        let component_type = self.take(2)? as u8;
        let mut encoded_coefficients = [None; 6];
        if component_type == 3 {
            let mut present = [false; 6];
            for flag in &mut present {
                *flag = self.bit()?;
            }
            for (value, flag) in encoded_coefficients.iter_mut().zip(present) {
                if flag {
                    *value = Some(self.word()?);
                }
            }
        } else {
            self.take(6)?;
        }
        Ok(AgtmComponentMix {
            component_type,
            encoded_coefficients,
        })
    }
}

/// Decode every field read by Google's version-0 payload decoder. Unlike the
/// reference, truncated curves/mixes and out-of-range counts fail explicitly.
/// The 64 KiB input bound also applies to preserved unknown extension bytes.
pub fn decode_agtm(data: &[u8]) -> Result<AgtmMetadata> {
    if data.len() > 65_536 {
        return Err(Error::InputLimit);
    }
    let header = crate::decode_agtm_header(data)?;
    let raw = crate::agtm::raw_payload(data);
    let mut bits = Bits {
        bytes: raw,
        position: header.remaining_bit_offset,
    };
    let adaptive = header.has_adaptive_tone_map;
    let reference_white = header.use_reference_white_tone_mapping == Some(true);
    let mut out = AgtmMetadata {
        header,
        color_space_mode: None,
        custom_chromaticities: None,
        common_component_mix: false,
        common_curve: false,
        rules: Vec::new(),
        consumed_bits: bits.position,
        raw_bytes: data.to_vec(),
    };
    if !adaptive || reference_white {
        return Ok(out);
    }
    let count = bits.take(3)? as usize;
    if count > 4 {
        return Err(Error::UnsupportedLayout(
            "AGTM alternate-image count exceeds four",
        ));
    }
    let mode = bits.take(2)? as u8;
    out.color_space_mode = Some(mode);
    out.common_component_mix = bits.bit()?;
    out.common_curve = bits.bit()?;
    if mode == 3 {
        let mut values = [0.0; 8];
        for value in &mut values {
            let raw = bits.word()?;
            if raw > 50_000 {
                return Err(Error::UnsupportedLayout(
                    "AGTM chromaticity is outside its supported range",
                ));
            }
            *value = f32::from(raw) / 50_000.0;
        }
        out.custom_chromaticities = Some(values);
    }
    for index in 0..count {
        let alternate_hdr_headroom_log2 = bits.headroom()?;
        let component_mix = if index == 0 || !out.common_component_mix {
            bits.mix()?
        } else {
            out.rules[0].component_mix.clone()
        };
        let (use_pchip_slope, xs) = if index == 0 || !out.common_curve {
            let points = bits.take(5)? as usize + 1;
            let pchip = bits.bit()?;
            bits.take(2)?;
            let xs = (0..points)
                .map(|_| bits.word().map(|v| f32::from(v) / 1000.0))
                .collect::<Result<Vec<_>>>()?;
            (pchip, xs)
        } else {
            (
                out.rules[0].use_pchip_slope,
                out.rules[0].curve.iter().map(|p| p.x).collect(),
            )
        };
        let sign = if out
            .header
            .baseline_hdr_headroom_log2
            .ok_or(Error::InvalidResource)?
            < alternate_hdr_headroom_log2
        {
            1.0
        } else {
            -1.0
        };
        let mut curve = xs
            .into_iter()
            .map(|x| {
                bits.word().map(|v| AgtmControlPoint {
                    x,
                    y: f32::from(v) / 10_000.0 * sign,
                    slope: None,
                    encoded_slope_angle: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if !use_pchip_slope {
            for point in &mut curve {
                let angle = bits.word()?;
                let theta = f32::from(angle) * std::f32::consts::PI / 36_000.0
                    - std::f32::consts::FRAC_PI_2;
                point.slope = Some(theta.tan());
                point.encoded_slope_angle = Some(angle);
            }
        }
        out.rules.push(AgtmToneMappingRule {
            alternate_hdr_headroom_log2,
            component_mix,
            use_pchip_slope,
            curve,
        });
    }
    out.consumed_bits = bits.position;
    Ok(out)
}

impl AgtmMetadata {
    /// Google's limited JSON view: only the first rule's x/y curve is projected.
    /// Values for inapplicable flags follow that view's defined defaults. The
    /// full object retains optional fields, all rules and original encoded data.
    pub fn reference_json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({"hdrReferenceWhite": self.header.hdr_reference_white_nits,
            "hasAdaptiveToneMap": self.header.has_adaptive_tone_map,
            "baselineHdrHeadroom": self.header.baseline_hdr_headroom_log2.unwrap_or(0.0),
            "useReferenceWhiteToneMapping": self.header.use_reference_white_tone_mapping.unwrap_or(false)});
        if let Some(first) = self.rules.first() {
            value["headroomAdaptiveToneMap"] = serde_json::json!({"alternateHdrHeadroom":first.alternate_hdr_headroom_log2,
                "curve": first.curve.iter().map(|p| serde_json::json!({"x":p.x,"y":p.y})).collect::<Vec<_>>()});
        }
        value
    }

    pub fn details_json(&self) -> serde_json::Value {
        serde_json::json!({"header":self.reference_json(),"colorSpaceMode":self.color_space_mode,
            "customChromaticities":self.custom_chromaticities,"commonComponentMix":self.common_component_mix,
            "commonCurve":self.common_curve,"consumedBits":self.consumed_bits,
            "rules":self.rules.iter().map(|r| serde_json::json!({"alternateHdrHeadroom":r.alternate_hdr_headroom_log2,
                "componentType":r.component_mix.component_type,"encodedMixCoefficients":r.component_mix.encoded_coefficients,
                "usePchipSlope":r.use_pchip_slope,"curve":r.curve.iter().map(|p| serde_json::json!({"x":p.x,"y":p.y,"slope":p.slope,"encodedSlopeAngle":p.encoded_slope_angle})).collect::<Vec<_>>() })).collect::<Vec<_>>()})
    }
}
