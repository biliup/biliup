/** Parse an ID or a millisecond position without accepting fractions or unsafe integers. */
export function parseWorkbenchNumber(raw: string | null, minimum = 0): number | null {
  if (raw === null || !/^\d+$/.test(raw)) return null
  const value = Number(raw)
  return Number.isSafeInteger(value) && value >= minimum ? value : null
}

function editorHref(key: 'project' | 'session' | 'selection', id: number, atMs?: number | null, endMs?:number|null): string {
  const params = new URLSearchParams({ [key]: String(id) })
  if (atMs != null && Number.isFinite(atMs)) params.set('t', String(Math.max(0, Math.round(atMs))))
  if (atMs != null && endMs != null && Number.isFinite(endMs) && endMs>atMs) params.set('end',String(Math.round(endMs)))
  return `/workbench?${params}`
}

/** Open a saved project. The position is measured on its edited timeline. */
export const projectEditorHref = (id: number, atMs?: number | null, endMs?:number|null) => editorHref('project', id, atMs,endMs)

/** Import/open a recording in the same editor. The position is measured on the source session. */
export const recordingEditorHref = (id: number, atMs?: number | null) => editorHref('session', id, atMs)

/** Resolve a saved live selection by identity, including its current in and out points. */
export const savedSelectionEditorHref = (id:number) => editorHref('selection',id)
