'use client'
import React, { useRef, useState } from 'react'
import { Banner, Form, Toast } from '@douyinfe/semi-ui'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { put } from '@/app/lib/api-streamer'
import FormSnapshot from '@/app/ui/FormSnapshot'
import { errorMessage } from '@/app/lib/use-fleet'
import { pairLabel, type FleetPair } from '@/app/lib/use-me'
import { fieldMarksCss, LOCAL_SECRET_FIELDS, secretsPayload, type ConfigValues } from '@/app/lib/fleet-config'
import { PlatformPanels } from '@/app/ui/plugins'
import { FormSheet } from '@/app/ui/shell'
import dashboard from '@/app/styles/dashboard.module.scss'
import styles from '../nodes/fleet-config.module.scss'

const SCOPE = 'local-secrets'

const PLATFORMS = PlatformPanels.filter((p) =>
  ['bilibili', 'douyin', 'douyu', 'twitcasting', 'twitch', 'youtube', 'user'].includes(p.key),
)


/** 空间配置页顶部：这台机器的配置由控制面管理 */
export function ManagedConfigBanner({ controller, canEditSecrets }: { controller: string; canEditSecrets: boolean }) {
  return (
    <Banner
      type="info"
      fullMode={false}
      closeIcon={null}
      style={{ marginBottom: 12 }}
      title={`这台机器已加入控制面 ${controller}，配置由控制面统一下发，这里只读`}
      description={
        canEditSecrets
          ? '下载、上传与各平台参数请到控制面「节点」页的「Fleet 配置」或「节点覆盖」里改。Cookie、密码等本机密钥不随控制面下发，点右上角「本机密钥」修改。'
          : '下载、上传与各平台参数请到控制面「节点」页修改。'
      }
    />
  )
}

/** 空间配置页顶部：一主一备里的节点，配置与对端双向同步、本机照常保存 */
export function PairConfigBanner({ controller, pair }: { controller: string; pair: FleetPair }) {
  return (
    <Banner
      type="info"
      fullMode={false}
      closeIcon={null}
      style={{ marginBottom: 12 }}
      title={`${pairLabel(pair)}：本机与 ${controller} 组成一主一备，这里的配置在两台之间双向同步`}
      description="在哪台保存都可以，同一项两边都改过时以后保存的为准；各平台的 Cookie 也同步。池大小、ffmpeg 路径、边录边传目录、最低可用空间、日志级别跟机器走，不同步。只有一台在线时改动先记在本机，对端回来后再同步。"
    />
  )
}

/**
 * 空间配置页上的「本机密钥」抽屉（空间配置的附属内容，`/dashboard?secrets=1`）：
 * 受控节点上改 Cookie、密码，只显示本机密钥字段，保存时其余配置原样回传（secretsPayload）
 */
export default function LocalSecretsSheet({
  entity,
  list,
  onClose,
  onSaved,
}: {
  entity: ConfigValues
  list: unknown
  onClose: () => void
  onSaved: () => void
}) {
  const apiRef = useRef<FormApi>(undefined)
  const snapshotRef = useRef<ConfigValues>({})
  const busy = useRef(false)
  const [saving, setSaving] = useState(false)
  const [platform, setPlatform] = useState(PLATFORMS[0].key)

  const save = async (values: ConfigValues) => {
    if (busy.current) return
    busy.current = true
    setSaving(true)
    try {
      await put('/v1/configuration', { arg: secretsPayload(entity, snapshotRef.current, values) })
      Toast.success('本机密钥已保存')
      onSaved()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      busy.current = false
      setSaving(false)
    }
  }

  return (
    <FormSheet
      title="本机密钥"
      size="md"
      fill
      onCancel={onClose}
      okText="保存"
      onOk={() => apiRef.current?.submitForm()}
      confirmLoading={saving}
    >
      <div className={styles.intro}>
        <Banner
          type="info"
          fullMode={false}
          closeIcon={null}
          description="Cookie、账号密码只存在这台机器上，不随控制面下发，控制面也看不到。"
        />
      </div>
      <div className={styles.formHost} data-field-scope={SCOPE}>
        <style>{fieldMarksCss(SCOPE, { show: LOCAL_SECRET_FIELDS })}</style>
        <Form
          className={dashboard.form}
          initValues={entity}
          getFormApi={(formApi) => (apiRef.current = formApi)}
          onSubmit={(values) => save(values)}
        >
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
              {PLATFORMS.map((p) => (
                <section key={p.key} className={dashboard.platformPanel} hidden={platform !== p.key} aria-label={p.name}>
                  <p.Component entity={entity} list={list} bare />
                </section>
              ))}
            </div>
          </div>
          <FormSnapshot snapshotRef={snapshotRef} />
        </Form>
      </div>
    </FormSheet>
  )
}
