#!/usr/bin/env python3
"""Same-input Google optimizer/delete/copy checks, no Moments authoring.

Compare bytes and rational media timing independently. Reference differences
are recorded rather than silently treating Google output as normative.
"""
import argparse,hashlib,json,shlex,subprocess,tempfile
from pathlib import Path
from google_differential import packets
ROOT=Path(__file__).resolve().parents[1];SHA='12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3'
def run(args,**kw):
    r=subprocess.run(list(map(str,args)),text=True,capture_output=True,**kw)
    if r.returncode:raise RuntimeError(r.stderr or r.stdout)
    return r.stdout
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def coded(values):return [[s[0],s[1],[p[0] for p in s[2]]] for s in values]
def compare(expected,actual):return {'coded_packets_equal':coded(expected)==coded(actual),'rational_pts_dts_durations_equal':expected==actual}
def main():
    p=argparse.ArgumentParser();p.add_argument('--reference-source',type=Path,required=True);p.add_argument('--reference-build',type=Path,required=True);a=p.parse_args()
    source=a.reference_source.resolve();assert run(['git','-C',source,'rev-parse','HEAD']).strip()==SHA
    run(['git','-C',source,'diff','--exit-code'])
    out=Path(tempfile.mkdtemp(prefix='mp4-diff-',dir=ROOT/'evidence/local'));build=a.reference_build.resolve()/'lib/core'
    link=shlex.split((build/'CMakeFiles/motion_photo_tests.dir/link.txt').read_text());ref=out/'reference'
    run(['c++','-std=c++17','-I'+str(source/'lib/core/motion_photo/includes'),'-I'+str(source/'lib/core/third_party/image_io/includes'),'-I'+str(a.reference_build.resolve()/'_deps/mp4v2-src/include'),ROOT/'scripts/mp4_reference_probe.cc','-o',ref,*link[link.index('-o')+2:]],cwd=build)
    run(['cargo','build','--locked','-p','liblivephoto','--example','remux_probe'],cwd=ROOT);probe=ROOT/'target/debug/examples/remux_probe'
    def google(args):return json.JSONDecoder().raw_decode(run([ref,*args]).split('MP4_REFERENCE_JSON\t')[1])[0]
    rows=[]
    for name,src in [('avc-bframes',ROOT/'web/public/demo/motion.mp4'),('avc-aac',ROOT/'web/public/acceptance/audio.mp4'),('apple-mebx',ROOT/'web/public/demo/apple-paired.mov'),('hvc1-real',ROOT/'web/public/acceptance/hevc.mp4')]:
        expected=packets(src);rust=out/(name+'-rust.mp4');report=json.loads(run([probe,'optimize',src,rust]));assert packets(rust)==expected
        support=google(['support',src]);entry={'case':name,'input_sha256':digest(src),'support_query':support,'rust':compare(expected,packets(rust)),'rust_report':report}
        if support['optimizationSupported']:
            dst=out/(name+'-reference.mp4');entry['google']=google(['primary-optimize',src,dst]);assert entry['google']['success']
            assert not any(t['handler']=='meta' for t in entry['google']['outputTracks'])
            entry['google_comparison']=compare(expected,packets(dst));entry['google_output_sha256']=digest(dst)
        else:
            assert name=='hvc1-real';entry['google_optimization']='explicit hvc1 public support-query rejection; optimization not invoked'
        rows.append(entry)
    # Read existing metadata, then explicitly remove its track. No construction.
    src=ROOT/'web/public/demo/apple-paired.mov';expected=packets(src);rust=out/'strip-rust.mov';google_out=out/'strip-google.mov'
    report=json.loads(run([probe,'remove',src,rust,'2']));reference=google(['strip',src,google_out]);assert reference['success'];assert packets(rust)==expected
    assert len(reference['inputTracks'])==2 and len(reference['outputTracks'])==1
    rows.append({'case':'strip-existing-apple-metadata','google':reference,'google_comparison':compare(expected,packets(google_out)),'rust':compare(expected,packets(rust)),'rust_report':report})
    # Import the exact primitive Google calls internally, keeping its public
    # exposure distinct from general-purpose LibLivePhoto track import.
    src=ROOT/'web/public/demo/motion.mp4';rust=out/'copy-rust.mp4';google_out=out/'copy-google.mp4';expected=packets(src)*2
    report=json.loads(run([probe,'import',src,rust,src,'1']));reference=google(['copy-track',src,google_out,src,'1']);assert reference['success'];assert packets(rust)==expected
    rows.append({'case':'copy-existing-video-track','google':reference,'google_comparison':compare(expected,packets(google_out)),'rust':compare(expected,packets(rust)),'rust_report':report})
    receipt={'reference_sha':SHA,'implementation_sha':run(['git','-C',ROOT,'rev-parse','HEAD']).strip(),'working_tree_diff_sha256':hashlib.sha256(run(['git','-C',ROOT,'diff','HEAD']).encode()).hexdigest(),'cases':rows,'limits':['Google builder primary-track path deletes metadata; Rust optimizer retains it. Removal requires explicit track selection.','Google MP4CopyTrack is audited as an internal primitive; this does not claim Google exposes arbitrary track import through its facade.','Fragmented/indexed/external/unknown reference layouts are rejected by the bounded Rust API.','No metadata track or Moments construction is performed.']}
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(out/'receipt.json')
if __name__=='__main__':main()
