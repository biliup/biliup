import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { Square, Clapperboard, RefreshCw } from "lucide-react";
import { useState, useRef, useEffect, useContext, useCallback } from "react";
import { playback, getPlayback } from "@/app/ui/editor/preview/playback";
import {currentEditorStore,EditorStoreContext} from '@/app/lib/editor/state/store/core';
import type {EditorStore} from '@/app/lib/editor/project-store';

type PreviewRun={key:string;mode:'alone'|'timeline';el:HTMLAudioElement;owner:EditorStore;project:unknown;from:number;fps:number;started:boolean;waiting:boolean;syncing:boolean;release:Array<()=>void>};

/**
 * One sound playing at a time across the panel: its key, the element, and the waveforms worked out
 * so far. F28 (D170): `withTimeline` plays the sound together with the timeline from the playhead;
 * stopping either (the button, the sound's end, a pause of the timeline) stops both, and the
 * playhead goes back to where it started, ready for the next try.
 */
export function usePreview() {
  const owner=useContext(EditorStoreContext);
  const [playing, setPlaying] = useState<string | undefined>();
  const [mode, setMode] = useState<"alone" | "timeline">("alone");
  const [progress, setProgress] = useState(0);
  const [loading,setLoading]=useState(false);
  const [error,setError]=useState<string>();
  const [peaks, setPeaks] = useState<Readonly<Record<string, ReadonlyArray<number>>>>({});
  const active=useRef<PreviewRun|undefined>(undefined);
  const mounted=useRef(false);
  const waves=useRef(new Map<string,AbortController>());
  const peakKey=(key:string,url:string)=>`${key}\0${url}`;
  const stop=useCallback((rewind=true,pauseTimeline=true)=>{
    const run=active.current;
    active.current=undefined;
    waves.current.forEach(request=>request.abort());waves.current.clear();
    if(run){
      run.release.forEach(release=>release());
      const el=run.el;
      el.onplaying=null;el.onwaiting=null;el.onstalled=null;el.ontimeupdate=null;el.onended=null;el.onerror=null;
      el.pause();el.removeAttribute('src');el.load();
      if(run.mode==='timeline'&&run.started&&currentEditorStore()===run.owner){
        if(pauseTimeline)playback.pause();
        if(rewind&&run.owner.getSnapshot().snapshot?.project===run.project&&getPlayback().fps===run.fps)playback.seek(run.from);
      }
    }
    if(mounted.current){setPlaying(undefined);setLoading(false);setProgress(0);setError(undefined);}
  },[]);
  useEffect(()=>{
    mounted.current=true;
    setPlaying(undefined);setMode('alone');setProgress(0);setLoading(false);setError(undefined);setPeaks({});
    const hidden=()=>{if(document.hidden)stop();};
    document.addEventListener('visibilitychange',hidden);
    return ()=>{
      mounted.current=false;stop();
      document.removeEventListener('visibilitychange',hidden);
    };
  },[owner,stop]);
  const start = (key: string, url: string, wave: boolean, withTimeline: boolean) => {
    stop();setError(undefined);
    if(!mounted.current||!owner||currentEditorStore()!==owner||owner.getSnapshot().connection!=='connected')return;
    const clock=getPlayback(),project=owner.getSnapshot().snapshot?.project;
    if(withTimeline&&(clock.frames<=1||clock.frame>=clock.frames-1)){
      setError(editorText('Move the playhead before the end to preview this sound with the timeline.'));return;
    }
    playback.pause();
    const el = new Audio();
    el.crossOrigin='use-credentials';el.preload='auto';el.src=url;
    el.dataset.key = key;
    const run:PreviewRun={key,mode:withTimeline?'timeline':'alone',el,owner,project,from:Math.floor(getPlayback().frame),fps:clock.fps,started:false,waiting:false,syncing:false,release:[]};
    active.current=run;
    const live=()=>mounted.current&&active.current===run&&currentEditorStore()===owner;
    const fail=(message:string)=>{if(!live())return;stop();setError(editorText(message));};
    el.ontimeupdate=()=>{if(live())setProgress(Number.isFinite(el.duration)&&el.duration>0?Math.max(0,Math.min(1,el.currentTime/el.duration)):0);};
    el.onended=()=>{if(live())stop();};
    el.onerror=()=>fail(el.error?.code===3||el.error?.code===4?'This audio format cannot be previewed in this browser.':'The audio preview could not be loaded.');
    el.onplaying=()=>{
      if(!live())return;
      setLoading(false);
      if(!withTimeline)return;
      run.syncing=true;
      if(!run.started)playback.seek(run.from);
      run.started=true;run.waiting=false;
      playback.play();
      run.syncing=false;
    };
    const waiting=()=>{
      if(!live())return;
      setLoading(true);
      if(withTimeline&&run.started){run.waiting=true;run.syncing=true;playback.pause();run.syncing=false;}
    };
    el.onwaiting=waiting;
    // A stalled network request can still have playable buffered audio.
    el.onstalled=()=>{if(el.readyState<3)waiting();};
    setPlaying(key);
    setMode(run.mode);setLoading(true);setProgress(0);
    run.release.push(owner.subscribe(()=>{
      if(active.current===run&&(owner.getSnapshot().snapshot?.project!==project||owner.getSnapshot().connection!=='connected'))stop(false);
    }));
    if (withTimeline) {
      run.release.push(playback.subscribe(()=>{
        if(active.current!==run||run.syncing)return;
        if(!live()){stop(false,false);return;}
        const position=getPlayback();
        if(position.fps!==run.fps){stop(false);return;}
        if(!run.started){if(position.frame!==run.from||position.playing)stop(false,false);return;}
        if(run.waiting){if(position.playing)stop(false,false);return;}
        if(!position.playing){stop();return;}
        if(position.rate!==1){stop(false,false);return;}
        const time=(position.frame-run.from)/run.fps;
        if(time<0||Number.isFinite(el.duration)&&time>=el.duration){stop(false,false);return;}
        // Follow the same clock as video, including seeking or a delayed video decoder.
        if(el.readyState>=2&&Math.abs(el.currentTime-time)>0.12){
          try{el.currentTime=time;}catch{fail('The audio preview could not be loaded.');}
        }
      }));
    }
    void el.play().catch(problem=>fail(problem?.name==='NotAllowedError'?'Click Listen again to allow audio playback.':'The audio preview could not be loaded.'));
    const cacheKey=peakKey(key,url);
    if(wave&&!peaks[cacheKey]&&!waves.current.has(cacheKey)){
      const request=new AbortController();waves.current.set(cacheKey,request);
      void waveOf(url,request.signal).then(value=>{
        if(value&&mounted.current&&!request.signal.aborted&&currentEditorStore()===owner)setPeaks(before=>Object.fromEntries(Object.entries({...before,[cacheKey]:value}).slice(-64)));
      }).finally(()=>{if(waves.current.get(cacheKey)===request)waves.current.delete(cacheKey);});
    }
  };
  const toggle = (key: string, url: string | undefined, wave: boolean) => {
    if (active.current?.key === key && active.current.mode === "alone") return stop();
    if (!url) return;
    start(key, url, wave, false);
  };
  const withTimeline = (key: string, url: string | undefined, wave: boolean) => {
    if (active.current?.key === key && active.current.mode === "timeline") return stop();
    if (!url) return;
    start(key, url, wave, true);
  };
  return { playing, mode, progress, loading, error, peaksFor:(key:string,url:string|undefined)=>url?peaks[peakKey(key,url)]:undefined, toggle, withTimeline, stop };
}

/** F28 (D170): play the sound and the timeline together from the playhead. */
export function TogetherButton({ preview, soundKey, url, wave }: { preview: Preview; soundKey: string; url: string | undefined; wave: boolean }) {
  useEditorLanguage();
  const on = preview.playing === soundKey && preview.mode === "timeline";
  return (
    <button className={`icon-btn${on ? " playing" : ""}`} data-testid="sound-with-timeline" data-playing={on} disabled={!url} title={on ? editorText("Stop") : editorText("Play with the timeline from the playhead")} onClick={() => preview.withTimeline(soundKey, url, wave)}>
      {on ? preview.loading?<RefreshCw className="spin"/>:<Square /> : <Clapperboard />}
    </button>
  );
}

/** The sound's peaks in 48 bins (0–1), decoded from its preview; undefined when it can't be read. */
async function waveOf(url: string,signal:AbortSignal): Promise<ReadonlyArray<number> | undefined> {
  try {
    const response=await fetch(url,{credentials:'include',signal});
    const maximum=20*1024*1024;
    if(!response.ok||Number(response.headers.get('content-length'))>maximum){await response.body?.cancel();return;}
    const reader=response.body?.getReader();if(!reader)return;
    const chunks:Uint8Array[]=[];let size=0;
    try{
      while(true){const part=await reader.read();if(part.done)break;size+=part.value.byteLength;if(size>maximum){await reader.cancel();return;}chunks.push(part.value);}
    }finally{reader.releaseLock();}
    signal.throwIfAborted();
    const bytes=new Uint8Array(size);let offset=0;
    for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
    const context = new OfflineAudioContext(1, 1, 44100);
    const buffer = await context.decodeAudioData(bytes.buffer);
    signal.throwIfAborted();
    const data = buffer.getChannelData(0);
    const bins = 48;
    const binSize = Math.max(1, Math.ceil(data.length / bins));
    const out: Array<number> = [];
    for (let b = 0; b < bins; b++) {
      let max = 0;
      for (let i = b * binSize; i < Math.min(data.length, (b + 1) * binSize); i++) max = Math.max(max, Math.abs(data[i]!));
      out.push(max);
    }
    const top = Math.max(...out, 1e-6);
    return out.map((v) => v / top);
  } catch {
    return undefined;
  }
}

export function Wave({ peaks, progress }: { peaks: ReadonlyArray<number>; progress: number }) {
  useEditorLanguage();
  return (
    <span className="lib-wave" data-testid="library-wave" aria-hidden="true">
      {peaks.map((p, i) => (
        <i key={i} className={i / peaks.length < progress ? "on" : ""} style={{ height: `${Math.max(8, p * 100)}%` }} />
      ))}
    </span>
  );
}

export type Preview = ReturnType<typeof usePreview>;
