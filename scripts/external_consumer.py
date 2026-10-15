#!/usr/bin/env python3
"""Build and execute a minimal consumer outside both source workspaces."""
import json
from pathlib import Path
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent
SOURCE='''use liblivephoto::{Input,MotionPhoto,ParseOptions};
fn main()->Result<(),Box<dyn std::error::Error>> {
 let path=std::env::args().nth(1).ok_or("fixture required")?;
 let bytes=std::fs::read(path)?;
 let model=MotionPhoto::probe(Input::SingleFile(&bytes),ParseOptions::strict())?;
 let parsed=MotionPhoto::parse(Input::SingleFile(&bytes),ParseOptions::strict())?;
 assert_eq!(model,parsed.asset().clone());
 let mut extracted=Vec::new();parsed.extract(parsed.motion_video(),&mut extracted)?;
 assert_eq!(extracted,parsed.motion_video_bytes()?);
 parsed.validate()?;
 println!("external consumer: probe, parse, extract, validate passed");Ok(())
}
'''

def main():
    with tempfile.TemporaryDirectory(prefix="liblivephoto-consumer-") as tmp:
        base=Path(tmp);consumer=base/"consumer";consumer.mkdir();(consumer/"src").mkdir()
        library=json.dumps((ROOT/"crates/liblivephoto").as_posix())
        manifest='[package]\nname="external-consumer"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\nliblivephoto={path='+library+'}\n'
        (consumer/"Cargo.toml").write_text(manifest);(consumer/"src/main.rs").write_text(SOURCE)
        (consumer/"src/bin").mkdir()
        (consumer/"src/bin/convert.rs").write_text((ROOT/"crates/liblivephoto/examples/convert.rs").read_text())
        seeds=base/"seeds"
        subprocess.run(["cargo","run","--quiet","-p","liblivephoto","--example","fuzz_seeds","--",str(seeds)],cwd=ROOT,check=True)
        subprocess.run(["cargo","run","--quiet","--manifest-path",str(consumer/"Cargo.toml"),"--bin","external-consumer","--",str(seeds/"asset_input/single-track")],cwd=consumer,check=True)
        def convert(*args):
            subprocess.run(["cargo","run","--quiet","--manifest-path",str(consumer/"Cargo.toml"),"--bin","convert","--",*map(str,args)],cwd=consumer,check=True,stdout=subprocess.DEVNULL)
        first=base/"apple"; android=base/"android"; returned=base/"returned"
        convert("apple",first,seeds/"asset_input/single-track","--pair-id","EXTERNAL-CONSUMER","--sidecars")
        convert("google-jpeg",android,first/"resource-0.jpg",first/"resource-1.mov","--sidecars")
        convert("apple",returned,android/"resource-0.jpg","--pair-id","EXTERNAL-RETURN","--sidecars")
        assert (returned/"resource-0.jpg").is_file() and (returned/"resource-1.mov").is_file()
        print("external consumer: Android -> Apple -> Android -> Apple conversion passed")
        lock=(consumer/"Cargo.lock").read_text()
        if 'name = "xdremux' in lock:raise RuntimeError("external consumer depends on XDRemux")
        print("external workspace dependency graph contains no XDRemux package")

if __name__=="__main__":main()
