import type { Canvas, Item, Media, MediaProbe, ModuleManifest, Project, Problem } from '@/app/lib/editor/core/timeline/types'
import { validateProject } from '@/app/lib/editor/core/timeline/validate'
import { builtinModules } from '@/app/lib/editor/core/timeline/builtin-modules'
import type { SessionDetail, SegmentView } from '../sessions'
import type { Marker } from '../markers'

/** File names are display labels only. Media access must resolve these IDs on the server. */
export interface RecordingSource {
  kind: 'recording'
  sessionId: number
  segmentId: number
  viewSessionId: number
  viewStartMs: number
  sourceStartMs: number
  durationMs: number
  fileName: string
  state: SegmentView['state']
  hasDanmaku: boolean
}
export interface FileSource {
  kind: 'file'
  assetId: string
  durationMs: number | null
  fileName: string
}
export type MediaSource = RecordingSource | FileSource

export interface EditorDocument {
  format: 'biliup.editor/1'
  project: Project
  sources: Record<string, MediaSource>
  /** Optional entry-point context; recording ownership belongs to each registered source. */
  origin: { sessionId: number | null; inMs: number; outMs: number }
  /** Stable identities survive trimming/moving the imported markers on the project timeline. */
  sourceMarkers: { markerId: number; sessionId: number; atMs: number; viewAtMs: number; projectFrame: number; label: string }[]
  warnings: string[]
}

export const DEFAULT_CANVAS: Canvas = { width: 1920, height: 1080, fps: 30 }

export function emptyProject(name: string, canvas: Canvas = DEFAULT_CANVAS): EditorDocument {
  return {format:'biliup.editor/1',origin:{sessionId:null,inMs:0,outMs:0},sources:{},sourceMarkers:[],warnings:[],
    project:{format:'biliup.timeline/1',name:name.trim()||'未命名工程',canvas,media:[],tracks:[
      {id:'titles',kind:'text',name:'字幕与标题',items:[]},
      {id:'overlays',kind:'video',name:'叠加画面',items:[]},
      {id:'main',kind:'video',role:'main',name:'主视频',items:[]},
      {id:'audio',kind:'audio',name:'配音与音乐',items:[]},
    ]}}
}

/** JSON object key order changes when Rust serializes a save; it is not an edit. */
export function documentFingerprint(document: EditorDocument): string {
  return JSON.stringify(document, (_key, value) => value && typeof value === 'object' && !Array.isArray(value)
    ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b))) : value)
}

function finiteTime(value: number, label: string): number {
  if (!Number.isFinite(value) || value < 0 || value > Number.MAX_SAFE_INTEGER) throw new Error(`${label}无效`)
  return value
}

function validId(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`${label}无效`)
  return value
}

/** Quantize absolute boundaries once; summing rounded durations introduces drift over long recordings. */
export function msToFrame(ms: number, fps: Canvas['fps']): number {
  return Math.round(finiteTime(ms, '时间') * fps / 1000)
}

/** Uploads are probed against the saved canvas; a local draft may already use another frame rate. */
export function mediaForCanvas(media: Media, durationMs: number | null, canvas: Pick<Canvas, 'fps'>): Media {
  if (media.kind === 'image' || durationMs === null) return media
  return { ...media, probe: { ...media.probe, frames: Math.max(1, Math.floor(durationMs * canvas.fps / 1000)) } }
}

/** A queued import keeps the time the user chose even if the canvas frame rate changes. */
export function frameForCanvas(frame: number, sourceFps: number, canvas: Pick<Canvas, 'fps'>): number {
  if (!Number.isFinite(sourceFps) || sourceFps <= 0) throw new Error('来源帧率无效')
  return Math.round(finiteTime(frame, '时间轴位置') * canvas.fps / sourceFps)
}

export function editorProblems(project: unknown, modules: ReadonlyMap<string, ModuleManifest> = builtinModules): Problem[] {
  return validateProject(project, modules)
}

/** Bound edits by the imported recording snapshot, independently of source metadata.
 * One frame of tolerance covers independently quantized absolute boundaries. */
export function documentProblems(document: EditorDocument): Problem[] {
  const problems = editorProblems(document.project)
  if (problems.length) return problems
  for (const track of document.project.tracks) for (const item of track.items) {
    if (!item.media) continue
    const source = document.sources[item.media]
    if (!source) { problems.push({ code: 'source', path: item.id, message: '片段引用了未登记素材' }); continue }
    if (source.durationMs === null) continue
    const limit = Math.ceil(source.durationMs * document.project.canvas.fps / 1000) + 1
    if (!item.loop && !item.hold && (item.in ?? 0) + item.duration * (item.speed ?? 1) > limit) {
      problems.push({ code: 'source-range', path: item.id, message: `${item.name ?? item.id} 超出了导入录像的时长` })
    }
  }
  return problems
}

/**
 * Snapshot a biliup session into Biliup's portable timeline. Keep gaps and unavailable segments
 * in their original positions; never silently ripple later footage over lost recordings.
 * An imported clip's range is expressed in the *view* session, including merged sessions.
 */
export function projectFromSession(
  detail: SessionDetail,
  options: { canvas?: Canvas; markers?: Marker[]; range?: { inMs: number; outMs: number }; name?: string; probes?:Readonly<Record<string,MediaProbe>> } = {},
): EditorDocument {
  validId(detail.id, '场次 ID')
  const canvas = options.canvas ?? DEFAULT_CANVAS
  const endMs = detail.segments.reduce((end, s) => Math.max(end, s.end_ms ?? s.start_ms), detail.duration_ms)
  const inMs = finiteTime(options.range?.inMs ?? 0, '入点')
  const outMs = finiteTime(options.range?.outMs ?? endMs, '出点')
  if (outMs <= inMs || outMs > endMs) throw new Error('选区必须在已录制的场次范围内，且出点晚于入点')
  const media: Media[] = []
  const items: Item[] = []
  const sources: Record<string, RecordingSource> = {}
  const warnings: string[] = []
  const fps = canvas.fps
  let previousEnd = 0
  const ids = new Set<number>()

  for (const segment of [...detail.segments].sort((a, b) => a.start_ms - b.start_ms || a.id - b.id)) {
    validId(segment.id, '分段 ID')
    if (ids.has(segment.id)) throw new Error(`重复的录制分段 #${segment.id}`)
    ids.add(segment.id)
    finiteTime(segment.start_ms, '分段起点')
    const segmentEnd = finiteTime(segment.end_ms ?? segment.start_ms, '分段终点')
    if (segmentEnd < segment.start_ms) throw new Error(`分段 #${segment.id} 的时间范围无效`)
    const from = Math.max(inMs, segment.start_ms)
    const to = Math.min(outMs, segmentEnd)
    if (to <= from) continue
    const start = msToFrame(from - inMs, fps)
    const end = msToFrame(to - inMs, fps)
    if (end <= start) { warnings.push(`分段 #${segment.id} 短于一帧，未加入时间轴`); continue }
    if (start < previousEnd) throw new Error(`录制分段 #${segment.id} 与前一分段重叠，请先修复场次时间轴`)
    const owner = validId(segment.source_session_id ?? detail.id, '来源场次 ID')
    const member = detail.members?.find(m => m.source_session_id === owner)
    if (detail.kind === 'group' && !member) throw new Error(`合并场次缺少分段 #${segment.id} 的来源信息`)
    const sourceStartMs = segment.start_ms - (member?.offset_ms ?? 0)
    finiteTime(sourceStartMs, '来源分段起点')
    const id = `recording-${owner}-${segment.id}`
    sources[id] = {
      kind: 'recording', sessionId: owner, segmentId: segment.id, viewSessionId: detail.id,
      viewStartMs: segment.start_ms, sourceStartMs, durationMs: segmentEnd - segment.start_ms,
      fileName: segment.file_name, state: segment.state, hasDanmaku: segment.has_danmaku,
    }
    // Logical path, never an absolute filesystem path or a client-supplied URL.
    const probe=options.probes?.[id]
    media.push({ id, kind: 'video', name: segment.file_name, path: `recordings/${id}.${segment.container}`,
      probe:{...probe,frames:Math.max(1,Math.floor((segmentEnd-segment.start_ms)*fps/1000))} })
    items.push({ id: `clip-${owner}-${segment.id}`, type: 'clip', media: id, name: segment.file_name,
      in: msToFrame(from - segment.start_ms, fps), duration: end - start,
      ...(start > previousEnd ? { gap: start - previousEnd } : {}),
    })
    previousEnd = end
    if (segment.state === 'missing' || segment.state === 'deleted') warnings.push(`${segment.file_name} 的素材不可用，已保留其时间轴位置`)
    if (segment.state === 'recording') warnings.push(`${segment.file_name} 仍在录制，工程只引用导入时已录制的范围`)
  }
  if (!items.length) throw new Error('选区中没有可导入的录像分段')
  // Retain a trailing gap as an explicit black card, just as an interior gap stays empty.
  const totalFrames = msToFrame(outMs - inMs, fps)
  if (previousEnd < totalFrames) items.push({ id: 'trailing-gap', type: 'component', component: 'component.solid',
    name: '无录像区间', duration: totalFrames - previousEnd, props: { color: '#000000' } })
  const sourceMarkers = (options.markers ?? []).filter(m => m.at_ms >= inMs && m.at_ms <= outMs).map(m => {
    const sessionId = m.source_session_id ?? m.session_id
    const offset = detail.members?.find(member => member.source_session_id === sessionId)?.offset_ms ?? 0
    return { markerId: m.id, sessionId, atMs: m.at_ms - offset, viewAtMs: m.at_ms,
      projectFrame: Math.min(totalFrames - 1, msToFrame(m.at_ms - inMs, fps)), label: m.label }
  })
  const project: Project = {
    format: 'biliup.timeline/1', name: options.name?.trim() || detail.title || detail.streamer_name || `场次 ${detail.id}`,
    canvas, media,
    tracks: [
      { id: 'titles', kind: 'text', name: '字幕与标题', items: [] },
      { id: 'overlays', kind: 'video', name: '叠加画面', items: [] },
      { id: 'main', kind: 'video', role: 'main', name: '主视频', items },
      { id: 'audio', kind: 'audio', name: '配音与音乐', items: [] },
    ],
    markers: sourceMarkers.map(m => ({ frame: m.projectFrame, label: m.label })),
  }
  const problems = editorProblems(project)
  if (problems.length) throw new Error(problems.map(p => p.message).join('；'))
  return { format: 'biliup.editor/1', project, sources, origin: { sessionId: detail.id, inMs, outMs }, sourceMarkers, warnings }
}
