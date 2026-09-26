'use client'
import React, { useRef, useState } from 'react'
import useSWR, { mutate as revalidate } from 'swr'
import {
  Banner,
  Button,
  Empty,
  Form,
  Popconfirm,
  SideSheet,
  Spin,
  TabPane,
  Tabs,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { fetcher } from '@/app/lib/api-streamer'
import { humDate } from '@/app/lib/utils'
import { useWindowWidth } from '@/app/lib/useIsMobile'
import FormSnapshot from '@/app/ui/FormSnapshot'
import { errorMessage, FLEET_NODES_KEY, type FleetNode } from '@/app/lib/use-fleet'
import {
  changedKeys,
  DELIVERABLE_KEYS,
  fieldMarksCss,
  FLEET_CONFIG_HISTORY_KEY,
  FLEET_CONFIG_KEY,
  globalPayload,
  nodeConfigKey,
  overridePayload,
  PER_NODE_KEYS,
  saveFleetConfig,
  saveNodeOverride,
  type ConfigHistory,
  type ConfigValues,
  type FieldMarks,
  type FleetConfig,
  type NodeConfig,
} from '@/app/lib/fleet-config'
import { PlatformPanels } from '@/app/ui/plugins'
import Global from '@/app/ui/plugins/global'
import Developer from '@/app/ui/plugins/developer'
import dashboard from '@/app/styles/dashboard.module.scss'
import { ConfigSyncTag } from './NodeConfigStatus'
import styles from './fleet-config.module.scss'

const { Text } = Typography

export type ConfigTarget = { mode: 'global' } | { mode: 'override'; nodeId: number }

const TAB_GLOBAL = 'global'
const TAB_PLATFORM = 'platform'
const TAB_DEVELOPER = 'developer'
const TAB_HISTORY = 'history'
/** 「用户 Cookie」一栏全是本机密钥，不随 Fleet 下发 */
const PLATFORMS = PlatformPanels.filter((p) => p.key !== 'user')
const PER_NODE_TEXT = '池大小、ffmpeg 路径、边录边传目录、最低可用空间、日志级别'
const SWR_OPTIONS = { revalidateOnFocus: false }

const keyList = (keys: string[]) => keys.join('、')
const savedAt = (ms: number | null) => (ms ? humDate(Math.floor(ms / 1000)) : '')


function HistoryPane() {
  const { data, error, isLoading, mutate } = useSWR<ConfigHistory>(FLEET_CONFIG_HISTORY_KEY, fetcher, SWR_OPTIONS)
  if (isLoading) {
    return (
      <div className={styles.center}>
        <Spin />
      </div>
    )
  }
  if (!data) {
    return (
      <div className={styles.center}>
        <Empty title="加载失败" description={errorMessage(error)} />
        <Button onClick={() => mutate()}>重试</Button>
      </div>
    )
  }
  if (!data.versions.length) {
    return (
      <div className={styles.center}>
        <Empty title="还没有历史版本" description="保存一次全局配置后，这里会列出每一版改了什么" />
      </div>
    )
  }
  return (
    <div className={styles.history}>
      <Text type="tertiary" size="small">
        只读，保留最近 {data.keep} 版。每一版是保存时的完整共享配置，下面列出与上一版相比改了哪些键。
      </Text>
      {data.versions.map((version, index) => {
        const previous = data.versions[index + 1]
        const changed = previous ? changedKeys(previous.config, version.config) : null
        return (
          <section key={version.version} className={styles.version} aria-label={`第 ${version.version} 版`}>
            <div className={styles.versionHead}>
              <b>第 {version.version} 版</b>
              {index === 0 ? (
                <Tag size="small" color="green">
                  当前
                </Tag>
              ) : null}
              <Text type="tertiary" size="small">
                {savedAt(version.updated_at)}
                {version.updated_by !== null ? ` · 用户 #${version.updated_by}` : ''}
              </Text>
            </div>
            <Text size="small" className={styles.keys}>
              {changed === null
                ? '保留着的最早一版'
                : changed.length
                  ? `改了：${keyList(changed)}`
                  : '与上一版相同'}
            </Text>
          </section>
        )
      })}
    </div>
  )
}

function ConfigForm({
  scope,
  formKey,
  initValues,
  marks,
  editable,
  withHistory,
  withFfmpegPath,
  apiRef,
  snapshotRef,
  onSubmit,
}: {
  scope: string
  formKey: string
  initValues: ConfigValues
  marks: FieldMarks
  editable: boolean
  withHistory: boolean
  withFfmpegPath: boolean
  apiRef: React.MutableRefObject<FormApi | undefined>
  snapshotRef: React.MutableRefObject<ConfigValues>
  onSubmit: (values: ConfigValues) => void
}) {
  const [tab, setTab] = useState(TAB_GLOBAL)
  const [platform, setPlatform] = useState(PLATFORMS[0].key)
  return (
    <div className={styles.formHost} data-field-scope={scope}>
      <style>{fieldMarksCss(scope, marks)}</style>
      <Form
        key={formKey}
        className={dashboard.form}
        initValues={initValues}
        disabled={!editable}
        getFormApi={(formApi) => (apiRef.current = formApi)}
        onSubmit={(values) => onSubmit(values)}
        onSubmitFail={(errors) => {
          const count = Object.keys(errors ?? {}).length
          Toast.warning(`有 ${count} 项未通过校验，请检查标红的字段`)
        }}
      >
        <Tabs type="line" className={dashboard.tabs} activeKey={tab} onChange={setTab}>
          <TabPane tab="全局设置" itemKey={TAB_GLOBAL}>
            <div className={dashboard.pane}>
              <Global disabled={!editable} />
            </div>
          </TabPane>
          <TabPane tab="平台设置" itemKey={TAB_PLATFORM}>
            <div className={dashboard.platformLayout}>
              <nav className={dashboard.platformNav} aria-label="平台列表">
                {PLATFORMS.map((p) => (
                  <button
                    key={p.key}
                    type="button"
                    className={`${dashboard.platformNavItem} ${platform === p.key ? dashboard.platformNavItemActive : ''}`}
                    aria-pressed={platform === p.key}
                    onClick={() => setPlatform(p.key)}
                  >
                    {p.name}
                  </button>
                ))}
              </nav>
              <div className={dashboard.platformBody}>
                {/* 与空间配置页一样全部挂载、只切 hidden，否则没挂载的字段不在表单值里 */}
                {PLATFORMS.map((p) => (
                  <section
                    key={p.key}
                    className={dashboard.platformPanel}
                    hidden={platform !== p.key}
                    aria-label={p.name}
                  >
                    <p.Component entity={initValues} list={[]} bare />
                  </section>
                ))}
              </div>
            </div>
          </TabPane>
          <TabPane tab="开发者选项" itemKey={TAB_DEVELOPER}>
            <div className={dashboard.pane}>
              <Developer />
              {withFfmpegPath ? (
                <Form.Input
                  field="ffmpeg_path"
                  label="ffmpeg 路径（ffmpeg_path）"
                  placeholder="留空则用节点自己的设置"
                  extraText="这台节点上 ffmpeg 可执行文件的路径，保存后节点立即换用"
                  style={{ width: '100%' }}
                  fieldStyle={{ alignSelf: 'stretch', padding: 0 }}
                  showClear
                />
              ) : null}
            </div>
          </TabPane>
          {withHistory ? (
            <TabPane tab="历史版本" itemKey={TAB_HISTORY}>
              {tab === TAB_HISTORY ? <HistoryPane /> : null}
            </TabPane>
          ) : null}
        </Tabs>
        <FormSnapshot snapshotRef={snapshotRef} />
      </Form>
    </div>
  )
}

function Loading() {
  return (
    <div className={styles.center}>
      <Spin size="large" />
    </div>
  )
}

function LoadFailed({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  return (
    <div className={styles.center}>
      <Empty title="加载失败" description={errorMessage(error)} />
      <Button onClick={onRetry}>重试</Button>
    </div>
  )
}

/** 保存中不再响应第二次提交（连点、回车） */
function useSaving() {
  const busy = useRef(false)
  const [saving, setSaving] = useState(false)
  const run = async (task: () => Promise<void>) => {
    if (busy.current) return
    busy.current = true
    setSaving(true)
    try {
      await task()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      busy.current = false
      setSaving(false)
    }
  }
  return { saving, run }
}

function GlobalSheetBody({ canManage }: { canManage: boolean }) {
  const { data, error, isLoading, mutate } = useSWR<FleetConfig>(FLEET_CONFIG_KEY, fetcher, SWR_OPTIONS)
  const apiRef = useRef<FormApi>(undefined)
  const snapshotRef = useRef<ConfigValues>({})
  const { saving, run } = useSaving()

  const save = (values: ConfigValues) =>
    run(async () => {
      if (!data) return
      const saved = await saveFleetConfig(globalPayload(data.config, snapshotRef.current, values))
      if (saved.changed) {
        Toast.success(`已保存为第 ${saved.version} 版，正在下发到在线节点`)
      } else {
        Toast.info('内容没有变化，没有生成新版本')
      }
      await mutate(saved, { revalidate: false })
      revalidate(FLEET_CONFIG_HISTORY_KEY).catch(() => undefined)
      revalidate(FLEET_NODES_KEY).catch(() => undefined)
    })


  if (isLoading) return <Loading />
  if (!data) return <LoadFailed error={error} onRetry={() => mutate()} />

  return (
    <>
      <div className={styles.intro}>
        <div className={styles.meta}>
          {data.saved ? (
            <>
              <Tag size="small" color="blue">
                第 {data.version} 版
              </Tag>
              <span>
                {savedAt(data.updated_at)} 保存{data.updated_by !== null ? ` · 用户 #${data.updated_by}` : ''}
              </span>
            </>
          ) : (
            <Tag size="small" color="grey">
              还没保存过
            </Tag>
          )}
        </div>
        {data.saved ? null : (
          <Banner
            type="warning"
            fullMode={false}
            closeIcon={null}
            description="还没保存过全局配置，节点现在各用各的配置。下面是默认值，保存后共享字段会下发到所有在线节点并覆盖它们本机的值。"
          />
        )}
        <Banner
          type="info"
          fullMode={false}
          closeIcon={null}
          description={`带「按节点」标记的字段（${PER_NODE_TEXT}）跟机器走，全局配置不管，请在节点卡片的「节点覆盖」里设置。Cookie、密码等不在这里，留在各节点本机的「空间配置」里。`}
        />
      </div>
      <ConfigForm
        scope="fleet-global"
        formKey={`v${data.version}`}
        initValues={data.config}
        marks={{
          show: DELIVERABLE_KEYS,
          lock: PER_NODE_KEYS,
          badges: [{ keys: PER_NODE_KEYS, text: '按节点', tone: 'grey' }],
        }}
        editable={canManage}
        withHistory
        withFfmpegPath={false}
        apiRef={apiRef}
        snapshotRef={snapshotRef}
        onSubmit={save}
      />
      <div className={styles.footer}>
        <span className={styles.footerHint}>
          {canManage ? '保存后下发到所有在线节点' : '只读：修改需要「管理节点」权限'}
        </span>
        {canManage ? (
          <Button theme="solid" loading={saving} onClick={() => apiRef.current?.submitForm()}>
            保存
          </Button>
        ) : null}
      </div>
    </>
  )
}

function OverrideSheetBody({ node, canManage }: { node: FleetNode; canManage: boolean }) {
  const { data, error, isLoading, mutate } = useSWR<NodeConfig>(nodeConfigKey(node.id), fetcher, SWR_OPTIONS)
  const apiRef = useRef<FormApi>(undefined)
  const snapshotRef = useRef<ConfigValues>({})
  const { saving, run } = useSaving()
  const where = node.online ? `正在下发到 ${node.name}` : `${node.name} 上线后下发`

  const store = (override: ConfigValues, message: string) =>
    run(async () => {
      await saveNodeOverride(node.id, override)
      Toast.success(message)
      await mutate()
      revalidate(FLEET_NODES_KEY).catch(() => undefined)
    })

  const save = (values: ConfigValues) => {
    if (!data) return
    const global = data.global.saved ? data.global.config : null
    const override = overridePayload(data.override, snapshotRef.current, values, global)
    if (JSON.stringify(override) === JSON.stringify(data.override)) {
      Toast.info('覆盖没有变化')
      return
    }
    const count = Object.keys(override).length
    return store(override, count ? `已保存 ${count} 项覆盖，${where}` : `已撤掉全部覆盖，${where}`)
  }


  if (isLoading) return <Loading />
  if (!data) return <LoadFailed error={error} onRetry={() => mutate()} />

  const state = data.state
  const overrideKeys = Object.keys(data.override)
  return (
    <>
      <div className={styles.intro}>
        <div className={styles.meta}>
          {node.online ? <ConfigSyncTag state={state} /> : <Tag size="small">离线</Tag>}
          {state.outdated ? (
            <Tag size="small" color="orange">
              版本比控制面旧
            </Tag>
          ) : null}
          <span className={styles.keys}>{overrideKeys.length ? `已覆盖：${keyList(overrideKeys)}` : '没有覆盖，全部跟随全局'}</span>
        </div>
        {node.online && state.sync === 'failed' ? (
          <Banner
            type="danger"
            fullMode={false}
            closeIcon={null}
            title="节点没有应用最近下发的配置，仍按原来的配置运行"
            description={state.error ?? undefined}
          />
        ) : null}
        {node.online && state.sync === 'unsupported' ? (
          <Banner
            type="warning"
            fullMode={false}
            closeIcon={null}
            description="这台节点的版本只收房间、不收配置。覆盖照样保存，升级后自动生效。"
          />
        ) : null}
        <Banner
          type="info"
          fullMode={false}
          closeIcon={null}
          description={`只填这台节点与全局不同的项，留空或改回与全局相同就撤掉覆盖。${
            data.global.saved
              ? ''
              : '还没保存过全局配置，节点只收这里的覆盖，其余字段用它本机的值。'
          }带「按节点」标记的字段（${PER_NODE_TEXT}）控制面不知道节点本机的值，空着表示节点保留自己的。`}
        />
      </div>
      <ConfigForm
        scope={`fleet-node-${node.id}`}
        formKey={`${data.global.version}:${JSON.stringify(data.override)}`}
        initValues={data.global.saved ? data.delivered : data.override}
        marks={{
          show: DELIVERABLE_KEYS,
          badges: [
            { keys: overrideKeys, text: '已覆盖', tone: 'primary' },
            { keys: PER_NODE_KEYS.filter((key) => !overrideKeys.includes(key)), text: '按节点', tone: 'grey' },
          ],
        }}
        editable={canManage}
        withHistory={false}
        withFfmpegPath
        apiRef={apiRef}
        snapshotRef={snapshotRef}
        onSubmit={save}
      />
      <div className={styles.footer}>
        <span className={styles.footerHint}>{canManage ? '只保存与全局不同的项' : '只读：修改需要「管理节点」权限'}</span>
        {canManage ? (
          <>
            <Popconfirm
              title={`清空 ${node.name} 的覆盖？`}
              content="共享字段回到全局配置，按节点的字段回到它接入时本机的值"
              onConfirm={() => store({}, `已清空覆盖，${where}`)}
            >
              <Button type="danger" disabled={!overrideKeys.length || saving}>
                清空覆盖
              </Button>
            </Popconfirm>
            <Button theme="solid" loading={saving} onClick={() => apiRef.current?.submitForm()}>
              保存
            </Button>
          </>
        ) : null}
      </div>
    </>
  )
}

/** 节点页的「Fleet 配置」（全局）与节点卡片上的「节点覆盖」，表单复用空间配置页的组件 */
export default function FleetConfigSheet({
  target,
  node,
  canManage,
  onClose,
}: {
  target: ConfigTarget
  /** 覆盖模式下的节点；节点已被移除时为 undefined */
  node: FleetNode | undefined
  canManage: boolean
  onClose: () => void
}) {
  const width = useWindowWidth()
  let body: React.ReactNode
  if (target.mode === 'global') {
    body = <GlobalSheetBody canManage={canManage} />
  } else if (node) {
    body = <OverrideSheetBody node={node} canManage={canManage} />
  } else {
    body = (
      <div className={styles.center}>
        <Empty title="节点不存在或已被移除" />
      </div>
    )
  }
  return (
    <SideSheet
      visible
      title={target.mode === 'global' ? 'Fleet 配置' : `节点覆盖 · ${node?.name ?? ''}`}
      width={Number.isFinite(width) ? Math.min(960, width) : 960}
      onCancel={onClose}
      footer={null}
      bodyStyle={{ padding: 0, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}
    >
      <div className={styles.body}>{body}</div>
    </SideSheet>
  )
}
