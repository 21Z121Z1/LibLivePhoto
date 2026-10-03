#![no_main]
use liblivephoto_format::*;
libfuzzer_sys::fuzz_target!(|data:&[u8]| {
    if let Ok(boxes)=isobmff::parse_boxes(data,0..data.len()) {
        for b in boxes {
            match b.kind.as_bytes() {
                b"meta"=>{let _=isobmff::parse_meta_box(data,&b);},
                b"iloc"=>{let _=isobmff::parse_iloc(data,&b);},
                b"iinf"=>{let _=isobmff::parse_iinf(data,&b);},
                b"iref"=>{let _=isobmff::parse_iref(data,&b);},
                _=>{},
            }
        }
    }
    let _=isobmff::scan_top_level_boxes(data);
    let _=jpeg_image_end(data,0);let _=jpeg_icc_profile(data);let _=jpeg_exif_tiff(data);
    let _=heif_exif_tiff(data);let _=exif_makernote(data);let _=exif_user_comment(data);
});
