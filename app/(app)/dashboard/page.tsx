'use client'
import React, { useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import {
  Button,
  Form,
  Avatar,
  Toast,
  Notification,
  Typography,
  Tabs,
  TabPane,
} from '@douyinfe/semi-ui'
import { IconPlusCircle, IconStar } from '@douyinfe/semi-icons'
import useSWR from 'swr'
import { fetcher, put } from '@/app/lib/api-streamer'
import useSWRMutation from 'swr/mutation'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { useBiliUsers } from '../../lib/use-streamers'
import { useMe } from '../../lib/use-me'
import styles from '../../styles/dashboard.module.scss'
import PageHeader from '../components/PageHeader'

// 注册各平台组件
import { PlatformPanels } from '../../ui/plugins'
import Global from '../../ui/plugins/global'
import Developer from '../../ui/plugins/developer'

const TAB_GLOBAL = '1'
const TAB_PLATFORM = '2'
const TAB_DEVELOPER = '3'

/** Semi 把校验错误按字段路径存成嵌套对象（{ user: { bili_cookie: '…' } }），拍平成 x-field-id 形式 */
function errorFieldPaths(errors: unknown, prefix = ''): string[] {
  if (!errors || typeof errors !== 'object' || Array.isArray(errors) || '$$typeof' in errors) {
    return prefix ? [prefix] : []
  }
  return Object.entries(errors as Record<string, unknown>).flatMap(([k, v]) =>
    errorFieldPaths(v, prefix ? `${prefix}.${k}` : k),
  )
}

const fieldElement = (path: string) =>
  document.querySelector<HTMLElement>(`.semi-form-field[x-field-id="${path}"]`)

const Dashboard: React.FC = () => {
const { data: entity, error, isLoading } = useSWR('/v1/configuration', fetcher)
  const { trigger } = useSWRMutation('/v1/configuration', put)
  const formRef = useRef<FormApi>(undefined)
  // const [formKey, setFormKey] = useState(0); // 初始化一个key
  // 触发表单重新挂载
  // const remountForm = () => {
  //     setFormKey((prevKey) => prevKey + 1); // 更新key的值
  // };

  // const [labelPosition, setLabelPosition] = useState<
  //     "top" | "left" | "inset"
  // >("inset");
  // useEffect(() => {
  //     const unRegister = registerMediaQuery(responsiveMap.lg, {
  //         match: () => {
  //             setLabelPosition("left");
  //         },
  //         unmatch: () => {
  //             setLabelPosition("top");
  //         },
  //     });
  //     return () => unRegister();
  // }, []);

  // useEffect(() => {
  //     remountForm();
  // }, [entity]);

  const { biliUsers } = useBiliUsers()
  const { can } = useMe()
  // 非超管拿到的是脱敏后的配置（凭据、账号 Cookie 等为空），只读展示，不能保存
  const editable = can('config.edit')

  // 平台设置：左列平台名是唯一的导航，右栏只显示选中平台的字段。列表来自插件注册表 PlatformPanels
  const [activePlatform, setActivePlatform] = useState(PlatformPanels[0].key)
  const [activeTab, setActiveTab] = useState(TAB_GLOBAL)

  // 校验失败时出错字段可能藏在未选中的 Tab / 平台面板里（字段全部挂载、只切 hidden），
  // 用户看不到红字也不知道为什么保存没反应。这里切到第一个出错字段所在的面板并滚过去。
  // 切面板的 state 更新用 flushSync 同步提交，之后字段已可见，直接滚动、聚焦即可，
  // 不再需要「记一个待定位字段 → effect 里滚动再清掉」的中间 state。
  const handleSubmitFail = (errors: Record<string, unknown>) => {
    const fields = errorFieldPaths(errors)
      .map(path => ({ path, el: fieldElement(path) }))
      .filter((f): f is { path: string; el: HTMLElement } => !!f.el)
      .sort((a, b) => (a.el.compareDocumentPosition(b.el) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1))
    const first = fields[0]
    if (!first) return
    const tab = first.el.closest<HTMLElement>('[data-tab]')?.dataset.tab
    const platform = first.el.closest<HTMLElement>('[data-platform]')?.dataset.platform
    flushSync(() => {
      if (tab) setActiveTab(tab)
      if (platform) setActivePlatform(platform)
    })
    first.el.scrollIntoView({ block: 'center', behavior: 'smooth' })
    first.el.querySelector<HTMLElement>('input, textarea')?.focus({ preventScroll: true })
    Toast.warning(`有 ${fields.length} 项未通过校验，已定位到第一项`)
  }

  if (isLoading) {
    return <>Loading</>
  }
  if (error) {
    return <> error {JSON.stringify(error)}</>
  }

  const list = biliUsers?.map(item => {
    return {
      value: item.value,
      label: (
        <>
          <Avatar size="extra-small" src={item.face} />
          <span style={{ marginLeft: 8 }}>{item.name}</span>
        </>
      ),
    }
  })
  // const handleSelectChange = (value) => {
  //         let text = value === 'male' ? 'Hi male' : 'Hi female!';
  //         formRef.current?.setValue('Note', text);
  //     };

  return (
    <>
      <PageHeader
        icon={<IconStar size="large" />}
        title="空间配置"
        description={
          editable
            ? '全局下载 / 上传参数、各平台录制参数与开发者选项。修改后需点击右上角「保存」才会生效'
            : '只读：当前角色可以查看配置，敏感字段已隐藏；修改需要超级管理员'
        }
        actions={
          editable && <Button
            onClick={() => {
              formRef.current?.submitForm()
            }}
            icon={<IconPlusCircle />}
            theme="solid"
          >
            保存
          </Button>
        }
      />
      {/* 页头下方占满剩余高度；每个 Tab 面板在内部滚动，页面本身不滚，「保存」始终可见 */}
      <div className={styles.page}>
        <Form
          className={styles.form}
          initValues={entity}
          disabled={!editable}
          onSubmit={async values => {
            try {
              const payload = { ...values }
              if (payload.file_size === undefined || payload.file_size === '') {
                payload.file_size = null
              }
              if (payload.segment_time === undefined || payload.segment_time === '') {
                payload.segment_time = null
              }
              if (payload.min_free_space === undefined || payload.min_free_space === '') {
                payload.min_free_space = null
              }
              // 后端是非负整数，不接受空值；清空即关闭
              if (
                payload.retention_hours === undefined ||
                payload.retention_hours === '' ||
                payload.retention_hours === null
              ) {
                payload.retention_hours = 0
              }
              await trigger(payload)
              Toast.success('保存成功')
            } catch (e: any) {
              // error handling
              Notification.error({
                title: '保存失败',
                content: <Typography style={{ maxWidth: 450 }}>{e.message}</Typography>,
                // theme: 'light',
                // duration: 0,
                style: { width: 'min-content' },
              })
              throw e
            }
          }}
          onSubmitFail={handleSubmitFail}
          getFormApi={formApi => (formRef.current = formApi)}
        >
          <Tabs type="line" className={styles.tabs} activeKey={activeTab} onChange={setActiveTab}>
            <TabPane tab="全局设置" itemKey={TAB_GLOBAL}>
              <div className={styles.pane} data-tab={TAB_GLOBAL}>
                <Global disabled={!editable} />
              </div>
            </TabPane>
            <TabPane tab="平台设置" itemKey={TAB_PLATFORM}>
              {/* 左列平台列表 + 右栏选中平台的字段，两栏并排、各自独立滚动 */}
              <div className={styles.platformLayout} data-tab={TAB_PLATFORM}>
                <nav className={styles.platformNav} aria-label="平台列表">
                  {PlatformPanels.map(p => (
                    <button
                      key={p.key}
                      type="button"
                      className={`${styles.platformNavItem} ${
                        activePlatform === p.key ? styles.platformNavItemActive : ''
                      }`}
                      aria-pressed={activePlatform === p.key}
                      onClick={() => setActivePlatform(p.key)}
                    >
                      {p.name}
                    </button>
                  ))}
                </nav>
                <div className={styles.platformBody}>
                  {/* 所有平台的字段始终保持挂载，只用 hidden 切换显示。
                      卸载会注销 Semi Form 字段状态，提交时仅剩挂载字段；后端 PUT /configuration
                      整表覆盖保存，会清空其他平台的参数与凭据 */}
                  {PlatformPanels.map(p => (
                    <section
                      key={p.key}
                      className={styles.platformPanel}
                      hidden={activePlatform !== p.key}
                      aria-label={p.name}
                      data-platform={p.key}
                    >
                      <p.Component entity={entity} list={list} bare />
                    </section>
                  ))}
                </div>
              </div>
            </TabPane>
            <TabPane tab="开发者选项" itemKey={TAB_DEVELOPER}>
              <div className={styles.pane} data-tab={TAB_DEVELOPER}>
                <Developer />
              </div>
            </TabPane>
          </Tabs>
        </Form>
      </div>
    </>
  )
}

export default Dashboard
