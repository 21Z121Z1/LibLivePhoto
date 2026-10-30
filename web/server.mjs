// Local static files only: no upload/conversion endpoint, no public bind.
import http from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root=path.join(path.dirname(fileURLToPath(import.meta.url)),'dist');
const mime={'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.mjs':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.wasm':'application/wasm','.jpg':'image/jpeg','.mp4':'video/mp4','.mov':'video/quicktime','.json':'application/json'};
http.createServer(async(req,res)=>{
  res.setHeader('Content-Security-Policy',"default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self' blob:; img-src 'self' blob: data:; media-src 'self' blob:; style-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'");
  res.setHeader('X-Content-Type-Options','nosniff');
  if(!['GET','HEAD'].includes(req.method)){res.writeHead(405);res.end('Static local preview: uploads are not supported');return;}
  try {
    const name=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    const file=path.resolve(root,`.${name==='/'?'/index.html':name}`);
    if(!file.startsWith(root+path.sep)){res.writeHead(403);res.end();return;}
    const meta=await stat(file);if(!meta.isFile())throw new Error('not a file');
    res.setHeader('Content-Type',mime[path.extname(file)]??'application/octet-stream');
    const bytes=await readFile(file);res.writeHead(200,{'Content-Length':bytes.length});res.end(req.method==='HEAD'?undefined:bytes);
  }catch{res.writeHead(404);res.end('File not found; run npm run build first');}
}).listen(Number(process.env.PORT??4173),'127.0.0.1',()=>console.log(`Local lab: http://127.0.0.1:${process.env.PORT??4173}`));
