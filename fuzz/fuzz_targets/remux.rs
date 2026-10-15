#![no_main]
use liblivephoto::*;
libfuzzer_sys::fuzz_target!(|data:&[u8]| {
    let options=ParseOptions{max_input_bytes:1_048_576,..ParseOptions::compatible()};
    if let Ok(p)=MotionPhoto::parse(Input::SingleFile(data),options) {
        let copy=p.plan(TargetProfile::PreserveSource,WritePolicy::default()).execute().unwrap();
        assert_eq!(copy.files[0].bytes,data);
        for target in [TargetProfile::AndroidJpeg,TargetProfile::AndroidHeif,TargetProfile::ApplePair,TargetProfile::SamsungJpegSef] {
            let policy=WritePolicy{unmapped:UnmappedPolicy::PreserveSidecars,pairing:PairingPolicy::Set("FUZZ".into())};
            let plan=p.plan(target,policy);
            if plan.can_execute(){let _=plan.execute();}
        }
    }
});
