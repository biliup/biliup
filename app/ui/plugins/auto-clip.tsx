'use client'
import React, { useState } from 'react'
import useSWR from 'swr'
import {
  Button,
  Collapsible,
  Form,
  Switch,
  Tag,
  Toast,
  Typography,
  useFormApi,
  useFormState,
} from '@douyinfe/semi-ui'
import {
  IconAlertTriangle,
  IconChevronDown,
  IconChevronUp,
  IconMinusCircle,
  IconScissors,
  IconTickCircle,
  IconUploadError,
} from '@douyinfe/semi-icons'
import SectionTitle from '../../(app)/components/SectionTitle'
import { API_BASE, fetcher, handleResponse } from '../../lib/api-streamer'
import styles from '../../styles/dashboard.module.scss'
import { FormDialog } from '../shell'

type CheckStatus = 'ok' | 'warning' | 'failed' | 'skipped'

type Check = {
  status: CheckStatus
  elapsed_ms: number | null
  message: string
  capable: boolean | null
  http_status: number | null
}

type ProbeReport = {
  tested_at: number
  chat: Check
  vision: Check
  asr: Check
}

type AutoClipStatus = {
  key_source: 'env' | 'config' | null
}

export type AutoClipValues = Record<string, unknown> | null | undefined

const NUMBER_FIELDS = [
  'max_asr_minutes',
  'max_chat_tokens',
  'chat_timeout_secs',
  'asr_timeout_secs',
  'min_clip_secs',
  'max_clip_secs',
  'max_candidates',
]

/** 掩码里的省略号：后端只回显 key 的首尾，交回原样的掩码表示「不改」 */
const MASK_MARK = '…'

/**
 * 提交前整理 `auto_clip`：数字框清空时是空串，后端要 null；整块什么都没填（没用过这个功能）
 * 就去掉这个键，保存后的配置与以前一模一样。
 */
export function normalizeAutoClip(section: AutoClipValues): AutoClipValues {
  if (!section || typeof section !== 'object') return undefined
  const cleaned: Record<string, unknown> = { ...section }
  for (const key of NUMBER_FIELDS) {
    if (cleaned[key] === '' || cleaned[key] === undefined) cleaned[key] = null
  }
  const used = Object.values(cleaned).some(
    v => v !== null && v !== undefined && v !== '' && v !== false,
  )
  return used ? cleaned : undefined
}

function hostOf(url: unknown) {
  if (typeof url !== 'string' || !url.trim()) return ''
  try {
    return new URL(url.trim()).host
  } catch {
    return url.trim()
  }
}

/** 与后端保存时的整理一致：空白算没填，地址去掉结尾的 `/` */
function filled(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null
}

function normalUrl(value: unknown): string | null {
  const text = filled(value)
  return text === null ? null : text.replace(/\/+$/, '')
}

const isMask = (key: unknown) => typeof key === 'string' && key.includes(MASK_MARK)

/**
 * 两把 key 各自会发往哪些地址，与后端 `chat_key_destinations` / `asr_key_destinations` 一致：
 * 转写地址没填就用 chat 的；没单独填转写 key 时，chat 的 key 也会发往转写地址。
 */
function keyDestinations(section: Record<string, any>) {
  const base = normalUrl(section.base_url)
  const asrBase = normalUrl(section.asr_base_url) ?? base
  const chat = new Set<string>()
  if (base) chat.add(base)
  if (filled(section.asr_api_key) === null && asrBase) chat.add(asrBase)
  const asr = new Set<string>()
  if (asrBase) asr.add(asrBase)
  return { chat, asr }
}

/**
 * 交回掩码时后端会不会拒绝保存，拒绝时给出与后端相同的原因：服务器上没存过这把 key，
 * 或者这把 key 要发往的地址多出了保存时没有的。
 */
export function maskedKeyProblem(
  section: Record<string, any>,
  saved: Record<string, any>,
  which: 'chat' | 'asr',
): string | null {
  const field = which === 'chat' ? 'api_key' : 'asr_api_key'
  if (!isMask(section[field])) return null
  const label = which === 'chat' ? 'API key' : '转写 API key'
  if (filled(saved[field]) === null) return `${label} 是掩码，但服务器上没有保存过 key：请粘贴完整的 key`
  const allowed = keyDestinations(saved)[which]
  for (const target of keyDestinations(section)[which]) {
    if (!allowed.has(target)) return `改了接口地址后请重新填写完整的 ${label}：已保存的 key 不会发往新地址`
  }
  return null
}

const CHECK_LABELS: [keyof Pick<ProbeReport, 'chat' | 'vision' | 'asr'>, string][] = [
  ['chat', 'chat'],
  ['vision', '看图'],
  ['asr', '转写'],
]

function StatusIcon({ status }: { status: CheckStatus }) {
  switch (status) {
    case 'ok':
      return <IconTickCircle style={{ color: 'var(--semi-color-success)' }} aria-label="通过" />
    case 'warning':
      return <IconAlertTriangle style={{ color: 'var(--semi-color-warning)' }} aria-label="有限制" />
    case 'failed':
      return <IconUploadError style={{ color: 'var(--semi-color-danger)' }} aria-label="失败" />
    default:
      return <IconMinusCircle style={{ color: 'var(--semi-color-text-2)' }} aria-label="跳过" />
  }
}

const fieldStyle = { alignSelf: 'stretch', padding: 0 } as const

type Props = {
  /** 只读角色：表单整体禁用，不是表单字段的开关与按钮也要跟着禁用 */
  disabled?: boolean
  /** 服务器上保存的配置，用来判断 key 是不是原样的掩码、地址有没有改 */
  entity?: Record<string, any>
}

const AutoClip: React.FC<Props> = ({ disabled, entity }) => {
  const { values } = useFormState()
  const formApi = useFormApi()
  const [open, setOpen] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [testing, setTesting] = useState(false)
  const [report, setReport] = useState<ProbeReport | null>(null)
  const { data: status, mutate: refreshStatus } = useSWR<AutoClipStatus>(
    '/v1/auto-clip/status',
    fetcher,
  )

  const section = (values?.auto_clip ?? {}) as Record<string, any>
  const saved = (entity?.auto_clip ?? {}) as Record<string, any>
  const enabled = section.enabled === true
  const host = hostOf(section.base_url)
  const asrHost = hostOf(filled(section.asr_base_url) ?? section.base_url)
  const chatKeyProblem = maskedKeyProblem(section, saved, 'chat')
  const asrKeyProblem = maskedKeyProblem(section, saved, 'asr')
  // 清空了单独的转写 key 时，chat 的 key 会跟着发往转写地址，提示里说清楚是为什么
  const chatKeyFollowsAsr =
    !!chatKeyProblem && filled(section.asr_api_key) === null && filled(saved.asr_api_key) !== null

  const toggle = (on: boolean) => {
    if (on) setConfirming(true)
    else formApi.setValue('auto_clip.enabled', false)
  }

  const runTest = async () => {
    setTesting(true)
    try {
      const res = await fetch(`${API_BASE}/v1/auto-clip/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(normalizeAutoClip(section) ?? {}),
      })
      if (res.status === 400) {
        const body = await res.json().catch(() => null)
        throw new Error(body?.message ?? '请求不合法')
      }
      await handleResponse(res)
      const result: ProbeReport = await res.json()
      setReport(result)
      refreshStatus()
      const checks = [result.chat, result.vision, result.asr]
      if (checks.some(c => c.status === 'failed')) {
        Toast.error('测试未通过，按下面的提示检查配置')
      } else if (checks.some(c => c.status === 'warning')) {
        Toast.warning('能连通，但有需要注意的限制')
      } else if (checks.every(c => c.status === 'skipped')) {
        Toast.info('没有可测的项目：先填接口地址和模型')
      } else {
        Toast.success('连接正常')
      }
    } catch (e: any) {
      Toast.error(`测试失败：${e?.message ?? e}`)
    } finally {
      setTesting(false)
    }
  }

  const summary = enabled
    ? `已开启 · ${section.chat_model || '未填模型'}${host ? ` · ${host}` : ''}${
        asrHost && asrHost !== host ? ` · 转写 ${asrHost}` : ''
      }`
    : '未开启'

  return (
    <div className={styles.frameAutoClip} data-testid="auto-clip-section">
      <button
        type="button"
        className={styles.collapseHead}
        aria-expanded={open}
        aria-controls="auto-clip-body"
        onClick={() => setOpen(o => !o)}
      >
        <SectionTitle
          icon={<IconScissors size="small" />}
          title={
            <>
              自动切片 <Tag size="small" color="orange">实验</Tag>
            </>
          }
        />
        <span className={styles.collapseSummary}>{summary}</span>
        {open ? <IconChevronUp /> : <IconChevronDown />}
      </button>
      {/* 折叠时字段仍挂载：空间配置整表覆盖保存，卸载会丢键 */}
      <Collapsible isOpen={open} keepDOM>
        <div id="auto-clip-body" className={styles.autoClipBody}>
          <Typography.Paragraph type="tertiary" style={{ fontSize: 14 }}>
            用你配置的模型从录像里挑出候选片段（语音转写 + 弹幕 + 截图），接受后才变成切片草稿，不会自动导出或投稿。
            候选在回看页的「候选」里处理；可以在剪辑台「直播场次」或回看页手动「生成候选」，也可以在直播间编辑里打开
            「下播后自动生成候选」。只支持 OpenAI 兼容接口（/chat/completions 与
            /audio/transcriptions）；接口地址可以指向自建服务（whisper.cpp server、LocalAI、vLLM 等）。
          </Typography.Paragraph>

          <Form.Slot label="启用（auto_clip.enabled）" style={fieldStyle}>
            <Switch
              aria-label="启用自动切片"
              checked={enabled}
              disabled={disabled}
              onChange={toggle}
            />
          </Form.Slot>

          <Form.Input
            field="auto_clip.base_url"
            label="接口地址（base_url）"
            placeholder="https://api.openai.com/v1"
            extraText="一般以 /v1 结尾，biliup 会在后面接 /chat/completions。"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />
          <Form.Input
            field="auto_clip.api_key"
            label="API key（api_key）"
            placeholder="sk-..."
            autoComplete="off"
            extraText={
              <div style={{ fontSize: 14 }}>
                保存后只显示首尾（如 sk-…abcd），不改就原样保留；要换就整段粘贴新的。
                {status?.key_source === 'env' && (
                  <div>已设置环境变量 BILIUP_AUTO_CLIP_API_KEY，它优先于这里填的 key。</div>
                )}
                {chatKeyProblem && (
                  <div style={{ color: 'var(--semi-color-warning)' }} data-testid="auto-clip-key-retype">
                    {chatKeyProblem}
                    {chatKeyFollowsAsr ? '（清空了转写 API key，这把 key 也要发往转写地址）' : ''}。
                  </div>
                )}
              </div>
            }
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />
          <Form.Input
            field="auto_clip.chat_model"
            label="分析用的模型（chat_model）"
            placeholder="例如 gpt-4o-mini"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />

          <div className={styles.subTitle}>语音转写（可单独配置）</div>
          <Form.Input
            field="auto_clip.asr_base_url"
            label="转写接口地址（asr_base_url）"
            placeholder="留空 = 同上面的接口地址"
            extraText="只提供 chat 的服务大多没有 /audio/transcriptions，可以另填一家或自建的 whisper 服务。"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />
          <Form.Input
            field="auto_clip.asr_api_key"
            label="转写 API key（asr_api_key）"
            placeholder="留空 = 同上面的 API key"
            autoComplete="off"
            extraText={
              asrKeyProblem ? (
                <div
                  style={{ fontSize: 14, color: 'var(--semi-color-warning)' }}
                  data-testid="auto-clip-asr-key-retype"
                >
                  {asrKeyProblem}。
                </div>
              ) : undefined
            }
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />
          <Form.Input
            field="auto_clip.asr_model"
            label="转写模型（asr_model）"
            placeholder="例如 whisper-1"
            extraText="生成候选必填：先转写语音，候选的边界按分句时间定。转写服务不返回分句时间戳时边界会粗一些。"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />
          <Form.Input
            field="auto_clip.asr_language"
            label="语言（asr_language）"
            placeholder="留空自动识别，例如 zh"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />
          <Form.Input
            field="auto_clip.asr_prompt"
            label="热词（asr_prompt）"
            placeholder="主播名、游戏名、常见的梗"
            extraText="原样传给转写接口的 prompt，能提高专有名词的识别率。"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          />

          <div className={styles.subTitle}>截图与用量</div>
          <Form.Select
            field="auto_clip.thumbnails"
            label="随分析发送截图（thumbnails）"
            placeholder="自动（auto）"
            extraText="自动：「测试连接」确认模型能看图才发，每场最多 64 张。关掉可以少发一些画面给第三方。"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
            showClear
          >
            <Form.Select.Option value="auto">自动（auto）</Form.Select.Option>
            <Form.Select.Option value="on">开（on）</Form.Select.Option>
            <Form.Select.Option value="off">关（off）</Form.Select.Option>
          </Form.Select>
          <Form.InputNumber
            field="auto_clip.max_asr_minutes"
            label="每场转写上限（max_asr_minutes）"
            extraText="超过就不转写并提示，防止长场次费用失控。留空为 300 分钟。"
            min={1}
            precision={0}
            placeholder="默认 300"
            suffix="分钟"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />
          <Form.InputNumber
            field="auto_clip.max_chat_tokens"
            label="每场 chat 用量上限（max_chat_tokens）"
            extraText="输入加输出的 token 总数。留空为 300000。"
            min={1}
            precision={0}
            placeholder="默认 300000"
            suffix="token"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />
          <Form.InputNumber
            field="auto_clip.chat_timeout_secs"
            label="chat 请求超时（chat_timeout_secs）"
            extraText="单次请求的最长等待时间，留空为 180 秒。「测试连接」最多等 30 秒。"
            min={1}
            precision={0}
            placeholder="默认 180"
            suffix="秒"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />
          <Form.InputNumber
            field="auto_clip.asr_timeout_secs"
            label="转写请求超时（asr_timeout_secs）"
            extraText="单块音频的最长等待时间，留空为 300 秒。「测试连接」最多等 60 秒。"
            min={1}
            precision={0}
            placeholder="默认 300"
            suffix="秒"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />

          <div className={styles.subTitle}>候选</div>
          <Form.InputNumber
            field="auto_clip.min_clip_secs"
            label="候选最短（min_clip_secs）"
            extraText="告诉模型候选的长度范围，短于它的候选会被丢掉。留空为 15 秒。"
            min={1}
            precision={0}
            placeholder="默认 15"
            suffix="秒"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />
          <Form.InputNumber
            field="auto_clip.max_clip_secs"
            label="候选最长（max_clip_secs）"
            extraText="长于它的候选会被丢掉；比最短还短时按最短算。留空为 180 秒。"
            min={1}
            precision={0}
            placeholder="默认 180"
            suffix="秒"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />
          <Form.InputNumber
            field="auto_clip.max_candidates"
            label="每场最多候选数（max_candidates）"
            extraText="按模型自评从高到低保留这么多个，最多 200。留空为 20。"
            min={1}
            max={200}
            precision={0}
            placeholder="默认 20"
            suffix="个"
            style={{ width: '100%' }}
            fieldStyle={fieldStyle}
          />

          <div className={styles.autoClipTest}>
            <Button onClick={runTest} loading={testing} disabled={disabled}>
              测试连接
            </Button>
            <Typography.Text type="tertiary" size="small">
              用表单里当前的值测，不用先保存；会各发一次很小的请求（可能产生极少量费用）。
            </Typography.Text>
          </div>
          {report && (
            <ul className={styles.autoClipResults} aria-label="测试结果">
              {CHECK_LABELS.map(([key, label]) => {
                const check = report[key]
                return (
                  <li key={key} data-status={check.status}>
                    <StatusIcon status={check.status} />
                    <strong>{label}</strong>
                    {check.elapsed_ms !== null && (
                      <Typography.Text type="tertiary" size="small">
                        {check.elapsed_ms} ms
                      </Typography.Text>
                    )}
                    <span className={styles.autoClipMessage}>{check.message}</span>
                  </li>
                )
              })}
            </ul>
          )}
        </div>
      </Collapsible>
      <FormDialog
        title="开启自动切片（实验）"
        size="sm"
        visible={confirming}
        okText="开启"
        onOk={() => {
          formApi.setValue('auto_clip.enabled', true)
          setConfirming(false)
        }}
        onCancel={() => setConfirming(false)}
      >
        <div data-testid="auto-clip-privacy">
          {asrHost && asrHost !== host ? (
            <>
              开启后，生成候选时录像的音频（静音部分除外）会发送到转写接口
              <strong> {asrHost} </strong>
              ；弹幕摘要、转写文字和截图（每场最多 64 张，可关）会发送到分析接口
              <strong> {host || '你配置的接口地址'} </strong>
              。这两家服务按各自的条款处理和计费。
            </>
          ) : (
            <>
              开启后，生成候选时录像的音频（静音部分除外）、弹幕摘要、转写文字和截图（每场最多 64 张，可关）会发送到
              <strong> {host || '你配置的接口地址'} </strong>
              ，由该服务按它的条款处理和计费。
            </>
          )}
          只有手动「生成候选」或直播间打开了「下播后自动生成候选」时才会发送。确定开启吗？
        </div>
      </FormDialog>
    </div>
  )
}

export default AutoClip
