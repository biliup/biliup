// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Browser-owned capture: the native picker is opened directly by the Record click.
import { type Running, type TakeOptions, stopTake, running } from './take';
import { set, state, errorText, type PartKind } from './state';
import { pausePreview, previewWanted, openPreview, cameraConstraints, micConstraints } from './devices';
import {editorText} from '@/app/lib/editor/i18n';

const DISPLAY_AUDIO: MediaTrackConstraints={echoCancellation:false,noiseSuppression:false,autoGainControl:false};
export const NO_SYSTEM_SOUND='No shared audio. Enable audio in the browser sharing dialog or choose a tab that supports audio.';
function watchInput(stream:MediaStream,r:Running){
  for(const track of stream.getTracks())track.addEventListener('ended',()=>{
    if(running!==r||r.cancelled||!['recording','countdown'].includes(state.phase))return
    r.failure=editorText('A recording device disconnected.');void stopTake()
  },{once:true})
  return stream
}
async function displayStream(r:Running,sound:boolean){
  if(!navigator.mediaDevices?.getDisplayMedia)throw new Error(editorText('Screen capture requires HTTPS or localhost and a supported browser.'));
  const stream=await navigator.mediaDevices.getDisplayMedia({video:{frameRate:30},audio:sound?DISPLAY_AUDIO:false});
  r.streams.push(stream);
  stream.getVideoTracks()[0]?.addEventListener('ended',()=>{if(running===r)void stopTake()},{once:true});
  return stream;
}
function systemStream(display:MediaStream){
  const audio=display.getAudioTracks();
  if(!audio.length)throw new Error(editorText(NO_SYSTEM_SOUND));
  return new MediaStream(audio);
}
async function takeCamera(options:TakeOptions,r:Running){
  pausePreview();
  r.cleanups.push(()=>{
    if(state.preview&&r.streams.includes(state.preview))set({preview:undefined});
    if(previewWanted)void openPreview(previewWanted.id,previewWanted.resolution);
  });
  const stream=watchInput(await navigator.mediaDevices.getUserMedia({video:cameraConstraints(options.camera,options.resolution??'1080p')}),r);
  r.cameraStream=stream;
  r.streams.push(stream);
  if(previewWanted&&!r.cancelled)set({preview:stream,previewError:undefined});
  return stream;
}
export async function openStreams(options:TakeOptions,r:Running):Promise<MediaStream|string>{
  try{
    if(options.mode==='voiceover'){
      const mic=watchInput(await navigator.mediaDevices.getUserMedia({audio:micConstraints(options.mic)}),r);
      r.streams.push(mic);return mic;
    }
    if(options.mode==='system')return systemStream(await displayStream(r,true));
    const picture=options.mode==='camera'?await takeCamera(options,r):await displayStream(r,false);
    if(options.mode!=='camera'&&!r.cancelled)set({screenLive:picture});
    const tracks=[...picture.getVideoTracks()];
    if(options.withMic&&!r.cancelled){
      const mic=watchInput(await navigator.mediaDevices.getUserMedia({audio:micConstraints(options.mic)}),r);
      r.streams.push(mic);tracks.push(...mic.getAudioTracks());
    }
    return new MediaStream(tracks);
  }catch(error){return errorText(error)}
}
export async function openParts(options:TakeOptions,r:Running):Promise<Map<PartKind,MediaStream>|string>{
  const parts=options.parts??[],out=new Map<PartKind,MediaStream>();
  try{
    // Display first: preserve the click's activation before other permission dialogs.
    if(parts.includes('screen')||parts.includes('system')){
      const display=await displayStream(r,parts.includes('system'));
      if(r.cancelled)return out;
      if(parts.includes('screen')){out.set('screen',new MediaStream(display.getVideoTracks()));set({screenLive:out.get('screen')})}
      if(parts.includes('system'))out.set('system',systemStream(display));
    }
    if(parts.includes('camera')){out.set('camera',await takeCamera(options,r));if(r.cancelled)return out}
    if(parts.includes('voiceover')){
      const mic=watchInput(await navigator.mediaDevices.getUserMedia({audio:micConstraints(options.mic)}),r);
      r.streams.push(mic);out.set('voiceover',mic);
    }
    return out.size?out:editorText('Turn on at least one source.');
  }catch(error){return errorText(error)}
}
