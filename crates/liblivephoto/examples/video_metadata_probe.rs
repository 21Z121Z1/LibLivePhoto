use liblivephoto::*;
use serde_json::json;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(std::env::args().nth(1).ok_or("metadata file required")?)?;
    let m = decode_video_metadata(&bytes)?;
    let frame = |f: &FrameScore| json!({"scoreBits":f.encoded_score,"timeUs":f.timestamp_us});
    println!(
        "{}",
        json!({"success":true,"model":m.model_version,"primary":m.primary_score.as_ref().map(frame),"frames":m.high_resolution_scores.as_ref().map(|v|v.iter().map(frame).collect::<Vec<_>>()),"lowStabilized":m.low_resolution_stabilized(),"highStabilized":m.high_resolution_stabilized()})
    );
    Ok(())
}
