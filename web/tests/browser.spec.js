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
 // Observe the compositor's actual presented frame, independently of the
 // application's rational-time/frame-index calculations. First leave frame 13.
 await page.locator('#show-video').click();
 await page.locator('#video').evaluate(v=>new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(new Error('frame 24 not presented')),5000);function frame(_now,m){if(m.mediaTime===1.5){clearTimeout(timeout);resolve();}else v.requestVideoFrameCallback(frame);}v.requestVideoFrameCallback(frame);v.currentTime=1.5;}));
 await page.locator('#video').evaluate(v=>{window.presentedPhotoFrame=new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(new Error('photo frame not presented')),5000);function frame(_now,m){if(m.mediaTime===.8125){clearTimeout(timeout);resolve({mediaTime:m.mediaTime,presentedFrames:m.presentedFrames,width:m.width,height:m.height});}else v.requestVideoFrameCallback(frame);}v.requestVideoFrameCallback(frame);});});
 await page.locator('#cover-time').click();const presented=await page.evaluate(()=>window.presentedPhotoFrame);expect(presented.mediaTime).toBe(.8125);await expect.poll(()=>page.locator('#video').evaluate(v=>v.currentTime)).toBe(.8125);await expect(page.locator('#stage')).toHaveAttribute('data-frame-index','13');
 await page.locator('#next-frame').click();await expect.poll(()=>page.locator('#video').evaluate(v=>v.currentTime)).toBe(.875);
 await page.locator('#previous-frame').click();await expect.poll(()=>page.locator('#video').evaluate(v=>v.currentTime)).toBe(.8125);
 await page.locator('#frame-fallback').click();await expect(page.locator('#frame-cover')).toBeVisible();await expect(page.locator('#frame-cover')).toHaveAttribute('data-source-time','812500/1000000');
 // The project's JPEG is generated from frame 13. Compare actual browser-decoded
 // video pixels to it, allowing JPEG/H.264 losses; frame 0 has a different moon.
 const pixels=await page.evaluate(async()=>{const cover=new Image();cover.src='./demo/cover.jpg';await cover.decode();const c=document.createElement('canvas');c.width=720;c.height=480;const x=c.getContext('2d');x.drawImage(cover,0,0);const a=x.getImageData(0,0,720,480).data,b=document.getElementById('frame-cover').getContext('2d').getImageData(0,0,720,480).data;let sum=0;for(let i=0;i<a.length;i+=4)for(let k=0;k<3;k++)sum+=Math.abs(a[i+k]-b[i+k]);return sum/(720*480*3);});expect(pixels).toBeLessThan(5);
 await page.locator('#show-video').click();await page.locator('#play').click();await expect.poll(()=>page.locator('#video').evaluate(v=>!v.paused)).toBe(true);await page.locator('#stage').press('Space');await expect.poll(()=>page.locator('#video').evaluate(v=>v.paused)).toBe(true);await page.locator('#stage').press('m');await expect.poll(()=>page.locator('#video').evaluate(v=>v.muted)).toBe(false);expect(writes).toEqual([]);
 const download=page.waitForEvent('download');await page.locator('#resources a').nth(1).click();const d=await download;const p=await d.path();expect(fs.readFileSync(p).equals(fs.readFileSync(path.join(demo,'motion.mp4')))).toBe(true);
 await page.locator('#show-still').click();await page.setViewportSize({width:1440,height:1080});await page.screenshot({path:testInfo.outputPath('desktop.png'),fullPage:true});
 await testInfo.attach('timing-acceptance',{body:JSON.stringify({photoTime:'812500/1000000',coverFrame:13,actuallyPresented:presented,nextFrameTime:.875,frameCount:48,meanPixelError:pixels,networkWrites:writes}),contentType:'application/json'});
});
test('Apple pair uses actual identifier and rational time; mismatched pair is rejected',async({page})=>{
 await ready(page,true);await expect(page.locator('#format')).toHaveText('AppleLivePhoto');await expect(page.locator('#association')).toContainText('LIBLIVEPHOTO-SYNTHETIC-DEMO');await expect(page.locator('#photo-time')).toHaveText('1625 / 2000 s');
 const bad=Buffer.from(fs.readFileSync(path.join(demo,'apple-paired.mov')));const id=bad.indexOf('LIBLIVEPHOTO-SYNTHETIC-DEMO-20261019');expect(id).toBeGreaterThan(0);bad[id]=0x78;
 await page.locator('#files').setInputFiles([{name:'apple-still.jpg',mimeType:'image/jpeg',buffer:fs.readFileSync(path.join(demo,'apple-still.jpg'))},{name:'wrong.mov',mimeType:'video/quicktime',buffer:bad}]);await expect(page.locator('#work-status')).toContainText('identifierMismatch');
});
test('ordinary pair can reparse the same files with explicit association and exact time',async({page})=>{
 await page.goto('/');await page.locator('#files').setInputFiles([path.join(demo,'cover.jpg'),path.join(demo,'motion.mp4')]);await expect(page.locator('#work-status')).toContainText('missingPairMetadata');
 await page.locator('#declared').check();await page.locator('#reparse').click();await expect(page.locator('#work-status')).toContainText('解析完成');await expect(page.locator('#photo-time')).toContainText('未知');await expect(page.locator('#cover-time')).toBeDisabled();
 await page.locator('#time-value').fill('13');await page.locator('#time-scale').fill('16');await page.locator('#reparse').click();await expect(page.locator('#photo-time')).toHaveText('13 / 16 s');await expect(page.locator('#cover-time')).toBeEnabled();
 await page.locator('#files').setInputFiles({name:'bad.jpg',mimeType:'image/jpeg',buffer:Buffer.from('not a photo')});await expect(page.locator('#work-status')).toHaveClass('error');

});
test('mobile layout, keyboard controls and cancellation leave local state consistent',async({page},testInfo)=>{
 await page.setViewportSize({width:390,height:844});await ready(page);expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
 await page.locator('#stage').focus();await page.keyboard.press('Space');await expect.poll(()=>page.locator('#video').evaluate(v=>!v.paused)).toBe(true);await page.keyboard.press('Space');
 await page.locator('#show-still').click();await page.screenshot({path:testInfo.outputPath('mobile.png'),fullPage:true});
 // Queue a large local read and cancel immediately. No previous result may
 // replace the next operation when the worker/reader finishes late.
 await page.locator('#files').setInputFiles({name:'pending.jpg',mimeType:'image/jpeg',buffer:Buffer.alloc(16*1024*1024)});await page.locator('#cancel').click();await expect(page.locator('#work-status')).toContainText('已取消');await ready(page);await expect(page.locator('#frame-times')).toContainText('48 帧');
});
test('real AAC audio decodes and mute/keyboard controls operate on the original movie',async({page},testInfo)=>{
 await page.goto('/');await page.locator('#declared').check();await page.locator('#time-value').fill('13');await page.locator('#time-scale').fill('16');
 await page.locator('#files').setInputFiles([path.join(demo,'cover.jpg'),path.resolve('public/acceptance/audio.mp4')]);await expect(page.locator('#work-status')).toContainText('解析完成');await expect(page.locator('#play')).toBeEnabled();await expect(page.locator('#frame-times')).toContainText('48 帧');
 const audio=await page.locator('#video').evaluate(async v=>{const bytes=await(await fetch(v.src)).arrayBuffer();const context=new OfflineAudioContext(2,1,48000);const buffer=await context.decodeAudioData(bytes);const samples=buffer.getChannelData(0);let sum=0,crossings=0;for(let i=1;i<samples.length;i++){sum+=samples[i]*samples[i];if(samples[i-1]<=0&&samples[i]>0)crossings++;}return {channels:buffer.numberOfChannels,sampleRate:buffer.sampleRate,duration:buffer.duration,rms:Math.sqrt(sum/samples.length),frequency:crossings/buffer.duration};});
 expect(audio.channels).toBe(2);expect(audio.sampleRate).toBe(48000);expect(audio.rms).toBeGreaterThan(.03);expect(Math.abs(audio.frequency-440)).toBeLessThan(2);
 await expect(page.locator('#mute')).toHaveAttribute('aria-pressed','true');await page.locator('#play').click();await expect.poll(()=>page.locator('#video').evaluate(v=>!v.paused)).toBe(true);await page.locator('#mute').click();await expect.poll(()=>page.locator('#video').evaluate(v=>v.muted)).toBe(false);await page.locator('#stage').press('m');await expect.poll(()=>page.locator('#video').evaluate(v=>v.muted)).toBe(true);
 await testInfo.attach('audio-acceptance',{body:JSON.stringify(audio),contentType:'application/json'});
});
test('actual HEIC still and unsupported JPEG2000 movie show decoder limits without changing downloads',async({page})=>{
 await page.goto('/');await page.locator('#declared').check();
 await page.locator('#files').setInputFiles([path.resolve('public/acceptance/still.heic'),path.join(demo,'motion.mp4')]);await expect(page.locator('#work-status')).toContainText('解析完成');await expect(page.locator('#preview-message')).toContainText('浏览器无法解码原始照片');await expect(page.locator('#play')).toBeEnabled();
 const download=page.waitForEvent('download');await page.locator('#resources a').first().click();expect(fs.readFileSync(await(await download).path()).equals(fs.readFileSync(path.resolve('public/acceptance/still.heic')))).toBe(true);
 await page.locator('#compatible-cover').setInputFiles(path.join(demo,'cover.jpg'));await expect(page.locator('#still')).toBeVisible();await expect(page.locator('#stage-label')).toContainText('用户提供，非原图');
 await page.locator('#files').setInputFiles([path.join(demo,'cover.jpg'),path.resolve('public/acceptance/unsupported.mov')]);await expect(page.locator('#work-status')).toContainText('解析完成');await expect(page.locator('#preview-message')).toContainText('没有隐式转码');await expect(page.locator('#play')).toBeDisabled();await expect(page.locator('#resources a')).toHaveCount(2);
});
test('replacing media revokes URLs and discards late seeks and cancelled demo fetches',async({page})=>{
 await page.addInitScript(()=>{const create=URL.createObjectURL.bind(URL),revoke=URL.revokeObjectURL.bind(URL);window.localBlobUrls=new Set();URL.createObjectURL=blob=>{const value=create(blob);window.localBlobUrls.add(value);return value;};URL.revokeObjectURL=value=>{window.localBlobUrls.delete(value);revoke(value);};});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));await ready(page);expect(await page.evaluate(()=>window.localBlobUrls.size)).toBe(2);
 await page.locator('#compatible-cover').setInputFiles(path.join(demo,'cover.jpg'));await page.locator('#compatible-cover').setInputFiles([]);await page.locator('#compatible-cover').setInputFiles(path.join(demo,'cover.jpg'));expect(await page.evaluate(()=>window.localBlobUrls.size)).toBe(3);
 await page.evaluate(()=>{document.getElementById('next-frame').click();document.getElementById('cancel').click();});await expect(page.locator('#work-status')).toContainText('已取消');await expect(page.locator('#play')).toBeDisabled();expect(await page.evaluate(()=>window.localBlobUrls.size)).toBe(0);
 let release;const held=new Promise(resolve=>release=resolve);let intercepted;const seen=new Promise(resolve=>intercepted=resolve);await page.route('**/demo/motion-photo.jpg',async route=>{intercepted();await held;await route.continue().catch(()=>{});});
 await page.locator('#demo').click();await seen;await page.locator('#cancel').click();release();await page.unroute('**/demo/motion-photo.jpg');
 await page.locator('#apple-demo').click();await expect(page.locator('#format')).toHaveText('AppleLivePhoto');await expect(page.locator('#cover-time')).toBeEnabled();expect(await page.evaluate(()=>window.localBlobUrls.size)).toBe(2);await expect.poll(()=>page.locator('#video').evaluate(v=>!v.seeking)).toBe(true);await page.evaluate(()=>{document.getElementById('video').dispatchEvent(new Event('error'));document.getElementById('still').dispatchEvent(new Event('error'));});await expect(page.locator('#play')).toBeEnabled();await expect(page.locator('#preview-message')).not.toContainText('无法');expect(errors).toEqual([]);
});
