'use client'
import {useEffect,useRef,type ReactNode} from 'react'
import {createPortal} from 'react-dom'
import styles from '../workspace.module.css'

/** The browser's top layer escapes panel clipping and owns modal focus/inert behaviour. */
export function GlobalDialog({children,label,onClose,busy=false}:{children:ReactNode;label:string;onClose:()=>void;busy?:boolean}){
 const dialog=useRef<HTMLDialogElement>(null)
 useEffect(()=>{const element=dialog.current;if(!element)return;element.showModal();return()=>element.close()},[])
 if(typeof document==='undefined')return null
 return createPortal(<dialog ref={dialog} data-editor-dialog className={`${styles.workspace} ${styles.modalRoot}`} aria-label={label} onCancel={event=>{event.preventDefault();if(!busy)onClose()}} onKeyDown={event=>event.stopPropagation()}>
  <div className="modal-layer" onClick={event=>{event.stopPropagation();if(event.target===event.currentTarget&&!busy)onClose()}}>{children}</div>
 </dialog>,document.body)
}
