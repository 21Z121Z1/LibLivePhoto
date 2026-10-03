#![no_main]
use liblivephoto::*;
libfuzzer_sys::fuzz_target!(|data:&[u8]| {
    for mode in [ParseMode::Strict,ParseMode::Compatible,ParseMode::Forensic] {
        let options=ParseOptions{mode,max_input_bytes:1_048_576,max_scan_bytes:1_048_576,max_candidates:16};
        if let Ok(p)=MotionPhoto::parse(Input::SingleFile(data),options) {
            assert!(p.validate().is_ok());
            for r in p.resources(){assert!(p.extract(r,std::io::sink()).is_ok());}
        }
        if data.len()>2 {
            let split=usize::from(u16::from_be_bytes([data[0],data[1]]))%(data.len()-2)+2;
            let _=MotionPhoto::parse(Input::ApplePair{still:&data[2..split],movie:&data[split..]},options);
        }
    }
});
