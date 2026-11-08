import { apiFetch, fetcher, handleResponse } from '../api-streamer'
import { documentProblems, type EditorDocument } from './project'
import { editorText } from './i18n'

export interface ProjectSummary {
  id: number; session_id: number | null; name: string; revision: number; created_at: number; updated_at: number
}
export interface SavedProject extends ProjectSummary {
  document: EditorDocument
  history:{canUndo:boolean;canRedo:boolean;undoRevision:number|null;redoRevision:number|null;branchRevision:number}
}
export const projectsUrl = '/v1/editor/projects'
export interface SuggestionAcceptance {id:number;items:string[]}
export class ProjectSaveConflict extends Error {
  constructor(message = editorText('The project has a newer version. Your local edits are preserved.')) {
    super(message)
    this.name = 'ProjectSaveConflict'
  }
}
function checkDocument(document: EditorDocument): EditorDocument {
  const problems = documentProblems(document)
  if (problems.length) throw new Error(problems.map(p => p.message).join('；'))
  return document
}
export async function loadProject(id: number): Promise<SavedProject> {
  const saved = await fetcher(`${projectsUrl}/${id}`) as SavedProject
  checkDocument(saved.document)
  return saved
}
export async function loadRevision(id: number, revision: number): Promise<EditorDocument> {
  return checkDocument(await fetcher(`${projectsUrl}/${id}/revisions/${revision}`))
}
export async function stepProjectHistory(project:Pick<SavedProject,'id'|'revision'>,action:'undo'|'redo'|'jump',target?:number):Promise<SavedProject>{
  const response=await apiFetch(`${projectsUrl}/${project.id}/history`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({revision:project.revision,action,...(target!==undefined?{target}:{})})})
  await handleResponse(response)
  return response.json()
}

export async function saveProject(document: EditorDocument, previous?: Pick<SavedProject, 'id' | 'revision'>, generated?:ReadonlyArray<string>, history?:{label:string;kind:'edit'|'jump'}, suggestion?:SuggestionAcceptance): Promise<SavedProject> {
  const problems = documentProblems(document)
  if (problems.length) throw new Error(problems.map(p => p.message).join('；'))
  const res = await apiFetch(previous ? `${projectsUrl}/${previous.id}` : projectsUrl, {
    method: previous ? 'PUT' : 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(previous ? { revision: previous.revision, document, ...(generated?.length?{generated}:{}),...(history?{history}:{}),...(suggestion?{suggestion}:{}) } : document),
  })
  if (previous && res.status === 409) throw new ProjectSaveConflict()
  await handleResponse(res)
  return res.json()
}
export async function deleteProject(project: Pick<SavedProject, 'id' | 'revision'>): Promise<void> {
  await handleResponse(await apiFetch(`${projectsUrl}/${project.id}?revision=${project.revision}`, { method: 'DELETE' }))
}
