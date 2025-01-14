'use client'
import React from 'react'
import { Form, Select, Collapse } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
}

const Cookie: React.FC<Props> = props => {
  const entity = props.entity
  const list = props.list
  return (
    <>
      <Collapse.Panel header="用户 Cookie" itemKey="user">
        <Form.Input
          field="user.kuaishou_cookie"
          extraText={
            <div className="semi-form-field-extra">
              填入 Cookie 降低风控风险。 需要
              client_key,kuaishou.live.bfb1s,kuaishou.live.web_st,kuaishou.live.web_ph,userId 的值，
              请不要将所有 Cookie 填入。
            </div>
          }
          placeholder="client_key=none;kuaishou.live.bfb1s=none;kuaishou.live.web_st=none;kuaishou.live.web_ph=none;userId=none;"
          label="快手 Cookie（kuaishou_cookie）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.niconico-email"
          extraText="与您的 Niconico 账户相关联的电子邮件或电话号码。"
          label="ニコニコ動画 用户名（niconico-email）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.niconico-password"
          mode="password"
          extraText="您的 Niconico 账户的密码。"
          label="ニコニコ動画 密码（niconico-password）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.niconico-user-session"
          extraText="用户会话令牌的值，可作为提供密码的替代方法。"
          label="ニコニコ動画 用户会话令牌（niconico-user-session）"
          mode="password"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.niconico-purge-credentials"
          extraText="清除缓存的 Niconico 凭证，以启动一个新的会话并重新认证。"
          label="ニコニコ動画 清除凭证缓存（niconico-purge-credentials）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.afreecatv_username"
          extraText="您的 AfreecaTV 用户名。"
          label="AfreecaTV 用户名（afreecatv_username）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.afreecatv_password"
          extraText="您的 AfreecaTV 密码。"
          label="AfreecaTV 密码（afreecatv_password）"
          mode="password"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </Collapse.Panel>
    </>
  )
}

export default Cookie
