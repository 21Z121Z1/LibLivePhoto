#![no_main]
use liblivephoto::*;
libfuzzer_sys::fuzz_target!(|data:&[u8]| {
    let _=inspect_honor_metadata(data);
    let _=inspect_oplus_metadata(data);
    let _=decode_agtm_header(data);
    let _=decode_agtm(data);
    let _=decode_video_metadata(data);
    let _=validate_heif_color_information(data);
    let _=inspect_video_sample_descriptions(data);
    let _=validate_google_video_container(data,GoogleVideoTracks{primary:1,secondary:Some(2)});
    if let Ok(scores)=extract_video_metadata(data,PhotoTimeExpectation::Unchecked) {
        for sample in scores.iter().take(8) {
            let _=project_scoring_media_times(data,&sample.metadata,ScoringMediaTracks{primary:1,high_resolution:Some(1)},4096);
        }
    }
    let _=extract_agtm_from_video(data,MediaTime::new(0,1).unwrap());
    let _=XmpDocument::parse(data);
    let _=MetadataCollection::from_json(data);
    let _=MetadataEngine::new().parse(data,&HandlerOptions::default());
    let _=inspect_samsung_metadata(data);
    for mode in [ParseMode::Strict,ParseMode::Compatible,ParseMode::Forensic] {
        let options=ParseOptions{mode,max_input_bytes:1_048_576,max_scan_bytes:1_048_576,max_candidates:16};
        if let Ok(p)=MotionPhoto::parse(Input::SingleFile(data),options) {
            assert!(p.validate().is_ok());
            for r in p.resources(){assert!(p.extract(r,std::io::sink()).is_ok());}
            let _=p.still_metadata();
            let _=p.sample_descriptions();
            for track in p.asset().containers.iter().flat_map(|c|&c.tracks).filter(|t|t.kind==TrackKind::Video){let _=p.frame_times(track.id,4096);}
            let _=p.agtm_at(MediaTime::new(0,1).unwrap());
        }
        if data.len()>2 {
            let split=usize::from(u16::from_be_bytes([data[0],data[1]]))%(data.len()-2)+2;
            let _=MotionPhoto::parse(Input::ApplePair{still:&data[2..split],movie:&data[split..]},options);
        }
    }
});
