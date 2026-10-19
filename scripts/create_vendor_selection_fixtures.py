#!/usr/bin/env python3
"""Separate owned selection vectors; preserve the existing vendor13 unchanged."""
import argparse
import hashlib
import json
from pathlib import Path
from create_vendor_fixtures import sef

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True)
    out=parser.parse_args().out;out.mkdir(parents=True,exist_ok=False)
    data={}
    def save(name,body):
        with (out/name).open('xb') as stream:stream.write(body)
        data[name]=body
    a=(ROOT/'web/public/demo/motion.mp4').read_bytes()
    b=(ROOT/'fixtures/bindings/agtm-video.mp4').read_bytes()
    assert a!=b
    save('movie-a.mp4',a);save('movie-b.mp4',b)
    unknown=(0x1234,'Unspecified',b'opaque\0unknown')
    autoplay=(0x0a33,'MotionPhoto_AutoPlay',a)
    primary=(0x0a30,'MotionPhoto_Data',b)
    save('distinct-normal.jpg',sef([autoplay,unknown,primary]))
    save('distinct-no-primary.jpg',sef([autoplay,unknown,(0x4321,'UnspecifiedVideo',b)]))
    save('distinct-ambiguous.jpg',sef([autoplay,unknown,primary,(0x0a30,'MotionPhoto_Data',a)]))
    save('distinct-bad-pointer.jpg',sef([autoplay,unknown,(0x0a30,'MotionPhoto_Data',b'mpv2'+b'\xff'*4+(10).to_bytes(4,'big'))]))
    # Two declared primary pointers alias the SAME physical complete movie.
    prefix=b'\xff\xd8\xff\xd9'+a
    pointer=b'mpv2'+(4).to_bytes(4,'big')+len(a).to_bytes(4,'big')
    tail=sef([(0x0a30,'MotionPhoto_Data',pointer),(0x0a30,'MotionPhoto_Data',pointer)])[4:]
    save('pointer-alias.jpg',prefix+tail)
    cases=[]
    def case(name,source,index=None,expected=None,success=True,**options):
        request={'action':'extract','still':False,'video':True,'xmp':False}
        if index is not None:request['samsungRecordIndex']=index
        request.update(options)
        facts=[]
        if success:
            facts=[{'path':['metadata','samsungRecordIndex'],'value':int(index)},
                   {'path':['metadata','selection'],'value':'explicit-record-index'},
                   {'path':['metadata','encodedBytesUnchanged'],'value':True},
                   {'path':['metadata','role'],'value':'uninterpreted'}]
        cases.append({'id':name,'request':request,'primary':source,'success':success,'expect':facts,
                      'files':[{'mime':'video/mp4','fixture':expected}] if expected else []})
    for source,indices in [('distinct-normal.jpg',[(0,'movie-a.mp4'),(2,'movie-b.mp4')]),
                           ('distinct-no-primary.jpg',[(0,'movie-a.mp4'),(2,'movie-b.mp4')]),
                           ('distinct-ambiguous.jpg',[(2,'movie-b.mp4'),(3,'movie-a.mp4')]),
                           ('distinct-bad-pointer.jpg',[(0,'movie-a.mp4')]),
                           ('pointer-alias.jpg',[(0,'movie-a.mp4'),(1,'movie-a.mp4')])]:
        for index,expected in indices:case(source[:-4]+'-record-'+str(index),source,str(index),expected)
    case('bad-pointer-selection-rejected','distinct-bad-pointer.jpg','2',success=False)
    case('unknown-record-selection-rejected','distinct-normal.jpg','1',success=False)
    case('missing-record-selection-rejected','distinct-normal.jpg','4096',success=False)
    case('negative-selection-rejected','distinct-normal.jpg','-1',success=False)
    case('selection-overflow-rejected','distinct-normal.jpg','18446744073709551616',success=False)
    case('selection-provider-disabled','distinct-normal.jpg','0',success=False,disableThirdParty=True)
    case('ambiguous-default-extract-still-rejected','distinct-ambiguous.jpg',success=False)
    case('pointer-alias-default-extract-still-rejected','pointer-alias.jpg',success=False)
    case('selection-conflicting-still-flag-rejected','distinct-normal.jpg','0',success=False,still=True)
    case('selection-strict-parser-flag-rejected','distinct-normal.jpg','0',success=False,strict=True)
    # Source ranges must name the exact original payload, including aliases.
    for c in cases:
        if c['success']:
            payload=data[c['files'][0]['fixture']];source=data[c['primary']]
            at=source.rindex(payload) if c['primary']=='distinct-ambiguous.jpg' and c['request']['samsungRecordIndex']=='3' else source.index(payload)
            c['expect'].append({'path':['metadata','extent'],'value':{'start':str(at),'end':str(at+len(payload))}})
    vivo=(ROOT/'fixtures/vendor/vivo-alias.jpg').read_bytes();save('vivo-alias.jpg',vivo)
    marker=vivo.index(b'cameralbum!');length=int.from_bytes(vivo[marker-4:marker],'big');start=marker-4-length
    assert vivo[start-20:start]==b'vivoMediaExtInfovivo'
    facts={'sourceIndex':0,'extents':[{'start':str(start-20),'end':str(len(vivo))}],
           'jsonExtents':[{'start':str(start),'end':str(marker-4)}],
           'identifierExtents':[{'start':str(marker+15),'end':str(marker+43)}],
           'origins':['TerminalSource','MotionVideoUuid'],'rawJson':vivo[start:marker-4].decode()}
    cases.append({'id':'vivo-alias-source-ranges','request':{'action':'metadata'},'primary':'vivo-alias.jpg','success':True,
                  'expect':[{'path':['metadata','decoded',0,'value','declarations','footers',0,key],'value':value} for key,value in facts.items()], 'files':[]})
    assert len(cases)==20
    manifest={'scope':'independently authored read-only explicit SEF selection, distinct bytes and source aliases; no native role or writer',
              'cases':cases,'sha256':{name:hashlib.sha256(body).hexdigest() for name,body in data.items()}}
    with (out/'cases.json').open('x',encoding='utf-8',newline='\n') as stream:json.dump(manifest,stream,indent=2);stream.write('\n')
    print('selection cases',len(cases),'manifest SHA256',hashlib.sha256((out/'cases.json').read_bytes()).hexdigest())


if __name__=='__main__':main()
