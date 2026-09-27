'use client'
import React, { useState } from 'react'
import useSWR from 'swr'
import { Banner, Button, Checkbox, InputNumber, Modal, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import {
  type AutoClipStatus,
  autoClipError,
  fetchEstimate,
  sessionAutoClipUrl,
  startAutoClip,
} from '@/app/lib/auto-clip'
import { ReportedError } from '@/app/lib/markers'
import { formatSpan } from '@/app/lib/sessions'
import { useTextPref } from '@/app/lib/use-local-pref'
import styles from './auto-clip.module.scss'

/** 单价只存在这个浏览器里，只用来把预估换算成金额 */
const ASR_PRICE_KEY = 'biliup.autoClip.price.asrPerMinute'
const CHAT_PRICE_KEY = 'biliup.autoClip.price.chatPerMillion'

function price(raw: string): number | null {
  if (!raw.trim()) return null
  const n = Number(raw)
  return Number.isFinite(n) && n >= 0 ? n : null
}

const minutes = (seconds: number) => (seconds / 60).toFixed(seconds < 600 ? 1 : 0)

/**
 * 生成候选前的预估：要送转写的分钟数、chat 大约多少 token、每场上限，填了单价时换算成金额。
 * 超上限且预估已经确定（按静音表 / 按转写算）时不能确认，后端也会拒绝。
 */
export default function EstimateModal({
  sessionId,
  status,
  onClose,
}: {
  sessionId: number
  status: AutoClipStatus | undefined
  onClose: () => void
}) {
  const { Text } = Typography
  const [reuse, setReuse] = useState(true)
  const [hadTranscript, setHadTranscript] = useState(false)
  const [starting, setStarting] = useState(false)
  const [asrPrice, setAsrPrice] = useTextPref(ASR_PRICE_KEY)
  const [chatPrice, setChatPrice] = useTextPref(CHAT_PRICE_KEY)
  const { data, error, isLoading } = useSWR(
    [sessionAutoClipUrl(sessionId), 'estimate', reuse],
    () => fetchEstimate(sessionId, reuse),
    { revalidateOnFocus: false, shouldRetryOnError: false }
  )
  const estimate = data?.estimate ?? null
  if (estimate && estimate.transcribed_seconds > 0 && !hadTranscript) setHadTranscript(true)

  const blocked =
    !!estimate &&
    ((estimate.over_limit && estimate.basis === 'silence') ||
      (estimate.chat_over_limit && estimate.chat.basis === 'transcript'))
  const asrUnit = price(asrPrice)
  const chatUnit = price(chatPrice)
  const cost =
    estimate && (asrUnit !== null || chatUnit !== null)
      ? (asrUnit ?? 0) * (estimate.asr_seconds / 60) + (chatUnit ?? 0) * (estimate.chat.tokens / 1_000_000)
      : null

  const start = async () => {
    setStarting(true)
    try {
      await startAutoClip(sessionId, reuse)
      Toast.success({ content: '已开始生成候选，进度在状态里；做完后候选会出现在回看页的「候选」里', duration: 3 })
      onClose()
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `没能开始：${autoClipError(e)}`, duration: 5 })
    } finally {
      setStarting(false)
    }
  }

  const images = status?.thumbnails_active ? '截图' : null
  return (
    <Modal
      title="生成候选"
      visible
      onCancel={onClose}
      width={520}
      footer={
        <>
          <Button onClick={onClose}>取消</Button>
          <Button
            theme="solid"
            loading={starting}
            disabled={!estimate || blocked}
            onClick={start}
            data-testid="auto-clip-confirm"
          >
            确认生成
          </Button>
        </>
      }
    >
      {error ? (
        <Banner type="danger" fullMode={false} closeIcon={null} description={autoClipError(error)} />
      ) : isLoading || !estimate ? (
        <div className={styles.estimateLoading}>
          <Spin tip="正在估算…" />
        </div>
      ) : (
        <div className={styles.estimate} data-testid="auto-clip-estimate">
          <dl className={styles.estimateList}>
            <dt>送去转写</dt>
            <dd>
              <strong>约 {minutes(estimate.asr_seconds)} 分钟</strong>
              <Text type="tertiary" size="small">
                录像共 {formatSpan(estimate.recorded_seconds * 1000)}
                {estimate.basis === 'silence'
                  ? '，已跳过静音'
                  : '，还没抽过音频，先按录像时长算；跳过静音后实际会更少'}
                {estimate.transcribed_seconds > 0
                  ? `；已转写过的 ${formatSpan(estimate.transcribed_seconds * 1000)} 沿用，不再计费`
                  : ''}
              </Text>
            </dd>
            <dt>候选生成（chat）</dt>
            <dd>
              <strong>约 {estimate.chat.tokens.toLocaleString('zh-CN')} token</strong>
              <Text type="tertiary" size="small">
                分 {estimate.chat.windows} 窗问
                {estimate.chat.images > 0 ? `，含截图 ${estimate.chat.images} 张` : ''}
                {estimate.chat.basis === 'duration' ? '；还没转写，先按时长估' : ''}
              </Text>
            </dd>
            <dt>金额</dt>
            <dd>
              {cost !== null ? (
                <strong data-testid="auto-clip-cost">约 {cost.toFixed(cost < 1 ? 3 : 2)} 元</strong>
              ) : (
                <Text type="tertiary" size="small">
                  填上单价就能换算（只存在这个浏览器里，以服务商账单为准）
                </Text>
              )}
              <span className={styles.prices}>
                <InputNumber
                  size="small"
                  min={0}
                  value={asrPrice === '' ? undefined : Number(asrPrice)}
                  onNumberChange={(v) => setAsrPrice(v === undefined || Number.isNaN(v) ? '' : String(v))}
                  placeholder="转写单价"
                  suffix="元/分钟"
                  aria-label="转写单价（元/分钟）"
                  hideButtons
                />
                <InputNumber
                  size="small"
                  min={0}
                  value={chatPrice === '' ? undefined : Number(chatPrice)}
                  onNumberChange={(v) => setChatPrice(v === undefined || Number.isNaN(v) ? '' : String(v))}
                  placeholder="chat 单价"
                  suffix="元/百万 token"
                  aria-label="chat 单价（元/百万 token）"
                  hideButtons
                />
              </span>
            </dd>
          </dl>

          {estimate.over_limit && estimate.message ? (
            <Banner
              type={estimate.basis === 'silence' ? 'danger' : 'warning'}
              fullMode={false}
              closeIcon={null}
              description={estimate.message}
            />
          ) : null}
          {estimate.chat_over_limit && estimate.chat_message ? (
            <Banner
              type={estimate.chat.basis === 'transcript' ? 'danger' : 'warning'}
              fullMode={false}
              closeIcon={null}
              description={estimate.chat_message}
            />
          ) : null}

          <Text type="tertiary" size="small" className={styles.limitNote}>
            每场上限：转写 {estimate.max_asr_minutes} 分钟、chat {estimate.max_chat_tokens.toLocaleString('zh-CN')}{' '}
            token。超过时任务在调用模型之前停下并提示，不会产生超出上限的费用；上限在「空间配置」→「自动切片（实验）」里改。
          </Text>
          {status ? (
            <Text type="tertiary" size="small" className={styles.limitNote}>
              音频发往 {status.asr_host ?? '转写接口'}
              {status.asr_model ? `（${status.asr_model}）` : ''}；弹幕摘要、转写文字{images ? '和截图' : ''}发往{' '}
              {status.api_host ?? 'chat 接口'}
              {status.chat_model ? `（${status.chat_model}）` : ''}。
            </Text>
          ) : null}
          {hadTranscript ? (
            <Checkbox checked={!reuse} onChange={(e) => setReuse(!e.target.checked)} className={styles.retranscribe}>
              从头重新转写（改了转写模型或热词时用；会重新计转写费）
            </Checkbox>
          ) : null}
        </div>
      )}
    </Modal>
  )
}
