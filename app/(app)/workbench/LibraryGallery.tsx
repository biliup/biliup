'use client'
import Link from 'next/link'
import { useState, type ReactNode } from 'react'
import { ArrowLeft, ChevronLeft, ChevronRight, Clapperboard, Clock3, Layers3, ListVideo, Scissors } from 'lucide-react'
import { type SessionSummary, formatSpan, replayHref } from '@/app/lib/sessions'
import { thumbUrl } from '@/app/lib/publish'
import { useVisibleElement } from '@/app/lib/use-visible-element'
import { usePoster } from '@/app/ui/editor/preview/resources'
import s from './library.module.scss'

const art = ['dusk', 'summit', 'tide', 'orbit']
const shortDate = new Intl.DateTimeFormat('zh-CN', { month: 'short', day: 'numeric' })
const longDate = new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: 'long', day: 'numeric' })
const pad = (n: number) => String(n).padStart(2, '0')
function clock(ms: number) {
  const sec = Math.floor(Math.max(0, ms) / 1000), h = Math.floor(sec / 3600)
  return `${h ? h + ':' : ''}${pad(Math.floor(sec / 60) % 60)}:${pad(sec % 60)}`
}

/** Only ask the frame service for visible covers. Use a stable timestamp even as a live session grows. */
function Artwork({ record, backdrop = false }: { record: SessionSummary; backdrop?: boolean }) {
  const { ref, active } = useVisibleElement<HTMLSpanElement>()
  const url = record.segment_count > 0 && record.duration_ms > 0
    ? thumbUrl(record.id, record.duration_ms > 1000 ? 1000 : 0, backdrop ? 1280 : 480) : undefined
  // Share the editor's bounded, two-request preview queue and blob cache between hero and tiles.
  const poster = usePoster(url, active).value?.src
  const [broken, setBroken] = useState<string>()
  const ready = !!poster && poster !== broken
  return <span ref={ref} className={s.artwork} data-ready={ready} aria-hidden="true" style={{ backgroundImage: `url(/library-art/${art[Math.abs(record.id) % art.length]}.svg)` }}>
    {!ready && <span className={s.noFrame}><Clapperboard/>暂无预览帧</span>}
    {/* These are authenticated local API frames, not optimizable remote assets. */}
    {/* eslint-disable-next-line @next/next/no-img-element */}
    {ready && <img src={poster} alt="" draggable={false} onError={() => setBroken(poster)} />}
  </span>
}

/** Unavailable segments keep their sequence number without issuing a frame request. */
export function SegmentPoster({ sessionId, at, index }: { sessionId: number; at?: number; index: number }) {
  const { ref, active } = useVisibleElement<HTMLSpanElement>()
  const src = usePoster(at == null ? undefined : thumbUrl(sessionId, at, 240), active).value?.src
  const [broken, setBroken] = useState<string>()
  return <span ref={ref} className={s.segmentPoster} aria-hidden="true">
    <Clapperboard size={18}/>
    {/* eslint-disable-next-line @next/next/no-img-element */}
    {src && src !== broken && <img src={src} alt="" onError={() => setBroken(src)}/>}
    <b>{pad(index + 1)}</b>
  </span>
}

export function SessionSpotlight({ records, detail, canEdit, onOpen, onBack }: {
  records: SessionSummary[]; detail: SessionSummary | null; canEdit: boolean
  onOpen: (record: SessionSummary) => void; onBack: () => void
}) {
  const [featuredId, setFeaturedId] = useState<number | null>(null)
  const index = Math.max(0, records.findIndex(record => record.id === featuredId))
  const record = detail ?? records[index]
  if (!record) return null
  const step = (by: number) => setFeaturedId(records[(index + by + records.length) % records.length].id)
  const edit = canEdit && (record.segment_count > 0 || record.kind === 'group')
  // A short strip of alternatives, not another full copy of the collection. All entries remain in the grid.
  const strip = records.length > 1 ? Array.from({ length: Math.min(3, records.length) }, (_, n) => records[(Math.floor(index / 3) * 3 + n) % records.length]) : []
  return <section className={s.spotlight} data-detail={!!detail} aria-label={detail ? '场次详情' : '场次精选'}>
    <Artwork key={record.id} record={record} backdrop/><div className={s.shade}/>
    <div className={s.heroCopy}>
      <div className={s.heroLabel}><Clapperboard size={17}/>{detail ? '场次详情' : '精选录像'}<i/>{record.streamer_name}</div>
      <h2 title={record.title}>{record.title || '无标题'}</h2>
      <div className={s.heroMeta}><span>{longDate.format(record.started_at)}</span><span>{formatSpan(record.duration_ms)}</span><span>{record.segment_count} 个分段</span></div>
      <div className={s.heroActions}>
        {detail ? <button type="button" className={s.heroPrimary} onClick={onBack}><ArrowLeft/>返回馆藏</button> : <button id="library-feature-open" type="button" className={s.heroPrimary} onClick={() => onOpen(record)}><ListVideo/>查看录像</button>}
        {edit && <Link href={replayHref(record.id)} className={s.heroSecondary}><Scissors/>进入剪辑</Link>}
      </div>
    </div>
    {!detail && strip.length > 0 && <div className={s.featureNav}>
      <div className={s.pager}><button type="button" aria-label="上一个精选场次" onClick={() => step(-1)}><ChevronLeft/></button><span>{pad(index + 1)} <b>/</b> {pad(records.length)}</span><button type="button" aria-label="下一个精选场次" onClick={() => step(1)}><ChevronRight/></button></div>
      <div className={s.strip} role="group" aria-label="切换精选录像">{strip.map(item => <button type="button" key={item.id} aria-pressed={record.id === item.id} aria-label={`聚焦录像：${item.title || '无标题'}`} onClick={() => setFeaturedId(item.id)}>
        <span className={s.stripArt}><Artwork record={item}/></span><span className={s.stripName}>{item.streamer_name}</span>
      </button>)}</div>
    </div>}
  </section>
}

export function SessionCard({ record, onOpen, selecting, selected, onSelect, actions, job }: {
  record: SessionSummary; onOpen: (record: SessionSummary) => void; selecting: boolean
  selected: boolean; onSelect: () => void; actions: ReactNode; job: ReactNode
}) {
  const title = record.title || '无标题'
  return <article className={s.card} data-selected={selecting && selected}>
    <button id={`library-card-${record.id}`} type="button" className={s.cardOpen} onClick={() => onOpen(record)} aria-label={`查看录像：${title}`}>
      <span className={s.cardArt}><Artwork record={record}/><span className={s.cardOverlay}/>
        <span className={s.cardBadge} data-live={record.recording}><i/>{record.recording ? '录制中' : '已结束'}</span>
        {record.kind === 'group' && <span className={s.groupBadge}><Layers3/>合并 · {record.source_session_ids?.length ?? 0} 场</span>}
        <span className={s.timecode}><Clock3/>{clock(record.duration_ms)}</span>
      </span>
      <strong className={s.cardTitle} title={title}>{title}</strong>
      <span className={s.cardMeta}><span>{record.streamer_name}</span><i/> {shortDate.format(record.started_at)} <i/> {record.segment_count} 段</span>
    </button>
    {job && <div className={s.cardJob}>自动切片 {job}</div>}
    {selecting && <div className={s.cardAdmin}>
      <label><input type="checkbox" aria-label={`选择场次：${title}`} checked={selected} disabled={record.kind === 'group' || record.group_session_id != null} onChange={onSelect}/>选择</label>{actions}
    </div>}
  </article>
}
