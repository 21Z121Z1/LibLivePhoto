//! Exact movie-timeline rescaling. Encoded samples and media timelines do not change.
use crate::{MediaTime, MotionPhotoReadError as Error};
use liblivephoto_format::isobmff::{parse_boxes, BoxHeader};
type Result<T> = std::result::Result<T, Error>;
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
fn u32_at(p: &[u8], n: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        p.get(n..n + 4)
            .and_then(|v| v.try_into().ok())
            .ok_or(Error::InvalidPresentation)?,
    ))
}
fn children(data: &[u8], b: &BoxHeader) -> Result<Vec<BoxHeader>> {
    parse_boxes(data, b.data_start..b.data_end).map_err(|_| Error::InvalidPresentation)
}
fn version(data: &[u8], b: &BoxHeader) -> Result<u8> {
    match data.get(b.data_start) {
        Some(v @ 0..=1) => Ok(*v),
        _ => Err(Error::InvalidPresentation),
    }
}
fn duration_patch(
    data: &[u8],
    b: &BoxHeader,
    relative: usize,
    width: usize,
    factor: u64,
    patches: &mut Vec<(usize, Vec<u8>)>,
) -> Result<()> {
    let pos = b.data_start + relative;
    let p = data
        .get(pos..pos + width)
        .filter(|_| pos + width <= b.data_end)
        .ok_or(Error::InvalidPresentation)?;
    let v = if width == 4 {
        u64::from(u32_at(p, 0)?)
    } else {
        u64::from_be_bytes(p.try_into().map_err(|_| Error::InvalidPresentation)?)
    };
    let scaled = v.checked_mul(factor).ok_or(Error::InvalidPresentation)?;
    let bytes = if width == 4 {
        u32::try_from(scaled)
            .map_err(|_| Error::InvalidPresentation)?
            .to_be_bytes()
            .to_vec()
    } else {
        scaled.to_be_bytes().to_vec()
    };
    patches.push((pos, bytes));
    Ok(())
}
pub(crate) fn patches(movie: &[u8], time: MediaTime) -> Result<Vec<(usize, Vec<u8>)>> {
    if time.timescale == 0 || time.value < 0 {
        return Err(Error::InvalidPresentation);
    }
    let top = parse_boxes(movie, 0..movie.len()).map_err(|_| Error::InvalidPresentation)?;
    let moov = top
        .iter()
        .find(|b| b.kind.as_bytes() == b"moov")
        .ok_or(Error::InvalidPresentation)?;
    let bs = children(movie, moov)?;
    let mvhd = bs
        .iter()
        .find(|b| b.kind.as_bytes() == b"mvhd")
        .ok_or(Error::InvalidPresentation)?;
    let v = version(movie, mvhd)?;
    let scale_pos = if v == 0 { 12 } else { 20 };
    let p = &movie[mvhd.data_start..mvhd.data_end];
    let scale = u32_at(p, scale_pos)?;
    if scale == 0 {
        return Err(Error::InvalidPresentation);
    }
    let denominator = u64::from(time.timescale) / gcd(time.value as u64, u64::from(time.timescale));
    let new_scale = u64::from(scale)
        .checked_div(gcd(u64::from(scale), denominator))
        .and_then(|n| n.checked_mul(denominator))
        .ok_or(Error::InvalidPresentation)?;
    let new_scale = u32::try_from(new_scale).map_err(|_| Error::InvalidPresentation)?;
    if time
        .rescale_exact(new_scale)
        .is_none_or(|t| t.value > i64::from(u32::MAX))
    {
        return Err(Error::InvalidPresentation);
    }
    if new_scale == scale {
        return Ok(Vec::new());
    }
    if top.iter().any(|b| b.kind.as_bytes() == b"moof")
        || bs.iter().any(|b| b.kind.as_bytes() == b"mvex")
    {
        return Err(Error::InvalidPresentation);
    }
    let factor = u64::from(new_scale / scale);
    let mut patches = vec![(
        mvhd.data_start + scale_pos,
        new_scale.to_be_bytes().to_vec(),
    )];
    duration_patch(
        movie,
        mvhd,
        scale_pos + 4,
        if v == 0 { 4 } else { 8 },
        factor,
        &mut patches,
    )?;
    for t in bs.iter().filter(|b| b.kind.as_bytes() == b"trak") {
        let tc = children(movie, t)?;
        if let Some(tkhd) = tc.iter().find(|b| b.kind.as_bytes() == b"tkhd") {
            let v = version(movie, tkhd)?;
            duration_patch(
                movie,
                tkhd,
                if v == 0 { 20 } else { 28 },
                if v == 0 { 4 } else { 8 },
                factor,
                &mut patches,
            )?;
        }
        if let Some(edts) = tc.iter().find(|b| b.kind.as_bytes() == b"edts") {
            for elst in children(movie, edts)?
                .iter()
                .filter(|b| b.kind.as_bytes() == b"elst")
            {
                let v = version(movie, elst)?;
                let width = if v == 0 { 12 } else { 20 };
                let p = &movie[elst.data_start..elst.data_end];
                let count = u32_at(p, 4)? as usize;
                if count > 4096 || p.len().checked_sub(8) != count.checked_mul(width) {
                    return Err(Error::InvalidPresentation);
                }
                for n in 0..count {
                    duration_patch(
                        movie,
                        elst,
                        8 + n * width,
                        if v == 0 { 4 } else { 8 },
                        factor,
                        &mut patches,
                    )?;
                }
            }
        }
    }
    Ok(patches)
}
pub(crate) fn rescale(movie: &[u8], time: MediaTime) -> Result<Vec<u8>> {
    let patches = patches(movie, time)?;
    let mut output = movie.to_vec();
    for (pos, bytes) in patches {
        output[pos..pos + bytes.len()].copy_from_slice(&bytes);
    }
    Ok(output)
}
