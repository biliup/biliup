'use client'
import React, { useEffect, useRef, useState } from 'react'
import EditTemplate from '@/app/upload-manager/edit/page'
import {
  Button,
  Form,
  Layout,
  Nav,
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
import { registerMediaQuery, responsiveMap } from '@/app/lib/utils'
import { IconPlusCircle, IconStar, IconGlobe } from '@douyinfe/semi-icons'
import useSWR from 'swr'
import { fetcher, put } from '@/app/lib/api-streamer'
import useSWRMutation from 'swr/mutation'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { useBiliUsers } from '../lib/use-streamers'
import styles from '../styles/dashboard.module.scss'

// 注册各平台组件
import Global from '../ui/plugins/global'
import Cookie from '../ui/plugins/cookie'
import Bilibili from '../ui/plugins/bilibili'
import CC from '../ui/plugins/cc'
import Douyin from '../ui/plugins/douyin'
import Douyu from '../ui/plugins/douyu'
import Huya from '../ui/plugins/huya'
import Twitch from '../ui/plugins/twitch'
import YouTube from '../ui/plugins/youtube'
import Developer from '../ui/plugins/developer'
import Twitcasting from '@/app/ui/plugins/twitcasting'
import Kilakila from '@/app/ui/plugins/kilakila'

const Dashboard: React.FC = () => {
  const { Header, Content } = Layout
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
      <Header
        style={{
          backgroundColor: 'var(--semi-color-bg-1)',
          position: 'sticky',
          top: 0,
          zIndex: 1,
        }}
      >
        <Nav
          header={
            <>
              <div
                style={{
                  backgroundColor: '#6b6c75ff',
                  borderRadius: 'var(--semi-border-radius-large)',
                  color: 'var(--semi-color-bg-0)',
                  display: 'flex',
                  // justifyContent: 'center',
                  padding: '6px',
                }}
              >
                <IconStar size="large" />
              </div>
              <h4 style={{ marginLeft: '12px' }}>空间配置</h4>
            </>
          }
          footer={
            <Button
              onClick={() => {
                formRef.current?.submitForm()
              }}
              icon={<IconPlusCircle />}
              theme="solid"
              style={{ marginRight: 10 }}
            >
              保存
            </Button>
          }
          mode="horizontal"
        ></Nav>
      </Header>
      <Content>
        <main className={styles.rootConfigPanel}>
          <div className={styles.main}>
            <div className={styles.content}>
              <Form
                className={styles.form}
                // key={formKey}
                initValues={entity}
                onSubmit={async values => {
                  try {
                    await trigger(values)
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
                    maxWidth: 965,
                    // marginLeft: 'auto',
                    // marginRight: 'auto',
                    margin: '10px auto 0 auto',
                  }}
                >
                  <TabPane tab="全局设置" itemKey="1">
                    {/* 全局设置 */}
                    <Global />
                  </TabPane>
                  <TabPane tab="各平台下载" itemKey="2">
                    {/* 各平台下载 */}
                    <div className={styles.framePlatformConfig}>
                      <div className={styles.frameInside}>
                        <div className={styles.group}>
                          <div className={styles.buttonOnlyIconSecond}>
                            <div
                              className={styles.lineStory}
                              style={{
                                color: 'var(--semi-color-bg-0)',
                                display: 'flex',
                              }}
                            >
                              <IconGlobe size="small" />
                            </div>
                          </div>
                        </div>
                        <p className={styles.meegoSharedWebSettin}>各平台下载设置</p>
                      </div>
                      <Collapse keepDOM style={{ width: '100%' }}>
                        {/* 哔哩哔哩 */}
                        <Bilibili entity={entity} list={list} />
                        {/* CC直播 */}
                        <CC />
                        {/* 抖音 */}
                        <Douyin entity={entity} />
                        {/* 斗鱼 */}
                        <Douyu />
                        {/* 虎牙 */}
                        <Huya entity={entity} />
                        {/* 克拉克拉 */}
                        <Kilakila />
                        {/* 老鼠台 */}
                        <Twitch entity={entity} />
                        {/* Twitcasting */}
                        <Twitcasting />
                        {/* 油管 */}
                        <YouTube entity={entity} />
                        {/* 饼干 */}
                        <Cookie entity={entity} list={list} />
                      </Collapse>
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
      </Content>
    </>
  )
}

export default Dashboard
