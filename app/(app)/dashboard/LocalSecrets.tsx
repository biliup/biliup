'use client'
import React, { useRef, useState } from 'react'
import useSWR from 'swr'
import { Avatar, Banner, Button, Empty, Form, Spin, Toast } from '@douyinfe/semi-ui'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { fetcher, put } from '@/app/lib/api-streamer'
import FormSnapshot from '@/app/ui/FormSnapshot'
import { errorMessage } from '@/app/lib/use-fleet'
import { pairLabel, useMe, type FleetPair } from '@/app/lib/use-me'
import { useBiliUsers } from '@/app/lib/use-streamers'
import { fieldMarksCss, LOCAL_SECRET_FIELDS, secretsPayload, type ConfigValues } from '@/app/lib/fleet-config'
import { PlatformPanels } from '@/app/ui/plugins'
import { FormPage } from '@/app/ui/shell'
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

const TITLE = '本机密钥'
const BACK = { href: '/dashboard', label: '空间配置' }

/**
 * `/dashboard/secrets`：受控节点上改 Cookie、密码的页面，空间配置页头的「本机密钥」进来。
 * 只显示本机密钥字段，保存时其余配置原样回传（secretsPayload）
 */
export default function LocalSecretsPage() {
  const { data: entity, error, isLoading, mutate } = useSWR<ConfigValues>('/v1/configuration', fetcher)
  const { biliUsers } = useBiliUsers()
  const { me, can } = useMe()
  const editable = can('config.edit')
  const managed = !!me?.fleet_node?.config
  const apiRef = useRef<FormApi>(undefined)
  const snapshotRef = useRef<ConfigValues>({})
  const busy = useRef(false)
  const [saving, setSaving] = useState(false)
  const [formKey, setFormKey] = useState(0)
  const [platform, setPlatform] = useState(PLATFORMS[0].key)

  const list = biliUsers.map((item) => ({
    value: item.value,
    label: (
      <>
        <Avatar size="extra-small" src={item.face} />
        <span style={{ marginLeft: 8 }}>{item.name}</span>
      </>
    ),
  }))

  const save = async (values: ConfigValues) => {
    if (busy.current || !entity) return
    busy.current = true
    setSaving(true)
    try {
      await put('/v1/configuration', { arg: secretsPayload(entity, snapshotRef.current, values) })
      Toast.success('本机密钥已保存')
      await mutate()
      setFormKey((key) => key + 1)
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      busy.current = false
      setSaving(false)
    }
  }

  if (isLoading || !entity) {
    return (
      <FormPage title={TITLE} back={BACK} fill>
        <div className={styles.center}>
          {error ? (
            <>
              <Empty title="加载失败" description={errorMessage(error)} />
              <Button onClick={() => mutate()}>重试</Button>
            </>
          ) : (
            <Spin size="large" />
          )}
        </div>
      </FormPage>
    )
  }

  return (
    <FormPage
      title={TITLE}
      description={
        editable
          ? 'Cookie、账号密码只存在这台机器上，不随控制面下发，控制面也看不到'
          : '只读：修改本机密钥需要超级管理员'
      }
      back={BACK}
      okText={editable ? '保存' : undefined}
      onOk={() => apiRef.current?.submitForm()}
      okLoading={saving}
      fill
    >
      {managed ? null : (
        <div className={styles.intro}>
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            description="这台机器的配置不由控制面管理，这些字段也可以直接在「空间配置」里改。"
          />
        </div>
      )}
      <div className={styles.formHost} data-field-scope={SCOPE}>
        <style>{fieldMarksCss(SCOPE, { show: LOCAL_SECRET_FIELDS })}</style>
        <Form
          key={formKey}
          className={dashboard.form}
          initValues={entity}
          disabled={!editable}
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
    </FormPage>
  )
}
