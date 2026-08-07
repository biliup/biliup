'use client'
import React, { useRef, useState } from 'react'
import {
  Button,
  Form,
  Collapse,
  Avatar,
  Select,
  Space,
  Toast,
  Notification,
  Typography,
  Tabs,
  TabPane,
} from '@douyinfe/semi-ui'
import { IconPlusCircle, IconStar, IconGlobe } from '@douyinfe/semi-icons'
import useSWR from 'swr'
import { fetcher, put } from '@/app/lib/api-streamer'
import useSWRMutation from 'swr/mutation'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { useBiliUsers } from '../../lib/use-streamers'
import styles from '../../styles/dashboard.module.scss'
import PageHeader from '../components/PageHeader'
import SectionTitle from '../components/SectionTitle'
import dc from '@/app/ui/data-card.module.scss'

// 注册各平台组件
import plugins from '../../ui/plugins'
import Global from '../../ui/plugins/global'
import Developer from '../../ui/plugins/developer'


const Dashboard: React.FC = () => {
const { data: entity, error, isLoading } = useSWR('/v1/configuration', fetcher)
  const { trigger } = useSWRMutation('/v1/configuration', put)
  const formRef = useRef<FormApi>()
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

  // 平台设置：左列平台名 + 右栏仅渲染选中平台（默认哔哩哔哩）
  // key 与各平台插件 Collapse.Panel 的 itemKey 对齐，便于点击即展开
  const [activePlatform, setActivePlatform] = useState('bilibili')
  const PLATFORM_LIST = [
    { key: 'bilibili', name: '哔哩哔哩', Comp: plugins.Bilibili },
    { key: 'cc', name: 'CC', Comp: plugins.CC },
    { key: 'douyin', name: '抖音', Comp: plugins.Douyin },
    { key: 'douyu', name: '斗鱼', Comp: plugins.Douyu },
    { key: 'huya', name: '虎牙', Comp: plugins.Huya },
    { key: 'kilakila', name: '克拉克拉', Comp: plugins.Kilakila },
    { key: 'twitcasting', name: 'TwitCasting', Comp: plugins.Twitcasting },
    { key: 'twitch', name: 'Twitch', Comp: plugins.Twitch },
    { key: 'youtube', name: 'YouTube', Comp: plugins.Youtube },
  ]
  const COOKIE_ENTRY = { key: 'user', name: '用户 Cookie', Comp: plugins.Cookie }
  const activeEntry =
    activePlatform === 'user'
      ? COOKIE_ENTRY
      : PLATFORM_LIST.find(p => p.key === activePlatform) ?? PLATFORM_LIST[0]
  const ActiveComp = activeEntry.Comp

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
        description="管理全局下载、各平台与上传账号"
        actions={
          <Button
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
      <div className={dc.content}>
        <main className={styles.rootConfigPanel}>
          <div className={styles.main}>
            <div className={styles.content}>
              <Form
                className={styles.form}
                // key={formKey}
                initValues={entity}
                onSubmit={async values => {
                  try {
                    const payload = { ...values }
                    if (payload.file_size === undefined || payload.file_size === '') {
                      payload.file_size = null
                    }
                    if (payload.segment_time === undefined || payload.segment_time === '') {
                      payload.segment_time = null
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
                getFormApi={formApi => (formRef.current = formApi)}
              >
                <Tabs
                  type="line"
                  contentStyle={{
                    margin: '10px 0 0 0',
                  }}
                >
                  <TabPane tab="全局设置" itemKey="1">
                    {/* 全局设置 */}
                    <Global />
                  </TabPane>
                  <TabPane tab="平台设置" itemKey="2">
                    {/* 平台设置：左列平台名 + 右栏选中表单 */}
                    <div className={styles.framePlatformConfig}>
                      <SectionTitle icon={<IconGlobe size="small" />} title="平台设置" />
                      <div className={styles.platformLayout}>
                        <nav className={styles.platformNav}>
                          {PLATFORM_LIST.concat(COOKIE_ENTRY).map(p => (
                            <button
                              key={p.key}
                              type="button"
                              className={`${styles.platformNavItem} ${
                                activePlatform === p.key ? styles.platformNavItemActive : ''
                              }`}
                              onClick={() => setActivePlatform(p.key)}
                            >
                              {p.name}
                            </button>
                          ))}
                        </nav>
                        <div className={styles.platformBody}>
                          <Collapse key={activePlatform} defaultActiveKey={[activePlatform]}>
                            <ActiveComp key={activePlatform} entity={entity} list={list} />
                          </Collapse>
                        </div>
                      </div>
                    </div>
                  </TabPane>
                  <TabPane tab="开发者选项" itemKey="3">
                    {/* 开发者选项 */}
                    <Developer />
                  </TabPane>
                </Tabs>
                <Space />
                <Space style={{ height: '160px' }} />
              </Form>
            </div>
          </div>
        </main>
      </div>
    </>
  )
}

export default Dashboard
