// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A sandboxed graphic asks its owning compositor for registered project media. Its immutable
// frame context includes the source version; replacing media replaces the graphic iframe too.
import type {Project} from '@/app/lib/editor/core/timeline/types';
import {currentEditorStore} from '@/app/lib/editor/state/store/core';
import {graphicMediaUrl} from '@/app/lib/editor/state/store/urls';
import {editorText} from '@/app/lib/editor/i18n';
import {proxiesOn} from '../settings/proxies';
import {graphicMediaAsk} from './graphicMessage';
import {parseGraphicMediaRequest} from '@/app/lib/editor/core/visual/graphicMedia';

/** x-biliup-full: the exact picture size represented by a proxy picture. */
const fullOf = (value:string|null) => {
  const match = value ? /^(\d{1,4})x(\d{1,4})$/.exec(value) : null;
  return match ? {full:{width:Number(match[1]),height:Number(match[2])}} : {};
};

export function bindGraphicMedia(frame:HTMLIFrameElement,dir:string,project:Project):()=>void {
  const owner=currentEditorStore(),lifetime=new AbortController();
  const active=()=>!lifetime.signal.aborted&&!!owner&&currentEditorStore()===owner
    &&owner.getSnapshot().snapshot?.dir===dir&&frame.isConnected;
  const onMessage=(event:MessageEvent)=>{
    if(!active())return;
    const ask=graphicMediaAsk(event,[frame.contentWindow]);
    if(!ask)return;
    const target=event.source as Window;
    const reply=(result:{blob:Blob;full?:{width:number;height:number}}|{error:string})=>{
      if(active()&&frame.contentWindow===target)target.postMessage({biliup:'media-reply',seq:ask.seq,...result},'*');
    };
    const query=new URLSearchParams();
    for(const [key,value] of Object.entries({t:ask.t,w:ask.w,h:ask.h}))if(value!==undefined)query.set(key,String(value));
    const request=parseGraphicMediaRequest(ask.id,query);
    if('problem' in request)return reply({error:request.problem});
    const media=project.media.find(media=>media.id===request.id);
    if(!media)return reply({error:editorText('The graphic references media that is not in this version.')});
    const url=graphicMediaUrl(media,request.t,request.w,request.h,ask.p===true&&proxiesOn());
    if(!url)return;
    const signal=AbortSignal.any([lifetime.signal,AbortSignal.timeout(30000)]);
    void fetch(url,{credentials:'include',signal}).then(async response=>{
      if(!response.ok)throw new Error((await response.text())||editorText("Couldn't load graphic media ({0}).",{0:response.status}));
      reply({blob:await response.blob(),...fullOf(response.headers.get('x-biliup-full'))});
    }).catch(error=>{if(!lifetime.signal.aborted)reply({error:error instanceof Error?error.message:String(error)})});
  };
  window.addEventListener('message',onMessage);
  return ()=>{lifetime.abort();window.removeEventListener('message',onMessage)};
}
