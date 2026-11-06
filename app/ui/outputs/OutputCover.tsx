'use client'
import {useEffect,useRef,useState} from 'react'
import {Camera,ImagePlus} from 'lucide-react'
import {apiResourceUrl} from '@/app/lib/renders'
import {displayOutput,type OutputEntry} from '@/app/lib/outputs'
import {type CoverSelection,type OutputCover as Cover} from '@/app/lib/output-publish'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import styles from './outputs.module.css'

export function OutputCover({entry,cover,choice,durationMs,disabled,onChoice,onUpload,onCapture,onPreviewReady}:{
 entry:OutputEntry;cover:Cover|null;choice:CoverSelection;durationMs:number;disabled:boolean
 onChoice:(choice:CoverSelection)=>void;onUpload:(file:File)=>Promise<boolean>;onCapture:(atMs:number)=>Promise<boolean>
 onPreviewReady:(id:string,ready:boolean)=>void
}){
 useEditorLanguage()
 const input=useRef<HTMLInputElement>(null),video=useRef<HTMLVideoElement>(null),mounted=useRef(true)
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;video.current?.pause()}},[])
 const [capturing,setCapturing]=useState(false),[at,setAt]=useState(0),[videoError,setVideoError]=useState(false),[imageError,setImageError]=useState<string>(),[error,setError]=useState('')
 const [imageAttempt,setImageAttempt]=useState(0)
 useEffect(()=>{if(disabled){video.current?.pause();setCapturing(false)}},[disabled])
 const url=displayOutput(entry).url,max=Math.max(0,(durationMs-1)/1000)
 const capture=async()=>{
  video.current?.pause();setError('')
  const current=video.current&&!videoError&&video.current.readyState>=2?video.current.currentTime:at
  const ok=await onCapture(Math.min(durationMs-1,Math.max(0,Math.round(current*1000))))
  if(ok&&mounted.current)setCapturing(false)
 }
 const seek=(seconds:number)=>{
  if(!Number.isFinite(seconds))return
  const next=Math.max(0,Math.min(max,seconds));setAt(next)
  if(video.current?.readyState){video.current.pause();video.current.currentTime=next}
 }
 return <section className={styles.coverEditor} aria-label={t('Cover')}>
  <strong>{t('Cover')}</strong>
  <div className={styles.coverChoices}>
   <button type="button" className="btn" aria-pressed={choice.source==='template'} disabled={disabled} onClick={()=>{setCapturing(false);onChoice({source:'template'})}}>{t('Template cover')}</button>
   <button type="button" className="btn" aria-pressed={choice.source==='default'} disabled={disabled} onClick={()=>{setCapturing(false);onChoice({source:'default'})}}>{t('Bilibili default cover')}</button>
   <button type="button" className="btn" disabled={disabled} onClick={()=>input.current?.click()}><ImagePlus size={16}/>{t('Upload image')}</button>
   <button type="button" className="btn" aria-expanded={capturing} disabled={disabled||!url||durationMs<=0} onClick={()=>{setCapturing(value=>!value);setVideoError(false);setError('')}}><Camera size={16}/>{t('Capture from output')}</button>
   <input ref={input} type="file" hidden disabled={disabled} accept="image/jpeg,image/png,image/webp" onChange={event=>{
    const file=event.target.files?.[0];event.target.value='';if(!file)return
    if(!file.size||file.size>5*1024*1024){setError(t('Cover images must be between 1 byte and 5 MiB.'));return}
    setError('');void onUpload(file).then(ok=>{if(ok&&mounted.current)setCapturing(false)})
   }}/>
  </div>
  {capturing&&url?<div className={styles.coverCapture}>
   <video ref={video} src={apiResourceUrl(url)} crossOrigin="use-credentials" controls muted preload="metadata" onError={()=>setVideoError(true)} onTimeUpdate={event=>setAt(Math.min(max,event.currentTarget.currentTime))} onLoadedMetadata={event=>{event.currentTarget.currentTime=Math.min(at,max)}}/>
   {videoError&&<p role="status">{t('This browser cannot play the output. Choose a time to capture on the server.')}</p>}
   <label className={styles.coverTime}>{t('Time (seconds)')}<input className="input" type="number" min={0} max={max} step={0.001} disabled={disabled} value={Number(at.toFixed(3))} onChange={event=>seek(Number(event.target.value))}/><input type="range" aria-label={t('Capture time')} min={0} max={max} step={0.001} value={at} disabled={disabled} onChange={event=>seek(Number(event.target.value))}/></label>
   <div className={styles.dialogFooter}><button type="button" className="btn" disabled={disabled} onClick={()=>setCapturing(false)}>{t('Cancel')}</button><button type="button" className="btn primary" disabled={disabled} onClick={()=>void capture()}>{t('Use this frame')}</button></div>
  </div>:cover?<figure className={styles.coverPreview}>
   <img key={cover.id+'-'+imageAttempt} src={apiResourceUrl(cover.url)+'?view='+imageAttempt} crossOrigin="use-credentials" alt={t('Publication cover preview')} onError={()=>{setImageError(cover.id);onPreviewReady(cover.id,false)}} onLoad={()=>{setImageError(undefined);onPreviewReady(cover.id,true)}}/>
   <figcaption>{cover.name}</figcaption>
   {imageError===cover.id&&<p role="alert">{t('The cover preview could not be loaded.')} <button type="button" className="btn" onClick={()=>{onPreviewReady(cover.id,false);setImageAttempt(attempt=>attempt+1)}}>{t('Retry')}</button></p>}
  </figure>:null}
  {error&&<p role="alert">{error}</p>}
 </section>
}
