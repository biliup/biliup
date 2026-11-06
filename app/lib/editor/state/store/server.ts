import type { EditorCommands } from '../../commands'
import type { Result, CallError } from '../connection'
import type { TakeKind } from '../../contracts'
import { set, currentEditorStore } from './core'
import { chooseEditorFiles, readDroppedFiles, type BrowserFileSelection } from '../../browser-imports'
import type {CaptureProgress} from '../../capture-upload'
import type {ProjectFontMetadata} from '../../project-fonts'
export type CommandService = <K extends keyof EditorCommands>(name: K, input: EditorCommands[K]['input'], timeoutMs?: number) => Promise<Result<EditorCommands[K]['output']>>
let service: CommandService | undefined
export const currentCommandService=()=>service
export interface FileImportResult {
  references: Array<{kind:'media'|'item'|'track';id:string}>
  fonts: string[]
  failed: Array<{path:string;message:string}>
  skipped: Array<{path:string;reason:string}>
  warnings?: Array<{path:string;message:string}>
}
export interface FileDrop { frame: number; fps: number; track?: string }
export interface FileImportOptions { drop?: FileDrop; fontMetadata?: ReadonlyMap<File,ProjectFontMetadata> }
export type FileImporter = (files: ReadonlyArray<File>, options?: FileImportOptions) => Promise<FileImportResult | undefined>
let fileImporter: FileImporter | undefined
export const currentFileImporter=()=>fileImporter
export type FileUploader = (file: File, options?: {kind?:TakeKind;dir?:string;agentId?:string;device?:string;onProgress?:(progress:CaptureProgress)=>void}) => Promise<{readonly path:string}|{readonly message:string}>
let fileUploader:FileUploader|undefined
export const currentUploadSession=()=>fileUploader
export function attachCommands(next: CommandService, importer?: FileImporter, uploader?:FileUploader) { service = next; fileImporter=importer; fileUploader=uploader; return () => { if (service === next) { service = undefined; fileImporter=undefined;fileUploader=undefined } } }
export const importFiles: FileImporter = (files, options) => fileImporter ? fileImporter(files, options) : Promise.resolve(undefined)
export function importFileSelection(selection: Promise<BrowserFileSelection>, drop?: FileDrop) {
  const importer = fileImporter
  const owner = currentEditorStore()
  if (!importer || !owner) return
  void (async () => {
    try {
      const picked = await selection
      const result = picked.files.length ? await importer(picked.files, {drop}) : undefined
      if (picked.problems.length) owner.set({importReport:{
        failed:[...(result?.failed??[]),...picked.problems.map(message=>({path:'导入素材',message}))],skipped:result?.skipped??[],warnings:result?.warnings,
      }})
    } catch (error) {
      owner.set({importReport:{failed:[{path:'导入素材',message:error instanceof Error?error.message:String(error)}],skipped:[]}})
    }
  })()
}
export function chooseFiles(directory=false) { importFileSelection(chooseEditorFiles(directory)) }
export function importDroppedFiles(transfer:DataTransfer,drop?:FileDrop) { importFileSelection(readDroppedFiles(transfer),drop) }
export const call: CommandService = (name, input, timeoutMs) => service ? service(name, input, timeoutMs) : Promise.resolve({ ok: false, error: { _tag: 'NotConnected', message: '剪辑工程尚未打开' } })
export const plainError = (error: CallError) => error.message
export const refreshRecents = async () => { const result = await call('recents.list', {}); if (result.ok) set({ recents: result.value }) }
export const uploadOne:FileUploader=(file,options)=>fileUploader?fileUploader(file,options):Promise.resolve({message:'剪辑工程尚未打开'})
