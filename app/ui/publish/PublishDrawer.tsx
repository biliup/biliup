'use client'
import React, { useEffect, useMemo, useRef, useState } from 'react'
import useSWR from 'swr'
import {
  Banner,
  Button,
  Cascader,
  Checkbox,
  DatePicker,
  Input,
  Radio,
  RadioGroup,
  Select,
  SideSheet,
  Slider,
  Spin,
  TagInput,
  TextArea,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui'
import { IconDelete, IconImage, IconLive, IconPlay, IconSend, IconUpload } from '@douyinfe/semi-icons'
import { fetcher, type StudioEntity } from '@/app/lib/api-streamer'
import { type Clip, updateClip, useFfmpeg } from '@/app/lib/clips'
import { formatSessionTime, ReportedError } from '@/app/lib/markers'
import { formatPrecise } from '@/app/lib/sessions'
import { useMe } from '@/app/lib/use-me'
import { useTypeTree } from '@/app/lib/use-streamers'
import { useWindowWidth } from '@/app/lib/useIsMobile'
import {
  ARCHIVE_MANAGER_URL,
  COVER_TYPES,
  type CoverSource,
  coverFromFile,
  coverFromFrame,
  coverFromLive,
  coverUrl,
  DTIME_MAX_DAYS,
  DTIME_MIN_HOURS,
  type Enqueued,
  MAX_ARCHIVE_TITLE,
  MAX_BATCH,
  MAX_COVER_BYTES,
  MAX_DESC,
  MAX_PARTS,
  MAX_TAG_CHARS,
  MAX_TAGS,
  needsConfirm,
  type PreviewArchive,
  previewPublish,
  publishBatch,
  publishClip,
  type PublishBody,
  removeCover,
  type StudioOverride,
  thumbUrl,
  usePublishQueue,
} from '@/app/lib/publish'
import { JobStatus, NO_SUBMIT, QueueBanner } from './JobStatus'
import styles from './publish.module.scss'

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

const DRAWER_WIDTH = 560
type DtimeMode = 'template' | 'now' | 'at'
const STREAMER_TEMPLATE = 'streamer'
const NO_EDIT = '改封面、保存发布设置需要 clip.edit 权限'

/** 表单里的覆盖值：空的字段表示沿用模板 */
interface Form {
  title: string
  desc: string
  tags: string[]
  tid: number | null
  tidV2: number | null
  dtimeMode: DtimeMode
  dtimeAt: Date | null
  cover: CoverSource | null
}

function formOf(over: StudioOverride | undefined): Form {
  const o = over ?? {}
  return {
    title: o.title ?? '',
    desc: o.desc ?? '',
    tags: o.tags ?? [],
    tid: o.tid ?? null,
    tidV2: o.tid_v2 ?? null,
    dtimeMode: o.dtime === undefined ? 'template' : o.dtime === 0 ? 'now' : 'at',
    dtimeAt: o.dtime ? new Date(o.dtime * 1000) : null,
    cover: o.cover ?? null,
  }
}

function dtimeProblem(form: Form, now: number): string | null {
  if (form.dtimeMode !== 'at') return null
  if (!form.dtimeAt) return '选一个定时发布的时间'
  const ms = form.dtimeAt.getTime() - now
  if (ms < DTIME_MIN_HOURS * 3_600_000) return `定时发布要在 ${DTIME_MIN_HOURS} 小时之后`
  if (ms > DTIME_MAX_DAYS * 86_400_000) return `定时发布最多在 ${DTIME_MAX_DAYS} 天之内`
  return null
}

function overrideOf(form: Form): StudioOverride {
  const over: StudioOverride = {}
  if (form.title.trim()) over.title = form.title.trim()
  if (form.desc.trim()) over.desc = form.desc
  if (form.tags.length) over.tags = form.tags
  if (form.tid !== null) {
    over.tid = form.tid
    if (form.tidV2 !== null) over.tid_v2 = form.tidV2
  }
  if (form.dtimeMode === 'now') over.dtime = 0
  if (form.dtimeMode === 'at' && form.dtimeAt) over.dtime = Math.floor(form.dtimeAt.getTime() / 1000)
  if (form.cover) over.cover = form.cover
  return over
}

function coverText(cover: CoverSource | null): string {
  if (!cover) return '上传模板里的封面（模板没设封面时由 B 站自动截取）'
  if (cover.source === 'frame') return `录像里 ${formatSessionTime(cover.t_ms)} 的画面`
  if (cover.source === 'live') return '开播时的直播间封面'
  return '上传的图片'
}

function localTime(unix: number): string {
  return new Date(unix * 1000).toLocaleString('zh-CN', { hour12: false })
}

function clipName(clip: Clip): string {
  return clip.title || `未命名切片 #${clip.id}`
}

type TypeNode = { label: string; value: number; children: { label: string; value: number }[] }

/** 分区在树里的路径（父分区、子分区）；Cascader 的值要完整路径 */
function tidPath(tree: TypeNode[] | undefined, tid: number | null): number[] | undefined {
  if (tid === null || !tree) return undefined
  const parent = tree.find((t) => t.children.some((c) => c.value === tid))
  return parent ? [parent.value, tid] : undefined
}

function tidName(tree: TypeNode[] | undefined, tid: number | null): string {
  if (tid === null) return '模板未设'
  const parent = tree?.find((t) => t.children.some((c) => c.value === tid))
  const child = parent?.children.find((c) => c.value === tid)
  return parent && child ? `${parent.label} / ${child.label}` : `#${tid}`
}

function Field({ label, hint, children }: { label: string; hint?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className={styles.field}>
      <span className={styles.fieldLabel}>{label}</span>
      {children}
      {hint ? <span className={styles.fieldHint}>{hint}</span> : null}
    </div>
  )
}

export interface PublishTarget {
  /** 要发布的切片（都属于同一场） */
  clips: Clip[]
  /** 从切片列表的多选进来 */
  batch: boolean
}

/**
 * 发布抽屉：回看页的切片卡片、集中发布，以及直播预览页「剪下刚才」的 Toast 都打开它。
 * `currentMs` 是播放器现在的画面（场次时间），只有回看页有，用来「用当前画面」做封面。
 */
export function PublishDrawer({
  target,
  onClose,
  currentMs,
}: {
  target: PublishTarget | null
  onClose: () => void
  currentMs?: (() => number) | null
}) {
  const width = useWindowWidth()
  const title = target
    ? target.batch
      ? `集中发布 ${target.clips.length} 个切片`
      : `发布「${clipName(target.clips[0])}」`
    : '发布'
  return (
    <SideSheet
      visible={target !== null}
      onCancel={onClose}
      title={<span data-publish-drawer-title="">{title}</span>}
      width={Math.min(DRAWER_WIDTH, width || DRAWER_WIDTH)}
      bodyStyle={{ padding: 0 }}
      closeOnEsc
    >
      {target ? (
        <DrawerBody
          key={target.clips.map((c) => c.id).join(',')}
          target={target}
          onClose={onClose}
          currentMs={currentMs ?? null}
        />
      ) : null}
    </SideSheet>
  )
}

/** 在切片范围里拖动选一帧做封面：预览图由服务器从录像取（`/thumb`），松手后再取，拖动中不请求 */
function FramePicker({
  clip,
  initial,
  busy,
  onPick,
  onCancel,
}: {
  clip: Clip
  initial: number
  busy: boolean
  onPick: (t: number) => void
  onCancel: () => void
}) {
  const { Text } = Typography
  const [t, setT] = useState(initial)
  const [shown, setShown] = useState(initial)
  const [failed, setFailed] = useState<number | null>(null)
  return (
    <div className={styles.framePicker} data-frame-picker="">
      <div className={styles.coverBox}>
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          key={shown}
          src={thumbUrl(clip.session_id, shown, 384)}
          alt={`录像里 ${formatPrecise(shown)} 的画面`}
          onError={() => setFailed(shown)}
          style={failed === shown ? { display: 'none' } : undefined}
        />
        {failed === shown ? <span>取不到这一帧</span> : null}
      </div>
      <div className={styles.frameRow}>
        <Slider
          className={styles.frameSlider}
          min={clip.in_ms}
          max={clip.out_ms}
          step={100}
          value={t}
          tipFormatter={(v) => formatPrecise(Number(v))}
          onChange={(v) => setT(Number(v))}
          onAfterChange={(v) => setShown(Number(v))}
          aria-label="选取封面的时间点"
        />
        <span className={styles.meta}>{formatPrecise(t)}</span>
      </div>
      {failed === shown ? (
        <Text type="danger" size="small">
          这个时间点取不到画面（可能落在断流空档里，或录像已被清理），换个时间点
        </Text>
      ) : null}
      <div className={styles.frameRow}>
        <Button size="small" theme="solid" loading={busy} disabled={t !== shown && busy} onClick={() => onPick(t)}>
          用这一帧
        </Button>
        <Button size="small" theme="borderless" type="tertiary" disabled={busy} onClick={onCancel}>
          取消
        </Button>
      </div>
    </div>
  )
}

function DrawerBody({
  target,
  onClose,
  currentMs,
}: {
  target: PublishTarget
  onClose: () => void
  currentMs: (() => number) | null
}) {
  const { Text } = Typography
  const { can, isLoading: meLoading } = useMe()
  const canSubmit = can('upload.submit')
  const canEdit = can('clip.edit')
  const ffmpeg = useFfmpeg()
  const first = target.clips[0]
  const sessionId = first.session_id
  const [combine, setCombine] = useState(false)
  /** 每个切片一个稿件、又选了好几个：标题等按各切片自己存着的设置，这里只能换模板 */
  const perClip = target.batch && !combine && target.clips.length > 1
  const [templateId, setTemplateId] = useState<number | null | undefined>(target.batch ? undefined : first.template_id)
  const [form, setForm] = useState<Form>(() => (target.batch ? formOf(undefined) : formOf(first.studio_override)))
  const [coverNonce, setCoverNonce] = useState(() => Date.now())
  const [coverBusy, setCoverBusy] = useState<string | null>(null)
  const [picking, setPicking] = useState(false)
  const [publishing, setPublishing] = useState(false)
  const [saving, setSaving] = useState(false)
  const [confirmed, setConfirmed] = useState(false)
  /** 服务端说有切片的投稿结果未知（打开抽屉后才变成这样） */
  const [serverUnknown, setServerUnknown] = useState<string | null>(null)
  const [result, setResult] = useState<Enqueued | null>(null)
  const [openedAt] = useState(() => Date.now())
  const fileRef = useRef<HTMLInputElement>(null)
  const patch = (p: Partial<Form>) => setForm((f) => ({ ...f, ...p }))

  const { data: templates } = useSWR<StudioEntity[]>('/v1/upload/streamers', fetcher)
  const { typeTree, isError: typeTreeError } = useTypeTree()
  const queue = usePublishQueue(sessionId, canSubmit || can('file.view'))

  const tooMany = target.clips.length > (combine ? MAX_PARTS : MAX_BATCH)
  const problem = dtimeProblem(form, openedAt)
  const body: PublishBody = useMemo(() => {
    const b: PublishBody = { clip_ids: target.clips.map((c) => c.id) }
    if (target.batch) b.combine = combine
    if (templateId !== undefined) b.template_id = templateId
    if (!perClip) b.studio_override = overrideOf(form)
    return b
  }, [target, combine, templateId, perClip, form])

  const [preview, setPreview] = useState<{ archives: PreviewArchive[] | null; error: string | null; loading: boolean }>({
    archives: null,
    error: null,
    loading: true,
  })
  const bodyKey = JSON.stringify(body)
  const previewOn = canSubmit && !tooMany && !problem && result === null
  useEffect(() => {
    if (!previewOn) return
    let cancelled = false
    const timer = window.setTimeout(() => {
      setPreview((p) => ({ ...p, loading: true }))
      previewPublish(JSON.parse(bodyKey))
        .then((archives) => !cancelled && setPreview({ archives, error: null, loading: false }))
        .catch((e) => !cancelled && setPreview({ archives: null, error: errorText(e), loading: false }))
    }, 350)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [bodyKey, previewOn])

  const archives = preview.archives ?? []
  const firstRendered = archives[0]?.rendered ?? null
  const blocking = archives.find((a) => a.problem)?.problem ?? null
  const tree: TypeNode[] | undefined = typeTree?.map(
    (type: { label: string; value: number; children: { id: number; name: string }[] }) => ({
      label: type.label,
      value: type.value,
      children: type.children.map((c) => ({ label: c.name, value: c.id })),
    })
  )

  const unknownClips = target.clips.filter(needsConfirm)
  const mustConfirm = unknownClips.length > 0 || serverUnknown !== null

  /** 合成多 P 时封面存在第一个切片上 */
  const coverClip = first
  const setCover = async (label: string, run: () => Promise<StudioOverride | void>) => {
    setCoverBusy(label)
    try {
      const over = await run()
      patch({ cover: over?.cover ?? null })
      setCoverNonce(Date.now())
      setPicking(false)
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `换封面失败：${errorText(e)}`, duration: 5 })
    } finally {
      setCoverBusy(null)
    }
  }
  const frameReason = !canEdit ? NO_EDIT : !ffmpeg.ready ? `取帧要用服务器上的 ffmpeg：${ffmpeg.reason ?? '不可用'}。可以上传图片` : null
  const pickFile = (file: File | undefined) => {
    if (!file) return
    if (!COVER_TYPES.includes(file.type)) {
      Toast.warning({ content: '只支持 JPEG、PNG、WebP 图片', duration: 4 })
      return
    }
    if (file.size > MAX_COVER_BYTES) {
      Toast.warning({ content: `封面图片最大 5 MB（这张 ${(file.size / 1048576).toFixed(1)} MB），压缩或裁小一点再传`, duration: 5 })
      return
    }
    void setCover('upload', () => coverFromFile(coverClip, file))
  }
  const frameStart =
    form.cover?.source === 'frame' && form.cover.t_ms >= coverClip.in_ms && form.cover.t_ms <= coverClip.out_ms
      ? form.cover.t_ms
      : Math.round((coverClip.in_ms + coverClip.out_ms) / 2)

  const localReason = meLoading
    ? '正在读取权限…'
    : !canSubmit
      ? NO_SUBMIT
      : tooMany
        ? combine
          ? `一个稿件最多 ${MAX_PARTS} 个分 P`
          : `一次最多发布 ${MAX_BATCH} 个切片`
        : problem
          ? problem
          : mustConfirm && !confirmed
            ? '先到 B 站稿件管理核对，勾选上面的确认'
            : null
  const submitReason =
    localReason ?? (preview.loading ? null : (blocking ?? (preview.error ? `预览失败：${preview.error}` : null)))

  const publish = async () => {
    setPublishing(true)
    const confirm = mustConfirm && confirmed ? { confirm_unknown: true } : {}
    try {
      const res = target.batch
        ? await publishBatch(sessionId, { ...body, ...confirm })
        : await publishClip(first, { template_id: body.template_id, studio_override: body.studio_override, ...confirm })
      setResult(res)
      Toast.success({
        content:
          res.jobs.length > 1
            ? `已把 ${res.jobs.length} 个稿件排进发布队列，一次传一个`
            : '已排进发布队列：需要时先导出，再上传、投稿',
        duration: 3,
      })
    } catch (e) {
      const text = errorText(e)
      if (/结果未知/.test(text)) {
        setServerUnknown(text)
        setConfirmed(false)
      } else if (!(e instanceof ReportedError)) {
        Toast.error({ content: `没能发布：${text}`, duration: 6 })
      }
    } finally {
      setPublishing(false)
    }
  }

  const save = async () => {
    setSaving(true)
    try {
      await updateClip(first, { template_id: templateId ?? null, studio_override: overrideOf(form) })
      Toast.success({ content: '发布设置已保存', duration: 2 })
      onClose()
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `保存失败：${errorText(e)}`, duration: 5 })
    } finally {
      setSaving(false)
    }
  }

  if (result) {
    const byId = new Map((queue.data?.jobs ?? []).map((j) => [j.id, j]))
    const names = new Map(target.clips.map((c) => [c.id, clipName(c)]))
    return (
      <div className={styles.drawer} data-publish-stage="submitted">
        <div className={styles.drawerScroll}>
          <QueueBanner paused={queue.data?.paused} canSubmit={canSubmit} />
          {result.skipped.length ? (
            <Banner
              type="warning"
              closeIcon={null}
              data-publish-skipped=""
              description={
                <span className={styles.confirm}>
                  <strong>{result.skipped.reduce((n, s) => n + s.clip_ids.length, 0)} 个切片没排进队列，其余照常排队</strong>
                  <ul className={styles.skippedList}>
                    {result.skipped.map((s) => (
                      <li key={s.clip_ids.join(',')}>
                        {s.clip_ids.map((id) => names.get(id) ?? `切片 #${id}`).join('、')}：{s.reason}
                      </li>
                    ))}
                  </ul>
                </span>
              }
            />
          ) : null}
          <Text type="tertiary" size="small">
            同一时间只传一个稿件。进度也显示在回看页的切片卡片上，关掉这里不影响发布。
          </Text>
          <ul className={styles.resultList} aria-label="发布进度">
            {result.jobs.map((j) => {
              const job = byId.get(j.id) ?? (queue.data ? null : j)
              const label = j.clip_ids.map((id) => names.get(id) ?? `切片 #${id}`).join(' + ')
              return (
                <li key={j.id} className={styles.resultItem}>
                  <span className={styles.resultTitle}>{job?.title ?? label}</span>
                  {job ? (
                    <JobStatus job={job} canSubmit={canSubmit} />
                  ) : (
                    <Text type="tertiary" size="small">
                      这个任务已经不在发布队列里（服务重启后队列会清空）。到切片卡片上看结果
                    </Text>
                  )}
                </li>
              )
            })}
          </ul>
        </div>
        <footer className={styles.drawerFoot}>
          <Button theme="solid" onClick={onClose}>
            关闭
          </Button>
        </footer>
      </div>
    )
  }

  const templateOptions = [
    { value: STREAMER_TEMPLATE, label: '主播绑定的模板' },
    ...(templates ?? []).map((t) => ({ value: t.id, label: t.template_name })),
  ]

  return (
    <div className={styles.drawer} data-publish-stage="form">
      <div className={styles.drawerScroll}>
        <QueueBanner paused={queue.data?.paused} canSubmit={canSubmit} />
        {mustConfirm ? (
          <Banner
            type="danger"
            closeIcon={null}
            data-publish-unknown=""
            description={
              <span className={styles.confirm}>
                <strong>上次投稿的结果未知，B 站那边可能已经有这个稿件</strong>
                <span>
                  {unknownClips.length
                    ? `${unknownClips.map(clipName).join('、')}：投稿请求发出后服务退出了（或没记上结果）。`
                    : '服务端提示有切片上次投稿的结果未知。'}
                  先到{' '}
                  <a href={ARCHIVE_MANAGER_URL} target="_blank" rel="noopener noreferrer">
                    B 站稿件管理
                  </a>{' '}
                  看有没有这个稿件；有就不要再发，否则会重复投稿。
                </span>
                <Checkbox checked={confirmed} onChange={(e) => setConfirmed(!!e.target.checked)}>
                  我已到 B 站稿件管理核对过，没有这个稿件，再发一次
                </Checkbox>
              </span>
            }
          />
        ) : null}
        {target.batch ? (
          <Field
            label="怎么发"
            hint={
              combine
                ? `按时间顺序合成一个稿件，每个切片一个分 P（最多 ${MAX_PARTS} 个）`
                : '每个切片单独一个稿件，排队依次上传'
            }
          >
            <RadioGroup
              type="button"
              value={combine ? 'combine' : 'each'}
              onChange={(e) => setCombine(e.target.value === 'combine')}
              aria-label="发布方式"
            >
              <Radio value="each">每个切片一个稿件</Radio>
              <Radio value="combine">合成一个多 P 稿件</Radio>
            </RadioGroup>
          </Field>
        ) : null}

        <Field
          label="上传模板"
          hint={
            templateId === undefined && target.batch
              ? '不改：每个切片用自己选过的模板，没选过的用主播绑定的'
              : firstRendered
                ? `现在用「${firstRendered.template_name}」的账号、分区、标签等`
                : '账号、分区、标签等默认取自模板'
          }
        >
          <Select
            value={templateId === undefined ? undefined : (templateId ?? STREAMER_TEMPLATE)}
            placeholder={target.batch ? '不改（各切片自己的设置）' : '主播绑定的模板'}
            optionList={templateOptions}
            onChange={(v) => setTemplateId(v === STREAMER_TEMPLATE ? null : (v as number))}
            style={{ width: '100%' }}
            aria-label="上传模板"
            showClear={target.batch}
            onClear={() => setTemplateId(undefined)}
          />
        </Field>

        {perClip ? (
          <Banner
            type="info"
            closeIcon={null}
            description="每个切片一个稿件时，标题、简介、封面等按各个切片自己的发布设置（在切片的「发布」里改）。要统一填写，请选「合成一个多 P 稿件」。"
          />
        ) : (
          <>
            <Field
              label="稿件标题"
              hint={
                <>
                  {form.title.trim()
                    ? '留空用模板的标题（没有切片变量时用切片名）'
                    : `留空用默认：${firstRendered ? `「${firstRendered.title}」` : '切片名'}`}
                  。可用变量 {'{clip_title}'} 切片名、{'{clip_time}'} 切片开始的时间、{'{streamer}'} 主播、
                  {'{title}'} 直播标题
                </>
              }
            >
              <Input
                value={form.title}
                maxLength={MAX_ARCHIVE_TITLE}
                showClear
                placeholder={firstRendered?.title ?? ''}
                onChange={(v) => patch({ title: v })}
                aria-label="稿件标题"
              />
            </Field>
            <Field label="简介" hint="留空用模板的简介；变量同上，{url} 是直播间地址">
              <TextArea
                value={form.desc}
                maxCount={MAX_DESC}
                autosize={{ minRows: 2, maxRows: 6 }}
                placeholder={firstRendered?.desc || '（模板没有简介）'}
                onChange={(v) => patch({ desc: v })}
                aria-label="简介"
              />
            </Field>
            <Field label="标签" hint={`留空用模板的标签；最多 ${MAX_TAGS} 个，每个最多 ${MAX_TAG_CHARS} 个字，回车或逗号分隔`}>
              <TagInput
                value={form.tags}
                max={MAX_TAGS}
                maxLength={MAX_TAG_CHARS}
                separator={[',', '，']}
                addOnBlur
                allowDuplicates={false}
                placeholder={firstRendered?.tags.join('、') || '（模板没有标签，至少填一个）'}
                onChange={(v) => patch({ tags: v })}
                aria-label="标签"
              />
            </Field>
            <Field
              label="分区"
              hint={
                typeTree || !typeTreeError
                  ? '留空用模板的分区'
                  : '读不到 B 站的分区列表（检查投稿账号的登录状态），先沿用模板的分区'
              }
            >
              <Cascader
                value={tidPath(tree, form.tid)}
                treeData={tree}
                placeholder={typeTree ? '沿用模板' : typeTreeError ? '沿用模板' : '正在读取 B 站分区…'}
                disabled={!typeTree}
                showClear
                onChange={(v) => {
                  const value = Array.isArray(v) ? v[v.length - 1] : v
                  patch({ tid: typeof value === 'number' ? value : null, tidV2: null })
                }}
                style={{ width: '100%' }}
                aria-label="分区"
              />
            </Field>
            <Field
              label="封面"
              hint={
                <>
                  {combine ? '合成的稿件用第一个切片的封面。' : ''}现在：{coverText(form.cover)}。上传的图片支持 JPEG / PNG /
                  WebP，最大 5 MB
                </>
              }
            >
              <div className={styles.coverRow}>
                <div className={styles.coverBox} data-cover={form.cover?.source ?? 'template'}>
                  {form.cover ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img src={coverUrl(coverClip.id, coverNonce)} alt="切片封面" />
                  ) : (
                    <span>模板封面</span>
                  )}
                  {coverBusy ? <Spin size="small" wrapperClassName={styles.coverSpin} /> : null}
                </div>
                <div className={styles.coverActions}>
                  <Tooltip content={frameReason ?? '在切片范围里拖动选一帧'}>
                    <span className={styles.inlineWrap}>
                      <Button
                        size="small"
                        icon={<IconImage />}
                        disabled={frameReason !== null || coverBusy !== null}
                        onClick={() => setPicking(true)}
                      >
                        从录像取帧
                      </Button>
                    </span>
                  </Tooltip>
                  {currentMs ? (
                    <Tooltip content={frameReason ?? '取播放器现在这一帧（先把画面停在想要的位置）'}>
                      <span className={styles.inlineWrap}>
                        <Button
                          size="small"
                          icon={<IconPlay />}
                          disabled={frameReason !== null || coverBusy !== null}
                          loading={coverBusy === 'current'}
                          onClick={() => setCover('current', () => coverFromFrame(coverClip, currentMs()))}
                        >
                          用当前画面
                        </Button>
                      </span>
                    </Tooltip>
                  ) : null}
                  <Tooltip content={canEdit ? 'JPEG / PNG / WebP，最大 5 MB' : NO_EDIT}>
                    <span className={styles.inlineWrap}>
                      <Button
                        size="small"
                        icon={<IconUpload />}
                        disabled={!canEdit || coverBusy !== null}
                        loading={coverBusy === 'upload'}
                        onClick={() => fileRef.current?.click()}
                      >
                        上传图片
                      </Button>
                    </span>
                  </Tooltip>
                  <Tooltip content={canEdit ? '开播时记下的直播间封面' : NO_EDIT}>
                    <span className={styles.inlineWrap}>
                      <Button
                        size="small"
                        icon={<IconLive />}
                        disabled={!canEdit || coverBusy !== null}
                        loading={coverBusy === 'live'}
                        onClick={() => setCover('live', () => coverFromLive(coverClip))}
                      >
                        直播间封面
                      </Button>
                    </span>
                  </Tooltip>
                  {form.cover ? (
                    <Button
                      size="small"
                      theme="borderless"
                      type="tertiary"
                      icon={<IconDelete />}
                      disabled={!canEdit || coverBusy !== null}
                      onClick={() => setCover('remove', () => removeCover(coverClip))}
                    >
                      改回模板封面
                    </Button>
                  ) : null}
                  <input
                    ref={fileRef}
                    type="file"
                    accept={COVER_TYPES.join(',')}
                    hidden
                    data-cover-file=""
                    onChange={(e) => {
                      pickFile(e.target.files?.[0])
                      e.target.value = ''
                    }}
                  />
                </div>
              </div>
              {picking ? (
                <FramePicker
                  clip={coverClip}
                  initial={frameStart}
                  busy={coverBusy === 'frame'}
                  onPick={(t) => setCover('frame', () => coverFromFrame(coverClip, t))}
                  onCancel={() => setPicking(false)}
                />
              ) : null}
            </Field>
            <Field
              label="发布时间"
              hint={
                form.dtimeMode === 'at'
                  ? (problem ?? `到 ${form.dtimeAt?.toLocaleString('zh-CN', { hour12: false })} 自动公开`)
                  : form.dtimeMode === 'now'
                    ? '审核通过后立即公开（不用模板里的定时）'
                    : firstRendered?.dtime
                      ? `模板设了定时：${localTime(firstRendered.dtime)} 公开`
                      : '模板没设定时：审核通过后立即公开'
              }
            >
              <RadioGroup
                type="button"
                value={form.dtimeMode}
                onChange={(e) => patch({ dtimeMode: e.target.value as DtimeMode })}
                aria-label="发布时间"
              >
                <Radio value="template">跟随模板</Radio>
                <Radio value="now">立即</Radio>
                <Radio value="at">定时</Radio>
              </RadioGroup>
              {form.dtimeMode === 'at' ? (
                <DatePicker
                  type="dateTime"
                  value={form.dtimeAt ?? undefined}
                  onChange={(d) => patch({ dtimeAt: d instanceof Date ? d : null })}
                  disabledDate={(d) =>
                    !!d &&
                    (d.getTime() < openedAt - 86_400_000 || d.getTime() > openedAt + DTIME_MAX_DAYS * 86_400_000)
                  }
                  format="yyyy-MM-dd HH:mm"
                  placeholder={`${DTIME_MIN_HOURS} 小时之后、${DTIME_MAX_DAYS} 天之内`}
                  style={{ width: '100%', marginTop: 6 }}
                  aria-label="定时发布的时间"
                />
              ) : null}
            </Field>
          </>
        )}

        <div className={styles.reprint} role="group" aria-label="版权">
          <span className={styles.reprintHead}>
            <span className={styles.fieldLabel}>版权</span>
            <RadioGroup value={2} disabled aria-label="版权（固定为转载）">
              <Radio value={1}>自制</Radio>
              <Radio value={2}>转载</Radio>
            </RadioGroup>
          </span>
          <span>
            切片是直播内容的片段，固定按转载投稿，不能改成自制
            {firstRendered?.template_self_made ? '（模板里选的是自制，切片不跟随）' : ''}。
          </span>
          <Input
            size="small"
            disabled
            value={firstRendered?.source ?? ''}
            placeholder="转载来源：直播间地址"
            prefix="来源"
            aria-label="转载来源"
          />
          <span className={styles.fieldHint}>来源默认是直播间地址；上传模板里填了转载来源时用模板的</span>
        </div>

        {canSubmit ? (
          <section className={styles.previewBox} aria-label="稿件预览" aria-busy={preview.loading}>
            <div className={styles.previewHead}>
              <strong>预览</strong>
              {preview.loading && previewOn ? <Spin size="small" /> : null}
            </div>
            {preview.error ? <Text type="danger">{preview.error}</Text> : null}
            {archives.map((a, i) => (
              <div key={a.clip_ids.join(',')} className={styles.previewItem}>
                {archives.length > 1 ? <span className={styles.meta}>稿件 {i + 1}</span> : null}
                {a.rendered ? (
                  <>
                    <span className={styles.previewTitle}>{a.rendered.title || '（标题是空的）'}</span>
                    <span className={styles.meta}>
                      {a.rendered.tags.join('、') || '没有标签'} · 分区 {tidName(tree, a.rendered.tid)} ·{' '}
                      {a.rendered.dtime ? `定时 ${localTime(a.rendered.dtime)}` : '审核后立即公开'} · 转载
                    </span>
                    {a.rendered.part_titles.length > 1 ? (
                      <ol className={styles.partList}>
                        {a.rendered.part_titles.map((t, j) => (
                          <li key={j}>{t}</li>
                        ))}
                      </ol>
                    ) : null}
                  </>
                ) : null}
                {a.problem ? (
                  <Text type="danger" size="small">
                    {a.problem}
                  </Text>
                ) : null}
              </div>
            ))}
          </section>
        ) : null}
      </div>

      <footer className={styles.drawerFoot}>
        {submitReason ? (
          <Text type="danger" size="small" className={styles.drawerReason} data-publish-reason="">
            {submitReason}
          </Text>
        ) : (
          <Text type="tertiary" size="small" className={styles.drawerReason}>
            提交后：导出（还没导出时快速剪）→ 上传 → 投稿；同一时间只传一个稿件
          </Text>
        )}
        {!target.batch ? (
          <Tooltip content={canEdit ? '只存设置，不发布' : NO_EDIT}>
            <span className={styles.inlineWrap}>
              <Button disabled={!canEdit || !!problem || publishing} loading={saving} onClick={save}>
                只保存
              </Button>
            </span>
          </Tooltip>
        ) : null}
        <Button
          theme="solid"
          icon={<IconSend />}
          disabled={submitReason !== null || preview.loading || saving}
          loading={publishing}
          onClick={publish}
        >
          {combine ? '发布多 P 稿件' : target.clips.length > 1 ? `发布 ${target.clips.length} 个稿件` : '发布'}
        </Button>
      </footer>
    </div>
  )
}
