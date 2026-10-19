import init, {inspect_motion,inspect_pair} from './pkg/liblivephoto_bridge.js';
self.onmessage=async({data})=>{
  try {
    self.postMessage({phase:'parse'});await init();
    const bytes=data.buffers.map(b=>new Uint8Array(b));
    const inspection=bytes.length===1?inspect_motion(bytes[0]):inspect_pair(bytes[0],bytes[1],data.declared,data.timeValue,data.timescale);
    try {
      const manifest=JSON.parse(inspection.manifest()), still=inspection.still_bytes(), movie=inspection.movie_bytes();
      self.postMessage({phase:'complete',manifest,still,movie},[still.buffer,movie.buffer]);
    }finally{inspection.free();}
  }catch(error){let details;try{details=JSON.parse(String(error));}catch{details={code:'parseFailed',message:String(error)};}self.postMessage({phase:'error',error:details});}
};
