import { useSyncExternalStore } from 'react'
import messages from './locales/zh-CN.json'
import english from './locales/en.json'

export type EditorLanguage = 'zh-CN' | 'en'
export const EDITOR_LANGUAGE_KEY = 'biliup.editor.language'
let language: EditorLanguage | undefined
let restored = false
const listeners = new Set<() => void>()
export function getEditorLanguage(): EditorLanguage {
  return language ?? 'zh-CN'
}
/** Restore after hydration: the server and first client render both start in Chinese. */
export function restoreEditorLanguage() {
  if (restored) return
  restored = true
  try { language = typeof localStorage !== 'undefined' && localStorage.getItem(EDITOR_LANGUAGE_KEY) === 'en' ? 'en' : 'zh-CN' }
  catch { language = 'zh-CN' }
  notify()
}
const notify = () => listeners.forEach(listener => listener())
export function setEditorLanguage(next: EditorLanguage) {
  restored = true
  language = next === 'en' ? 'en' : 'zh-CN'
  try { localStorage.setItem(EDITOR_LANGUAGE_KEY, language) } catch { /* Current tab still switches if browser storage is unavailable. */ }
  notify()
}
function subscribe(listener: () => void) {
  listeners.add(listener)
  if (listeners.size === 1) window.addEventListener('storage', onStorage)
  restoreEditorLanguage()
  return () => { listeners.delete(listener); if (!listeners.size) window.removeEventListener('storage', onStorage) }
}
function onStorage(event: StorageEvent) { if (event.key === EDITOR_LANGUAGE_KEY) { language = event.newValue === 'en' ? 'en' : 'zh-CN'; notify() } }
export function useEditorLanguage() { return useSyncExternalStore(subscribe, getEditorLanguage, () => 'zh-CN' as const) }

/** UI copy only. Media names, subtitles, prompts and project content never pass through this lookup. */
export function editorText(source: string, values?: Record<string, string | number | undefined>): string
export function editorText(source: string | undefined, values?: Record<string, string | number | undefined>): string | undefined
export function editorText(source: string | undefined, values?: Record<string, string | number | undefined>): string | undefined {
  if (source === undefined) return source
  const catalog: Record<string, string> = getEditorLanguage() === 'zh-CN' ? messages : english
  const translated = catalog[source] ?? source
  return values ? translated.replace(/\{(\d+)\}/g, (token, key) => values[key] === undefined ? token : String(values[key])) : translated
}
