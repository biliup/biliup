const prefix='biliup.editor.workspace.'
export type EditorView={frame:number;fps:number;markIn?:number;markOut?:number}
export function readEditorView(key:string):EditorView|undefined{
 try{const value=JSON.parse(localStorage.getItem(`${prefix}view.${key}`)??'null');if(!value||!Number.isFinite(value.frame)||value.frame<0||!Number.isFinite(value.fps)||value.fps<=0)return
  return {frame:value.frame,fps:value.fps,markIn:Number.isFinite(value.markIn)&&value.markIn>=0?value.markIn:undefined,markOut:Number.isFinite(value.markOut)&&value.markOut>=0?value.markOut:undefined}
 }catch{return}
}
export function saveEditorView(key:string,value:EditorView){try{localStorage.setItem(`${prefix}view.${key}`,JSON.stringify({frame:value.frame,fps:value.fps,markIn:value.markIn,markOut:value.markOut}))}catch{}}
