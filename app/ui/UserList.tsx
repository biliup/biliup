import React, { useEffect, useRef, useState } from 'react'
import {
  fetcher,
  LiveStreamerEntity,
  proxy,
  requestDelete,
  sendRequest,
  StudioEntity,
  User,
} from '../lib/api-streamer'
import useSWR from 'swr'
import { useRouter } from 'next/router'
import {
  Button,
  Form,
  List,
  Modal,
  Notification,
  Radio,
  RadioGroup,
  Row,
  SideSheet,
  Toast,
  Typography,
} from '@douyinfe/semi-ui'
import AvatarCard from './AvatarCard'
import { IconPlusCircle } from '@douyinfe/semi-icons'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import useSWRMutation from 'swr/mutation'
import { useBiliUsers } from '../lib/use-streamers'
import QRcode from '@/app/ui/QRcode'
import { useWindowSize } from 'react-use';

type UserListProps = {
  onCancel?: (e: React.MouseEvent<Element, MouseEvent> | React.KeyboardEvent<Element>) => void
  visible?: boolean
  children?: React.ReactNode
}
const UserList: React.FC<UserListProps> = ({ onCancel, visible }) => {
  const { trigger } = useSWRMutation('/v1/users', sendRequest)
  const { trigger: deleteUser } = useSWRMutation('/v1/users', requestDelete)
  const { biliUsers: list } = useBiliUsers()
  const [modalVisible, setVisible] = useState(false)
  const [confirmLoading, setConfirmLoading] = useState(false)
  const { width } = useWindowSize()
  const showDialog = () => {
    setVisible(true)
  }

  const addUser = async (value: any) => {
    setConfirmLoading(true)
    try {
      const ret = await fetcher(`/bili/space/myinfo?user=${value}`, undefined)
      if (ret.code) {
        throw new Error(ret.message)
      }
      await trigger({
        // id: 0,
        // name: value,
        // value: value,
        // platform: 'bilibili-cookies',
        key: 'bilibili-cookies',
        value: value
      })
      setVisible(false)
      Toast.success('创建成功')
    } catch (e: any) {
      let messageObj = e.message
      try {
        messageObj = JSON.parse(messageObj).error
      } catch (e: any) {
        console.log(e)
      }
      return Notification.error({
        title: '创建失败',
        content: (
          <Typography.Paragraph style={{ maxWidth: 450 }}>{messageObj}</Typography.Paragraph>
        ),
        // theme: 'light',
        // duration: 0,
        style: { width: 'min-content' },
      })
    } finally {
      setConfirmLoading(false)
    }
  }
  const handleOk = async () => {
    let values = await api.current?.validate()
    await addUser(values?.value)
  }
  const handleCancel = () => {
    setVisible(false)
    console.log('Cancel button clicked')
  }
  const handleAfterClose = () => {
    console.log('After Close callback executed')
  }
  const updateList = async (id: number) => {
    try {
      await deleteUser(id)
      Toast.success('删除成功')
    } catch (e: any) {
      Notification.error({
        title: '删除失败',
        content: <Typography.Paragraph style={{ maxWidth: 450 }}>{e.message}</Typography.Paragraph>,
        // theme: 'light',
        // duration: 0,
        style: { width: 'min-content' },
      })
    }
  }
  const api = useRef<FormApi>()
  const [value, setValue] = useState()
  const [panel, setPanel] = useState(<></>)
  const onChange = (e: any) => {
    setValue(e.target.value)
    if (e.target.value === 2) {
      setPanel(<QRcode onSuccess={addUser} />)
    }
    if (e.target.value === 1) {
      setPanel(
        <Form getFormApi={formApi => (api.current = formApi)}>
          <Form.Input
            field="value"
            label="Cookie路径"
            trigger="blur"
            placeholder="cookies.json"
            rules={[{ required: true }]}
          />
        </Form>
      )
    }
  }
  return (
    <SideSheet
      title={<Typography.Title heading={4}>用户管理</Typography.Title>}
      visible={visible}
      width={Math.min(448, width ?? Number.MIN_VALUE)}
      footer={
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <Button
            onClick={showDialog}
            icon={<IconPlusCircle size="large" />}
            style={{ marginRight: 4, backgroundColor: 'rgba(var(--semi-indigo-0), 1)' }}
          >
            新增
          </Button>
        </div>
      }
      headerStyle={{ borderBottom: '1px solid var(--semi-color-border)' }}
      bodyStyle={{ borderBottom: '1px solid var(--semi-color-border)' }}
      onCancel={onCancel}
    >
      <List
        className="component-list-demo-booklist"
        dataSource={list}
        split={false}
        size="small"
        style={{ flexBasis: '100%', flexShrink: 0 }}
        renderItem={
          item => (
            <AvatarCard
              url={item.face}
              abbr={item.name}
              label={item.name}
              value={item.value}
              onRemove={async () => await updateList(item.id)}
            />
          )
          // <div style={{ margin: 4 }} className='list-item'>
          //     <Button type='danger' theme='borderless' icon={<IconMinusCircle />} onClick={() => updateList(item)} style={{ marginRight: 4 }} />
          //     {item}
          // </div>
        }
      />

      <Modal
        title="新建"
        visible={modalVisible}
        onOk={handleOk}
        style={{ width: 'min(600px, 90vw)' }}
        afterClose={handleAfterClose} //>=1.16.0
        onCancel={handleCancel}
        closeOnEsc={true}
        confirmLoading={confirmLoading}
        okButtonProps={{ disabled: value === 2 }}
        bodyStyle={{
          overflow: 'auto',
          maxHeight: 'calc(100vh - 320px)',
          paddingLeft: 10,
          paddingRight: 10,
        }}
      >
        <Row type="flex" justify="center">
          <RadioGroup type="button" buttonSize="large" onChange={onChange} value={value}>
            <Radio value={1}>cookie文件</Radio>
            <Radio value={2}>扫码登录</Radio>
          </RadioGroup>
        </Row>
        <Row>{panel}</Row>
      </Modal>
    </SideSheet>
  )
}

export default UserList
