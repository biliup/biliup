'use client'
import {useState} from 'react'
import useSWR from 'swr'
import {RefreshCw,X} from 'lucide-react'
import {autoClipError,fetchEstimate,startAutoClip,type AutoClipStatus} from '@/app/lib/auto-clip'
import {useTextPref} from '@/app/lib/use-local-pref'
import {editorText as t} from '@/app/lib/editor/i18n'
import {GlobalDialog} from '../../controls/GlobalDialog'
import {useEditorAction} from '../../controls/useEditorAction'

const unit=(raw:string)=>raw.trim()&&Number.isFinite(Number(raw))&&Number(raw)>=0?Number(raw):undefined
export function SuggestionEstimate({sessionId,status,onClose}:{sessionId:number;status:AutoClipStatus|undefined;onClose:()=>void}){
  const [reuse,setReuse]=useState(true)
  const [asr,setAsr]=useTextPref('biliup.autoClip.price.asrPerMinute'),[chat,setChat]=useTextPref('biliup.autoClip.price.chatPerMillion')
  const {busy,error,setError,editable,run}=useEditorAction()
  const {data,error:loadError,isValidating,mutate}=useSWR(['editor-suggestion-estimate',sessionId,reuse],()=>fetchEstimate(sessionId,reuse),{revalidateOnFocus:false,shouldRetryOnError:false})
  const estimate=data?.estimate,blocked=!estimate||(estimate.over_limit&&estimate.basis==='silence')||(estimate.chat_over_limit&&estimate.chat.basis==='transcript')
  const asrPrice=unit(asr),chatPrice=unit(chat),cost=estimate&&(asrPrice!==undefined||chatPrice!==undefined)?(asrPrice??0)*estimate.asr_seconds/60+(chatPrice??0)*estimate.chat.tokens/1e6:undefined
  const start=()=>{
    if(blocked||loadError||isValidating||!status?.enabled||!status.configured||!status.asr_model)return
    return run(()=>startAutoClip(sessionId,reuse),{onSuccess:onClose})
  }
  return <GlobalDialog label={t('Generate suggestions')} onClose={onClose} busy={busy}><section className="modal suggestion-estimate">
    <header><h2>{t('Generate suggestions')}</h2><button className="icon-btn" disabled={busy} aria-label={t('Close')} onClick={onClose}><X/></button></header>
    <label className="check"><input type="checkbox" checked={reuse} disabled={busy} onChange={event=>{setError('');setReuse(event.target.checked)}}/>{t('Reuse existing transcripts')}</label>
    {error&&!loadError&&<p role="alert">{error}</p>}
    {loadError?<div><p role="alert">{autoClipError(loadError)}</p><button className="btn" disabled={busy||isValidating} onClick={()=>void run(()=>mutate(),{readOnly:true})}><RefreshCw/>{t('Retry')}</button></div>:!estimate?<p>{t('Estimating…')}</p>:<>
      <dl><dt>{t('Transcription')}</dt><dd>{t('About {0} minutes',{0:(estimate.asr_seconds/60).toFixed(1)})}</dd><dt>{t('Analysis')}</dt><dd>{t('About {0} tokens',{0:estimate.chat.tokens.toLocaleString()})}</dd><dt>{t('Images')}</dt><dd>{estimate.chat.images}</dd><dt>{t('Transcription provider')}</dt><dd>{status?.asr_host??'—'}</dd><dt>{t('Analysis provider')}</dt><dd>{status?.api_host??'—'}</dd></dl>
      <div className="suggestion-prices"><label>{t('Price per audio minute')}<input className="field" type="number" min="0" step="any" value={asr} onChange={e=>setAsr(e.target.value)}/></label><label>{t('Price per million tokens')}<input className="field" type="number" min="0" step="any" value={chat} onChange={e=>setChat(e.target.value)}/></label></div>
      {cost!==undefined&&<p>{t('Estimated cost: {0}',{0:cost.toFixed(3)})}</p>}
      {estimate.message&&<p role={estimate.over_limit?'alert':undefined}>{estimate.message}</p>}{estimate.chat_message&&<p role={estimate.chat_over_limit?'alert':undefined}>{estimate.chat_message}</p>}
    </>}
    <footer><button className="btn" disabled={busy} onClick={onClose}>{t('Cancel')}</button><button className="btn primary" disabled={busy||!editable||blocked||isValidating||!!loadError||!status?.enabled||!status.configured||!status.asr_model} onClick={()=>void start()}>{t(busy?(loadError?'Estimating…':'Starting…'):'Confirm generation')}</button></footer>
  </section></GlobalDialog>
}
