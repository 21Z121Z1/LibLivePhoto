#![allow(dead_code)]
pub fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    [
        ((payload.len() + 8) as u32).to_be_bytes().as_slice(),
        kind.as_slice(),
        payload,
    ]
    .concat()
}
pub fn full(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    boxed(kind, &[vec![0; 4], payload.to_vec()].concat())
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}
fn track(id: u32, scale: u32, offset: u32) -> Vec<u8> {
    track_samples(id, scale, offset, 1, scale * 2, 5)
}
fn track_samples(id: u32, scale: u32, offset: u32, count: u32, step: u32, size: u32) -> Vec<u8> {
    let mut tkhd = words(&[0, 0, id, 0, count * step]);
    tkhd.extend_from_slice(&[0; 60]);
    let mdhd = full(
        b"mdhd",
        &[words(&[0, 0, scale, count * step]), vec![0; 4]].concat(),
    );
    let hdlr = full(
        b"hdlr",
        &[
            words(&[0]),
            b"vide".to_vec(),
            vec![0; 12],
            b"Video\0".to_vec(),
        ]
        .concat(),
    );
    let stsd = full(b"stsd", &[words(&[1]), boxed(b"avc1", &[0; 78])].concat());
    let stts = full(b"stts", &words(&[1, count, step]));
    let stsc = full(b"stsc", &words(&[1, 1, count, 1]));
    let stsz = full(b"stsz", &words(&[size, count]));
    let stco = full(b"stco", &words(&[1, offset]));
    let stbl = boxed(b"stbl", &[stsd, stts, stsc, stsz, stco].concat());
    let mdia = boxed(b"mdia", &[mdhd, hdlr, boxed(b"minf", &stbl)].concat());
    boxed(b"trak", &[full(b"tkhd", &tkhd), mdia].concat())
}
pub fn many_frame_movie(count: u32, tracks: usize) -> Vec<u8> {
    let ftyp = boxed(b"ftyp", b"isom\0\0\0\0isom");
    let mvhd = full(
        b"mvhd",
        &[words(&[0, 0, 1000, count]), vec![0; 80]].concat(),
    );
    let offset = (ftyp.len() + 8) as u32;
    let moov = boxed(
        b"moov",
        &[
            mvhd,
            (0..tracks)
                .flat_map(|n| track_samples((n + 1) as u32, 1000, offset, count, 1, 1))
                .collect(),
        ]
        .concat(),
    );
    [ftyp, boxed(b"mdat", &vec![0x55; count as usize]), moov].concat()
}
pub fn movie(scale: u32, tracks: usize) -> Vec<u8> {
    let ftyp = boxed(b"ftyp", b"isom\0\0\0\0isom");
    let mvhd = full(
        b"mvhd",
        &[words(&[0, 0, scale, scale * 2]), vec![0; 80]].concat(),
    );
    let provisional = boxed(
        b"moov",
        &[
            mvhd.clone(),
            (0..tracks)
                .flat_map(|n| track((n + 1) as u32, scale, 0))
                .collect(),
        ]
        .concat(),
    );
    let moov = boxed(
        b"moov",
        &[
            mvhd,
            (0..tracks)
                .flat_map(|n| {
                    track(
                        (n + 1) as u32,
                        scale,
                        (ftyp.len() + provisional.len() + 8 + n * 5) as u32,
                    )
                })
                .collect(),
        ]
        .concat(),
    );
    let media = (0..tracks)
        .flat_map(|n| vec![n as u8 + 1; 5])
        .collect::<Vec<_>>();
    [ftyp, moov, boxed(b"mdat", &media)].concat()
}
pub fn jpeg(xmp: &str) -> Vec<u8> {
    let packet = [b"http://ns.adobe.com/xap/1.0/\0".as_slice(), xmp.as_bytes()].concat();
    [
        vec![255, 216, 255, 225],
        ((packet.len() + 2) as u16).to_be_bytes().to_vec(),
        packet,
        vec![255, 217],
    ]
    .concat()
}
pub fn xmp(video_len: usize, time: i64, extra: &str) -> String {
    format!(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" xmlns:Vendor="urn:independent:test" Vendor:opaque="retain &amp; verify" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1" Camera:MotionPhotoPresentationTimestampUs="{time}"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0"/ ></rdf:li>{extra}<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{video_len}"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#).replace("/ >","/>")
}
pub fn input(time: i64, tracks: usize) -> Vec<u8> {
    let video = movie(1000, tracks);
    [jpeg(&xmp(video.len(), time, "")), video].concat()
}

/// Synthetic HEIF/AVIF structure. Its item bytes are not a decodable picture.
pub fn heif(brand: &[u8; 4]) -> Vec<u8> {
    use liblivephoto_format::{isobmff::*, FourCC};
    let ftyp = boxed(b"ftyp", &[brand.as_slice(), &[0; 4], b"mif1"].concat());
    let image = b"synthetic-encoded-item";
    let info = make_iinf_box(
        1,
        &[make_infe_box(
            1,
            FourCC::new(if brand == b"avif" { *b"av01" } else { *b"hvc1" }),
            0,
        )
        .unwrap()],
    )
    .unwrap();
    let iloc = make_iloc_box(
        1,
        4,
        4,
        0,
        0,
        &[IlocEntry {
            item_id: 1,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: image.len() as u64,
            }],
        }],
    )
    .unwrap();
    let iprp = boxed(
        b"iprp",
        &[
            boxed(b"ipco", &make_ispe_box(2, 3).unwrap()),
            make_ipma_box(
                0,
                0,
                &[IpmaEntry {
                    item_id: 1,
                    associations: vec![IpmaAssociation {
                        property_index: 1,
                        essential: true,
                    }],
                }],
            )
            .unwrap(),
        ]
        .concat(),
    );
    let meta = full(
        b"meta",
        &[
            make_pitm_box(0, 1).unwrap(),
            info,
            iloc,
            iprp,
            boxed(b"idat", image),
        ]
        .concat(),
    );
    [ftyp, meta].concat()
}

pub fn metadata_movie(
    description: &[u8; 4],
    config: &[u8],
    samples: &[(&[u8], u32)],
    composition_offset: i32,
) -> Vec<u8> {
    metadata_movie_impl(description, config, samples, composition_offset, None)
}
pub fn overlapping_metadata_movie(
    description: &[u8; 4],
    config: &[u8],
    sample: &[u8],
    count: usize,
) -> Vec<u8> {
    let samples = vec![(sample, 1); count];
    let offsets = (0..count).map(|i| -(i as i32)).collect::<Vec<_>>();
    metadata_movie_impl(description, config, &samples, 0, Some(&offsets))
}
fn metadata_movie_impl(
    description: &[u8; 4],
    config: &[u8],
    samples: &[(&[u8], u32)],
    composition_offset: i32,
    offsets: Option<&[i32]>,
) -> Vec<u8> {
    let ftyp = boxed(b"ftyp", b"isom\0\0\0\0isom");
    let v = boxed(b"mdat", b"video");
    let m = boxed(
        b"mdat",
        &samples
            .iter()
            .flat_map(|s| s.0.iter().copied())
            .collect::<Vec<_>>(),
    );
    let duration: u32 = samples.iter().map(|s| s.1).sum();
    let mvhd = full(b"mvhd", &[words(&[0, 0, 1000, 2000]), vec![0; 80]].concat());
    let mut tkhd = words(&[0, 0, 2, 0, duration]);
    tkhd.extend_from_slice(&[0; 60]);
    let stsd = full(
        b"stsd",
        &[
            words(&[1]),
            boxed(
                description,
                &[vec![0, 0, 0, 0, 0, 0, 0, 1], config.to_vec()].concat(),
            ),
        ]
        .concat(),
    );
    let stts = full(
        b"stts",
        &[
            words(&[samples.len() as u32]),
            samples.iter().flat_map(|s| words(&[1, s.1])).collect(),
        ]
        .concat(),
    );
    let mut sizes = words(&[0, samples.len() as u32]);
    for s in samples {
        sizes.extend_from_slice(&(s.0.len() as u32).to_be_bytes());
    }
    let stsz = full(b"stsz", &sizes);
    let stsc = full(b"stsc", &words(&[1, 1, samples.len() as u32, 1]));
    let stco = full(b"stco", &words(&[1, (ftyp.len() + v.len() + 8) as u32]));
    let mut ctts = if let Some(offsets) = offsets {
        full(
            b"ctts",
            &[
                words(&[offsets.len() as u32]),
                offsets
                    .iter()
                    .flat_map(|o| words(&[1, *o as u32]))
                    .collect(),
            ]
            .concat(),
        )
    } else {
        full(
            b"ctts",
            &words(&[1, samples.len() as u32, composition_offset as u32]),
        )
    };
    ctts[8] = 1;
    let stbl = boxed(b"stbl", &[stsd, stts, stsz, stsc, stco, ctts].concat());
    let mdia = boxed(
        b"mdia",
        &[
            full(
                b"mdhd",
                &[words(&[0, 0, 1000, duration]), vec![0; 4]].concat(),
            ),
            full(
                b"hdlr",
                &[words(&[0]), b"meta".to_vec(), vec![0; 12]].concat(),
            ),
            boxed(b"minf", &stbl),
        ]
        .concat(),
    );
    let moov = boxed(
        b"moov",
        &[
            mvhd,
            track(1, 1000, (ftyp.len() + 8) as u32),
            boxed(b"trak", &[full(b"tkhd", &tkhd), mdia].concat()),
        ]
        .concat(),
    );
    [ftyp, v, m, moov].concat()
}
pub fn sef(records: &[(u16, &str, Vec<u8>)]) -> Vec<u8> {
    let mut d = vec![255, 216, 255, 217];
    let mut entries = Vec::new();
    for (kind, name, payload) in records {
        let a = d.len();
        let mut tag = vec![0, 0];
        tag.extend_from_slice(&kind.to_le_bytes());
        d.extend_from_slice(&tag);
        d.extend_from_slice(&(name.len() as u32).to_le_bytes());
        d.extend_from_slice(name.as_bytes());
        d.extend_from_slice(payload);
        entries.push((a, d.len() - a, tag));
    }
    let s = d.len();
    d.extend_from_slice(b"SEFH");
    d.extend_from_slice(&107u32.to_le_bytes());
    d.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (a, n, t) in entries {
        d.extend_from_slice(&t);
        d.extend_from_slice(&((s - a) as u32).to_le_bytes());
        d.extend_from_slice(&(n as u32).to_le_bytes());
    }
    let n = d.len() - s;
    d.extend_from_slice(&(n as u32).to_le_bytes());
    d.extend_from_slice(b"SEFT");
    d
}
