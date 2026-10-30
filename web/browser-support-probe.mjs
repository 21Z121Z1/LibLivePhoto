// Capability probe, separate from the required Chrome acceptance tests. Records
// the original bundled-browser readiness failure without masking test outcomes.
import {chromium} from 'playwright';
import {mkdir,writeFile} from 'node:fs/promises';
const executablePath=process.env.BROWSER_EXECUTABLE;
const browser=await chromium.launch({headless:true,...(executablePath?{executablePath,args:['--no-sandbox']}: {})});
try {
 const page=await browser.newPage();await page.goto('http://127.0.0.1:4173');await page.locator('#demo').click();await page.locator('#work-status').filter({hasText:'解析完成'}).waitFor();
 await page.waitForFunction(()=>{const v=document.getElementById('video');return v.error||v.readyState>=2;});
 const state=await page.locator('#video').evaluate(v=>({userAgent:navigator.userAgent,readyState:v.readyState,networkState:v.networkState,error:v.error?{code:v.error.code,message:v.error.message}:null,h264Support:v.canPlayType('video/mp4; codecs="avc1.42E01E"'),currentTime:v.currentTime,duration:Number.isFinite(v.duration)?v.duration:null,photoTime:document.getElementById('photo-time').textContent,frameCount:document.getElementById('stage').dataset.frameCount,coverTimeEnabled:!document.getElementById('cover-time').disabled,message:document.getElementById('preview-message').textContent}));
 state.browser=await browser.version();state.executable=executablePath??'bundled-chromium';await mkdir('test-results',{recursive:true});await writeFile(`test-results/${executablePath?'chrome':'bundled'}-readiness.json`,JSON.stringify(state,null,2)+'\n');console.log('MEDIA_READINESS '+JSON.stringify(state));
}finally{await browser.close();}
