'use client'
import { useState } from 'react'
import useSWR from 'swr'
import { Banner, Button, Select, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import { IconServer } from '@douyinfe/semi-icons'
import { fetcher } from '@/app/lib/api-streamer'
import { errorMessage, type FleetNode } from '@/app/lib/use-fleet'
import {
  DEFAULT_PARAMS,
  HA_SINCE,
  PAIR_SINCE,
  candidatesKey,
  designate,
  joinPair,
  paramsIssue,
  type Candidates,
  type HaMode,
  type HaParams,
  type RowsView,
} from '@/app/lib/fleet-ha'
import ModeFields from './ModeFields'
import { AdoptionResult, RowPicker, defaultPicked, pickedCount, type Picked } from './RowPicker'
import { FormDialog } from '@/app/ui/shell'
import pageStyles from '../page.module.scss'
import styles from './ha.module.scss'

const { Text } = Typography

/** 这台节点能不能当备机；能当返回 null */
export function standbyIssue(node: FleetNode): string | null {
  if (node.removing) return '正在移除'
  if (!node.online) return '不在线：指定时要确认它的版本，等它连上'
  if ((node.proto ?? 0) < HA_SINCE) return `Fleet 协议版本是 ${node.proto ?? 0}，需要至少 ${HA_SINCE}：先升级 biliup`
  return null
}

type Draft = { standby: number; primary: Picked; node: Picked }
type Result = { standby: RowsView | null; primary: RowsView | null }

/**
 * 指定备机：主机是「本机」，选一台节点当备机、选模式，勾两台上要纳入配对的本地行。
 * 配对成功后留在「配对好了」的结果上，`onClose(true)` 时再换到面板
 */
export default function PairDialog({
  nodes,
  localNode,
  onClose,
  onPaired,
}: {
  nodes: FleetNode[]
  localNode: number | null
  onClose: (paired: boolean) => void
  onPaired: () => void
}) {
  const [standby, setStandby] = useState<number | null>(null)
  const [mode, setMode] = useState<HaMode>(1)
  const [params, setParams] = useState<HaParams>(DEFAULT_PARAMS)
  const [draft, setDraft] = useState<Draft | null>(null)
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<Result | null>(null)
  const { data, error, isLoading } = useSWR<Candidates>(
    standby !== null && localNode !== null ? candidatesKey(standby) : null,
    fetcher,
    { revalidateOnFocus: false },
  )
  const others = nodes.filter((n) => !n.local)
  const chosen = others.find((n) => n.id === standby)
  const picks: Draft | null =
    standby === null
      ? null
      : draft?.standby === standby
        ? draft
        : { standby, primary: defaultPicked(data?.primary), node: defaultPicked(data?.standby) }
  const issue = paramsIssue(params)
  const blocked = localNode === null || standby === null || !!issue || (!data && !error)

  const submit = async () => {
    if (busy || blocked || standby === null || !picks) return
    setBusy(true)
    try {
      const readable = data?.standby && !data.standby.error
      const res = await designate({
        standby,
        mode,
        params,
        adopt: readable ? picks.node : undefined,
      })
      let primary: RowsView | null = null
      if (pickedCount(picks.primary) > 0) {
        try {
          primary = await joinPair('controller', picks.primary)
        } catch (e) {
          primary = { node: localNode, error: `「本机」上的主播与模板没能加入：${errorMessage(e)}` }
        }
      }
      Toast.success({ id: 'fleet-ha-pair', content: `已配对：「本机」是主机，${chosen?.name ?? '节点'} 是备机` })
      setResult({ standby: res?.adoption ?? null, primary })
      onPaired()
    } catch (e) {
      Toast.error({ id: 'fleet-ha-pair', content: errorMessage(e), duration: 6 })
    } finally {
      setBusy(false)
    }
  }

  if (result) {
    return (
      <FormDialog
        title="配对好了"
        size="md"
        onCancel={() => onClose(true)}
        cancelText={null}
        okText="完成"
        onOk={() => onClose(true)}
      >
        <div className={pageStyles.dialogBody}>
          <Text type="tertiary" size="small">
            在录或上一场还没投完的，等这一场录完投完再加入；没纳入的照旧是那台自己的本地行，之后可以在面板上「加入配对」
          </Text>
          <AdoptionResult title={`${chosen?.name ?? '备机'}（备机）`} view={result.standby} />
          <AdoptionResult title="「本机」（主机）" view={result.primary} />
        </div>
      </FormDialog>
    )
  }

  return (
    <FormDialog
      title="设置一主一备"
      size="lg"
      closable={!busy}
      onCancel={() => onClose(false)}
      okText={picks ? `配对（纳入 ${pickedCount(picks.primary) + pickedCount(picks.node)} 项）` : '配对'}
      onOk={submit}
      confirmLoading={busy}
      okDisabled={blocked}
    >
      <div className={pageStyles.dialogBody}>
        <Text type="tertiary" size="small">
          两台录同样的房间，只有一台投稿，避免两份稿件。配对之后在哪台上改直播间、模板、空间配置、B 站账号都行，两台会同步
        </Text>
        {localNode === null ? (
          <Banner
            type="warning"
            fullMode={false}
            closeIcon={null}
            description="主机是控制面这台的「本机」节点：先在「节点」页启用「本机」，再来配对"
          />
        ) : null}
        <div className={styles.field}>
          <span className={styles.fieldLabel}>主机</span>
          <Text>
            <IconServer size="small" /> 「本机」（控制面这台）
          </Text>
        </div>
        <div className={styles.field}>
          <span className={styles.fieldLabel}>备机</span>
          <Select
            value={standby ?? undefined}
            placeholder={others.length ? '选一台节点' : '还没有别的节点'}
            disabled={busy || localNode === null}
            onChange={(v) => {
              setStandby(Number(v))
              setDraft(null)
            }}
            optionList={others.map((n) => {
              const why = standbyIssue(n)
              return {
                value: n.id,
                disabled: why !== null,
                label: (
                  <span className={pageStyles.nodeOption}>
                    <span className={pageStyles.nodeOptionName}>{n.name}</span>
                    {why ? (
                      <Text type="tertiary" size="small">
                        {why}
                      </Text>
                    ) : null}
                  </span>
                ),
              }
            })}
            style={{ width: '100%' }}
            aria-label="备机"
          />
          {chosen && (chosen.proto ?? 0) < PAIR_SINCE ? (
            <Text type="warning" size="small">
              它的 Fleet 协议版本是 {chosen.proto}，低于 {PAIR_SINCE}：两台不能双向同步，只能在控制面上改，它上面的本地行也不能纳入。建议先升级
            </Text>
          ) : null}
        </div>
        <ModeFields mode={mode} params={params} onMode={setMode} onParams={setParams} disabled={busy} />
        {standby !== null && localNode !== null ? (
          <div className={styles.field}>
            <span className={styles.fieldLabel}>纳入配对的本地行</span>
            <Text type="tertiary" size="small">
              两台上已有的直播间与投稿模板缺省都纳入，可以勾掉；主播用的模板会跟着一起纳入。勾掉的照旧是那台自己的，以后还能加
            </Text>
            {!data && isLoading ? (
              <div className={pageStyles.dialogCenter}>
                <Spin />
              </div>
            ) : error ? (
              <div className={styles.pickError}>读不到清单：{errorMessage(error)}</div>
            ) : picks ? (
              <>
                <RowPicker
                  title={chosen?.name ?? '备机'}
                  view={data?.standby}
                  picked={picks.node}
                  disabled={busy}
                  onChange={(node) => setDraft({ ...picks, node })}
                />
                <RowPicker
                  title="「本机」"
                  view={data?.primary}
                  picked={picks.primary}
                  disabled={busy}
                  onChange={(primary) => setDraft({ ...picks, primary })}
                />
              </>
            ) : null}
          </div>
        ) : null}
      </div>
    </FormDialog>
  )
}

/** 控制面没有配对时，节点页顶部的入口（弹层由页面持有：配对成功后这个入口就不显示了） */
export function PairEntry({ onOpen }: { onOpen: () => void }) {
  return (
    <section className={styles.entry} aria-label="一主一备">
      <IconServer className={styles.entryIcon} />
      <div className={styles.entryText}>
        <Text strong>一主一备</Text>
        <Text type="tertiary" size="small">
          让「本机」和一台节点录同样的房间、只由一台投稿；一台出问题另一台顶上
        </Text>
      </div>
      <Button onClick={onOpen}>设置一主一备</Button>
    </section>
  )
}
