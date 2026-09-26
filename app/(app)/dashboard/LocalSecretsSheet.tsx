'use client'
import React, { useRef, useState } from 'react'
import { Banner, Button, Form, SideSheet, Toast } from '@douyinfe/semi-ui'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { put } from '@/app/lib/api-streamer'
import { useWindowWidth } from '@/app/lib/useIsMobile'
import FormSnapshot from '@/app/ui/FormSnapshot'
import { errorMessage } from '@/app/lib/use-fleet'
import { fieldMarksCss, LOCAL_SECRET_FIELDS, secretsPayload, type ConfigValues } from '@/app/lib/fleet-config'
import { PlatformPanels } from '@/app/ui/plugins'
import dashboard from '@/app/styles/dashboard.module.scss'
import styles from '../nodes/fleet-config.module.scss'

const SCOPE = 'local-secrets'

/** 「用户 Cookie」栏里的快手 Cookie 写的是 user.kuaishou_cookie，实际字段在顶层，这里单独给一个 */
function Kuaishou() {
  return (
    <Form.Input
      field="kuaishou_cookie"
      label="快手 Cookie（kuaishou_cookie）"
      style={{ width: '100%' }}
      fieldStyle={{ alignSelf: 'stretch', padding: 0 }}
      showClear
    />
  )
}

const PLATFORMS = [
  ...PlatformPanels.filter((p) =>
    ['bilibili', 'douyin', 'douyu', 'twitcasting', 'twitch', 'youtube', 'user'].includes(p.key),
  ),
  { key: 'kuaishou', name: '快手', Component: Kuaishou },
]


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

/** 受控节点上改 Cookie、密码的入口：只显示本机密钥字段，其余配置原样回传 */
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
  const width = useWindowWidth()
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
    <SideSheet
      visible
      title="本机密钥"
      width={Number.isFinite(width) ? Math.min(760, width) : 760}
      onCancel={onClose}
      footer={null}
      bodyStyle={{ padding: 0, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}
    >
      <div className={styles.body}>
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
        <div className={styles.footer}>
          <Button onClick={onClose}>取消</Button>
          <Button theme="solid" loading={saving} onClick={() => apiRef.current?.submitForm()}>
            保存
          </Button>
        </div>
      </div>
    </SideSheet>
  )
}
