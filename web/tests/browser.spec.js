import {test,expect} from '@playwright/test';
import fs from 'node:fs';import path from 'node:path';
const demo=path.resolve('public/demo');
async function ready(page,apple=false){await page.goto('/');await page.locator(apple?'#apple-demo':'#demo').click();await expect(page.locator('#work-status')).toContainText('解析完成');try {await expect(page.locator('#cover-time')).toBeEnabled();}catch(error){
 const state=await page.locator('#video').evaluate(v=>({readyState:v.readyState,networkState:v.networkState,error:v.error?{code:v.error.code,message:v.error.message}:null,h264Support:v.canPlayType('video/mp4; codecs="avc1.42E01E"'),currentTime:v.currentTime,duration:v.duration}));
 throw new Error(`${error.message}\nMedia readiness diagnostic: ${JSON.stringify(state)}; status: ${await page.locator('#preview-message').textContent()}`);
}}
test('real WASM parses and extracts; cover time and frame stepping match source samples',async({page},testInfo)=>{
 const writes=[];page.on('request',r=>{if(!['GET','HEAD'].includes(r.method()))writes.push(r.url());});
 await ready(page);await expect(page.locator('#photo-time')).toHaveText('812500 / 1000000 s');await expect(page.locator('#frame-times')).toContainText('48 帧');await expect(page.locator('#resources a')).toHaveCount(2);
 await page.locator('#cover-time').click();await expect.poll(()=>page.locator('#video').evaluate(v=>v.currentTime)).toBe(.8125);await expect(page.locator('#stage')).toHaveAttribute('data-frame-index','13');
 await page.locator('#next-frame').click();await expect.poll(()=>page.locator('#video').evaluate(v=>v.currentTime)).toBe(.875);
 await page.locator('#previous-frame').click();await expect.poll(()=>page.locator('#video').evaluate(v=>v.currentTime)).toBe(.8125);
 await page.locator('#frame-fallback').click();await expect(page.locator('#frame-cover')).toBeVisible();await expect(page.locator('#frame-cover')).toHaveAttribute('data-source-time','812500/1000000');
 // The project's JPEG is generated from frame 13. Compare actual browser-decoded
 // video pixels to it, allowing JPEG/H.264 losses; frame 0 has a different moon.
 const pixels=await page.evaluate(async()=>{const cover=new Image();cover.src='./demo/cover.jpg';await cover.decode();const c=document.createElement('canvas');c.width=720;c.height=480;const x=c.getContext('2d');x.drawImage(cover,0,0);const a=x.getImageData(0,0,720,480).data,b=document.getElementById('frame-cover').getContext('2d').getImageData(0,0,720,480).data;let sum=0;for(let i=0;i<a.length;i+=4)for(let k=0;k<3;k++)sum+=Math.abs(a[i+k]-b[i+k]);return sum/(720*480*3);});expect(pixels).toBeLessThan(5);
 await page.locator('#show-video').click();await page.locator('#play').click();await expect.poll(()=>page.locator('#video').evaluate(v=>!v.paused)).toBe(true);await page.locator('#stage').press('Space');await expect.poll(()=>page.locator('#video').evaluate(v=>v.paused)).toBe(true);await page.locator('#stage').press('m');await expect.poll(()=>page.locator('#video').evaluate(v=>v.muted)).toBe(false);expect(writes).toEqual([]);
 const download=page.waitForEvent('download');await page.locator('#resources a').nth(1).click();const d=await download;const p=await d.path();expect(fs.readFileSync(p).equals(fs.readFileSync(path.join(demo,'motion.mp4')))).toBe(true);
 await page.locator('#show-still').click();await page.setViewportSize({width:1440,height:1080});await page.screenshot({path:testInfo.outputPath('desktop.png'),fullPage:true});
 await testInfo.attach('timing-acceptance',{body:JSON.stringify({photoTime:'812500/1000000',coverFrame:13,nextFrameTime:.875,frameCount:48,meanPixelError:pixels,networkWrites:writes}),contentType:'application/json'});
});
test('Apple pair uses actual identifier and rational time; mismatched pair is rejected',async({page})=>{
 await ready(page,true);await expect(page.locator('#format')).toHaveText('AppleLivePhoto');await expect(page.locator('#association')).toContainText('LIBLIVEPHOTO-SYNTHETIC-DEMO');await expect(page.locator('#photo-time')).toHaveText('1625 / 2000 s');
 const bad=Buffer.from(fs.readFileSync(path.join(demo,'apple-paired.mov')));const id=bad.indexOf('LIBLIVEPHOTO-SYNTHETIC-DEMO-20261019');expect(id).toBeGreaterThan(0);bad[id]=0x78;
 await page.locator('#files').setInputFiles([{name:'apple-still.jpg',mimeType:'image/jpeg',buffer:fs.readFileSync(path.join(demo,'apple-still.jpg'))},{name:'wrong.mov',mimeType:'video/quicktime',buffer:bad}]);await expect(page.locator('#work-status')).toContainText('identifierMismatch');
});
test('ordinary pair requires explicit association; invalid input and codec failure are visible',async({page})=>{
 await page.goto('/');await page.locator('#files').setInputFiles([path.join(demo,'cover.jpg'),path.join(demo,'motion.mp4')]);await expect(page.locator('#work-status')).toContainText('missingPairMetadata');
 await page.locator('#declared').check();await page.locator('#files').setInputFiles([]);await page.locator('#files').setInputFiles([path.join(demo,'cover.jpg'),path.join(demo,'motion.mp4')]);await expect(page.locator('#work-status')).toContainText('解析完成');await expect(page.locator('#photo-time')).toContainText('未知');await expect(page.locator('#cover-time')).toBeDisabled();
 await page.locator('#files').setInputFiles({name:'bad.jpg',mimeType:'image/jpeg',buffer:Buffer.from('not a photo')});await expect(page.locator('#work-status')).toHaveClass('error');
 await ready(page);await page.locator('#video').evaluate(v=>v.dispatchEvent(new Event('error')));await expect(page.locator('#preview-message')).toContainText('没有隐式转码');await expect(page.locator('#play')).toBeDisabled();
});
test('mobile layout, keyboard controls and cancellation leave local state consistent',async({page},testInfo)=>{
 await page.setViewportSize({width:390,height:844});await ready(page);expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
 await page.locator('#stage').focus();await page.keyboard.press('Space');await expect.poll(()=>page.locator('#video').evaluate(v=>!v.paused)).toBe(true);await page.keyboard.press('Space');
 await page.locator('#show-still').click();await page.screenshot({path:testInfo.outputPath('mobile.png'),fullPage:true});
 // Queue a large local read and cancel immediately. No previous result may
 // replace the next operation when the worker/reader finishes late.
 await page.locator('#files').setInputFiles({name:'pending.jpg',mimeType:'image/jpeg',buffer:Buffer.alloc(16*1024*1024)});await page.locator('#cancel').click();await expect(page.locator('#work-status')).toContainText('已取消');await ready(page);await expect(page.locator('#frame-times')).toContainText('48 帧');
});
