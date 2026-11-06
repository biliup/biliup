import {editorText} from './i18n'

export async function copyText(text:string):Promise<void> {
 if(!navigator.clipboard?.writeText)throw new Error(editorText('Clipboard access requires HTTPS or localhost.'))
 await navigator.clipboard.writeText(text)
}
