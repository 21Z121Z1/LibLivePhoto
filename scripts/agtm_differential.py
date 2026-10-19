#!/usr/bin/env python3
"""Same-input complete AGTM comparisons; synthetic payloads only, no image corpus."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SHA = "12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3"
def run(args, **kwargs):
    return subprocess.check_output(list(map(str,args)),text=True,**kwargs)
def words(values):
    return b"".join(int(v).to_bytes(2,"big") for v in values)
def close(a,b):
    if isinstance(a,dict):
        return a.keys()==b.keys() and all(close(a[k],b[k]) for k in a)
    if isinstance(a,list):
        return len(a)==len(b) and all(close(x,y) for x,y in zip(a,b))
    if isinstance(a,(int,float)) and not isinstance(a,bool):
        return math.isclose(a,b,rel_tol=1e-5,abs_tol=1e-6)
    return a==b
def vectors():
    yield "baseline",bytes([0,0])
    yield "custom-white",bytes([0,0x80])+words([1015])
    yield "reference-white",bytes([0,0x40])+words([10000])+bytes([0x80])
    for mode in range(3):
        yield f"zero-rules-mode-{mode}",bytes([0,0x40])+words([10000])+bytes([mode<<2])
    for preset in range(3):
        yield f"preset-mix-{preset}",bytes([0,0x40])+words([10000])+bytes([0x10])+words([20000])+bytes([preset<<6,0x0c])+words([0,1000,0,10000])
    shared=bytes([0,0x40])+words([10000])+bytes([0x2f])+words([35400,14600,8500,39850,6550,2300,15635,16450])
    shared+=words([20000])+bytes([0xe9])+words([100,2000,65535])+bytes([8])+words([0,1000,0,10000,18000,27000])+words([5000,1000,2000,18000,9000])
    yield "custom-shared-two-rules",shared
    yield "wrapped-t35",bytes([5,0xb5,0,0x90,0,1])+shared
    many=bytes([0,0x40])+words([10000])+bytes([0x40])
    for index in range(4):
        many+=words([20000+index*1000])+bytes([(index%3)<<6,0xfc])+words([i*1000 for i in range(32)])+words([i*500 for i in range(32)])
    yield "four-rules-32-points",many

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--reference-source",type=Path,required=True)
    args=parser.parse_args(); source=args.reference_source.resolve()
    assert run(["git","-C",source,"rev-parse","HEAD"]).strip()==SHA
    parent=ROOT/"evidence/local";parent.mkdir(exist_ok=True)
    output=Path(tempfile.mkdtemp(prefix="agtm-diff-",dir=parent)); reference=output/"reference"
    run(["c++","-std=c++17","-include","limits","-I"+str(source/"lib/core/motion_photo/includes"),ROOT/"scripts/agtm_reference_probe.cc",source/"lib/core/motion_photo/src/motion_photo/agtm.cc","-o",reference])
    subprocess.run(["cargo","build","--locked","-p","liblivephoto","--example","agtm_decode"],cwd=ROOT,check=True)
    rust=ROOT/"target/debug/examples/agtm_decode";rows=[]
    for name,payload in vectors():
        path=output/(name+".bin");path.write_bytes(payload)
        ref=json.loads(run([reference,path]));actual=json.loads(run([rust,path]))
        assert ref["success"] and actual["success"],(name,ref,actual)
        assert close(ref["reference"],actual["reference"]),(name,ref,actual)
        expected=ref.get("rules",[]);rules=actual["details"]["rules"]
        assert len(expected)==len(rules),(name,expected,rules)
        for a,b in zip(expected,rules):
            assert close(a["alternateHdrHeadroom"],b["alternateHdrHeadroom"])
            assert a["usePchipSlope"]==b["usePchipSlope"]
            assert len(a["curve"])==len(b["curve"])
            for x,y in zip(a["curve"],b["curve"]):
                assert close(x["x"],y["x"]) and close(x["y"],y["y"])
                if not a["usePchipSlope"]: assert close(x["slope"],y["slope"])
                else: assert y["slope"] is None
        if actual["details"]["colorSpaceMode"]==3:
            assert close(ref["chromaticities"],actual["details"]["customChromaticities"])
        rows.append({"name":name,"sha256":hashlib.sha256(payload).hexdigest(),"bytes":len(payload),"rules":len(rules),"reference_fields_equal":True})
        print(name,"passed",flush=True)
    receipt={"reference_sha":SHA,"implementation_base_sha":run(["git","-C",ROOT,"rev-parse","HEAD"]).strip(),"cases":rows,"differences":["Google does not assign component-mix values; Rust preserves their encoded coefficients without inventing weights.","Google returns partial/default results for truncated curves; Rust rejects truncation.","Google JSON projects one rule; Rust retains every rule and exposes that limited projection separately."]}
    (output/"receipt.json").write_text(json.dumps(receipt,indent=2)+"\n")
    print("receipt:",output/"receipt.json")

if __name__=="__main__":main()
