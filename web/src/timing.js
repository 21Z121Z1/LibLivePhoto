// Rational values are retained as strings. Floating point is used only at the
// HTML media / LivePhotosKit playback boundary, never for source rewriting.
export function seconds(time) {
  if (!time || !/^-?\d+$/.test(time.value) || !Number.isInteger(time.timescale) || time.timescale<=0 || time.timescale>0xffffffff) throw new Error('无效的精确时间');
  const value=BigInt(time.value), scale=BigInt(time.timescale), whole=value/scale;
  if (whole>BigInt(Number.MAX_SAFE_INTEGER)||whole<BigInt(Number.MIN_SAFE_INTEGER)) throw new Error('时间超出浏览器可表示范围');
  return Number(whole)+Number(value%scale)/Number(scale);
}
export function presentationTiming(manifest) {
  const video=manifest.tracks.filter(t=>t.kind==='Video');
  const frames=video.length===1&&video[0].frameTiming?.status==='available'?video[0].frameTiming.frames:[];
  return { exactPhotoTime:manifest.photoTime, photoTime:manifest.photoTime?seconds(manifest.photoTime):null,
    exactFrames:frames, frameTimes:frames.map(f=>seconds(f.time)),
    reason:video.length!==1?'多视频轨道的浏览器选择无法确认':video[0].frameTiming?.reason??null };
}
export function frameIndexAt(times,time){let answer=-1;for(let i=0;i<times.length;i++){if(times[i]>time)break;answer=i;}return answer;}
// Optional adapter for an app that already supplies Apple's Player. No CDN is
// loaded by this demo. An HTMLVideoElement does not auto-detect these timings.
export function configureLivePhotosKitPlayer(player,{photoSrc,videoElement,originalMovieSrc,timing}) {
  if(timing.photoTime===null||!timing.frameTimes.length) throw new Error('精确照片/帧时序不可用，不能自动配置 LivePhotosKit');
  player.photoSrc=photoSrc;
  player.videoSrc=videoElement;
  if(originalMovieSrc)player.metadataVideoSrc=originalMovieSrc;
  player.photoTime=timing.photoTime;
  player.frameTimes=[...timing.frameTimes];
  return player;
}
