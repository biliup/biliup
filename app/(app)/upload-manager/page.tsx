'use client'
import {
  Button,
  ButtonGroup,
  List,
  Popconfirm,
  Notification,
  Typography,
  Modal,
  Transfer,
  Card,
} from '@douyinfe/semi-ui'
import {
  IconCloudStroked,
  IconPlusCircle,
  IconUserListStroked,
  IconEdit2Stroked,
  IconSendStroked,
  IconDeleteStroked,
} from '@douyinfe/semi-icons'
import { useState } from 'react'
import Link from 'next/link'
import { fetcher, FileList, requestDelete, sendRequest, StudioEntity } from '../../lib/api-streamer'
import useSWR from 'swr'
import { useRouter } from 'next/navigation'
import UserList from '../../ui/UserList'
import useSWRMutation from 'swr/mutation'
import { useBiliUsers } from '../../lib/use-streamers'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'

export default function UploadManager() {
  const { Meta } = Card
  const { Text } = Typography
  const [visible, setVisible] = useState(false)
  const router = useRouter()
  const { trigger: deleteUpload } = useSWRMutation('/v1/upload/streamers', requestDelete)
  const { data: templates, error, isLoading } = useSWR<StudioEntity[]>(
    '/v1/upload/streamers',
    fetcher
  )
  const { biliUsers } = useBiliUsers()

  const handleAddLinkClick = (event: React.MouseEvent) => {
    if (biliUsers.length === 0) {
      event.preventDefault()
      change()
      Notification.info({
        title: '用户列表为空',
        position: 'top',
        content: '请先在右侧点击新增用户',
        duration: 3,
      })
    }
  }

  const change = () => setVisible(!visible)
  const onConfirm = async (id: number) => {
    await deleteUpload(id)
  }

  const [visibleModal, setVisibleModal] = useState(false)
  const [selectFiles, setSelectFiles] = useState<(string | number)[]>([])
  const [selectEntity, setSelectEntity] = useState<StudioEntity>()
  const showDialog = (entity: StudioEntity) => {
    setSelectEntity(entity)
    setVisibleModal(true)
  }
  const handleOk = async () => {
    await sendRequest('/v1/uploads', {
      arg: {
        files: selectFiles.map(String),
        template_id: selectEntity?.id,
      },
    })
    setVisibleModal(false)
  }

  const { data: fileList } = useSWR<FileList[]>('/v1/videos', fetcher)
  const data = fileList?.map((v) => ({
    label: v.name,
    value: v.name,
    disabled: false,
    key: v.key,
  }))
  const [transferData, setTransferData] = useState<(string | number)[]>([])

  const handleTransferChange = (values: (string | number)[], items: any[]) => {
    setSelectFiles(values)
    setTransferData(values)
  }

  const actions = (
    <>
      <Button
        onClick={change}
        type="tertiary"
        icon={<IconUserListStroked />}
        aria-label="用户管理"
        title="用户管理"
      />
      <Link href="/upload-manager/add" onClick={handleAddLinkClick}>
        <Button icon={<IconPlusCircle />} theme="solid">
          新建
        </Button>
      </Link>
    </>
  )

  return (
    <>
      <UserList visible={visible} onCancel={change} />
      <Modal
        size="medium"
        title="文件选择"
        okText="上传"
        style={{ width: 'min(600px, 90vw)' }}
        visible={visibleModal}
        onOk={handleOk}
        onCancel={() => setVisibleModal(false)}
        bodyStyle={{ overflow: 'auto' }}
        closeOnEsc={true}
      >
        <Transfer
          style={{ height: 416 }}
          dataSource={data}
          draggable
          value={transferData}
          onChange={handleTransferChange}
        />
      </Modal>

      <PageHeader
        icon={<IconCloudStroked size="large" />}
        title="投稿管理"
        description="管理上传模板,选择录制文件一键投稿"
        actions={actions}
      />
      <div className={dc.content}>
        <List
          grid={{
            gutter: 12,
            xs: 24,
            sm: 24,
            md: 12,
            lg: 8,
            xl: 6,
            xxl: 4,
          }}
          dataSource={templates}
          loading={isLoading}
          renderItem={(item: StudioEntity) => (
            <List.Item>
              <Card
                shadows="hover"
                style={{
                  maxWidth: 360,
                  margin: '8px 2px',
                  flexGrow: 1,
                  borderRadius: 12,
                }}
                bodyStyle={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '16px 18px',
                }}
              >
                <Meta
                  title={
                    <Text
                      ellipsis={{ showTooltip: true, pos: 'middle' }}
                      style={{ maxWidth: 150 }}
                    >
                      {item.template_name}
                    </Text>
                  }
                />
                <ButtonGroup style={{ minWidth: 100 }} theme="borderless">
                  <Button icon={<IconSendStroked />} onClick={() => showDialog(item)} />
                  <Button
                    icon={<IconEdit2Stroked />}
                    onClick={() => router.push(`/upload-manager/edit?id=${item.id}`)}
                  />
                  <Popconfirm
                    title="确定是否要删除？"
                    content="此操作将不可逆"
                    margin={50}
                    onConfirm={async () => await onConfirm(item.id)}
                  >
                    <Button theme="borderless" icon={<IconDeleteStroked />} />
                  </Popconfirm>
                </ButtonGroup>
              </Card>
            </List.Item>
          )}
        />
      </div>
    </>
  )
}
