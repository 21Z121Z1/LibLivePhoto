#![no_main]
use liblivephoto::*;
libfuzzer_sys::fuzz_target!(|data:&[u8]| {
    let _=remux::optimize(data,remux::OptimizeOptions::default());
    let _=remux::optimize(data,remux::OptimizeOptions{compact_unreferenced_media:true});
    let _=remux::import_tracks(data,data,&[1],remux::OptimizeOptions::default());
    let options=ParseOptions{max_input_bytes:1_048_576,..ParseOptions::compatible()};
    if let Ok(p)=MotionPhoto::parse(Input::SingleFile(data),options) {
        let copy=p.plan(TargetProfile::PreserveSource,WritePolicy::default()).execute().unwrap();
        assert_eq!(copy.files[0].bytes,data);
        let ids=p.asset().containers.iter().filter(|c| c.resource==p.motion_video().id).flat_map(|c| &c.tracks).filter(|t| t.kind==TrackKind::Metadata).map(|t| t.id).collect::<Vec<_>>();
        let _=remux::remove_metadata_tracks(p.motion_video_bytes().unwrap(),&ids);
        for target in [TargetProfile::AndroidJpeg,TargetProfile::AndroidHeif,TargetProfile::AndroidAvif,TargetProfile::ApplePair,TargetProfile::SamsungJpegSef] {
            let policy=WritePolicy{unmapped:UnmappedPolicy::PreserveSidecars,pairing:PairingPolicy::Set("FUZZ".into())};
            let plan=p.plan(target,policy);
            if plan.can_execute(){let _=plan.execute();}
        }
    }
});
