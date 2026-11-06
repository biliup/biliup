import {useEffect,useRef,type VideoHTMLAttributes} from 'react'
import type {Media} from '@/app/lib/editor/core/timeline/types'
import {bindEditorMedia,seekEditorMedia} from './mediaElement'

type Props=Omit<VideoHTMLAttributes<HTMLVideoElement>,'src'>&{
  readonly media:Pick<Media,'id'|'path'>
  readonly src:string|undefined
  readonly position:number|undefined
}

/** Library skimming and the stage use the same native video source lifecycle. */
export function MediaVideo({media,src,position,...props}:Props){
  const ref=useRef<HTMLVideoElement>(null),at=useRef(position);at.current=position
  useEffect(()=>{
    const video=ref.current
    if(!video||!src)return
    return bindEditorMedia(video,media,src,()=>at.current??video.currentTime)
  },[media.id,media.path,src])
  useEffect(()=>{if(ref.current&&position!==undefined)seekEditorMedia(ref.current,position)},[position,media.id,media.path,src])
  return <video {...props} ref={ref} onError={event=>{
    props.onError?.(event)
    if(event.currentTarget.error||event.currentTarget.dataset.sourceError)event.currentTarget.style.visibility='hidden'
  }} onLoadedData={event=>{
    event.currentTarget.style.visibility=''
    props.onLoadedData?.(event)
  }}/>
}
