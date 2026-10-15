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
    let mut tkhd = words(&[0, 0, id, 0, scale * 2]);
    tkhd.extend_from_slice(&[0; 60]);
    let mdhd = full(
        b"mdhd",
        &[words(&[0, 0, scale, scale * 2]), vec![0; 4]].concat(),
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
    let stts = full(b"stts", &words(&[1, 1, scale * 2]));
    let stsc = full(b"stsc", &words(&[1, 1, 1, 1]));
    let stsz = full(b"stsz", &words(&[5, 1]));
    let stco = full(b"stco", &words(&[1, offset]));
    let stbl = boxed(b"stbl", &[stsd, stts, stsc, stsz, stco].concat());
    let mdia = boxed(b"mdia", &[mdhd, hdlr, boxed(b"minf", &stbl)].concat());
    boxed(b"trak", &[full(b"tkhd", &tkhd), mdia].concat())
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
