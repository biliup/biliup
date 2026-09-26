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
  Tag,
  Tooltip,
  Banner,
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
import { useMe } from '../../lib/use-me'
import dc from '@/app/ui/data-card.module.scss'

export default function UploadManager() {
  const { Text } = Typography
  const [visible, setVisible] = useState(false)
  const router = useRouter()
  const { trigger: deleteUpload } = useSWRMutation('/v1/upload/streamers', requestDelete)
  const { data: templates, error, isLoading } = useSWR<StudioEntity[]>(
    '/v1/upload/streamers',
    fetcher
  )
  const { biliUsers } = useBiliUsers()
  const { me, can } = useMe()
  const canManageAccounts = can('account.manage')
  const canEditTemplates = can('template.edit')
  const canSubmit = can('upload.submit')
  // 本机加入了控制面时，控制面下发的投稿模板只读（后端对它们的改删返回 409）
  const fleet = me?.fleet_node
  const managedIds = new Set(fleet?.templates ?? [])
  const managedHint = fleet ? `由控制面 ${fleet.controller} 管理，请到控制面修改` : undefined

  const handleAddLinkClick = (event: React.MouseEvent) => {
    if (biliUsers.length === 0) {
      event.preventDefault()
      if (canManageAccounts) change()
      Notification.info({
        title: 'B 站账号列表为空',
        position: 'top',
        content: canManageAccounts
          ? '请先在右侧点击新增账号'
          : '请联系超级管理员先登记 B 站账号',
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
      {canManageAccounts && (
        <Button
          onClick={change}
          type="tertiary"
          icon={<IconUserListStroked />}
          aria-label="B 站账号"
          title="B 站账号"
        />
      )}
      {canEditTemplates && (
        <Link href="/upload-manager/add" prefetch={false} onClick={handleAddLinkClick}>
          <Button icon={<IconPlusCircle />} theme="solid">
            新建
          </Button>
        </Link>
      )}
    </>
  )

  return (
    <>
      {canManageAccounts && <UserList visible={visible} onCancel={change} />}
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
        description={
          canEditTemplates ? '管理上传模板,选择录制文件一键投稿' : '查看已配置的上传模板'
        }
        actions={canEditTemplates || canManageAccounts ? actions : undefined}
      />
      <div className={dc.content}>
        {fleet ? (
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            style={{ marginBottom: 12 }}
            description={
              managedIds.size > 0
                ? `标着「托管」的 ${managedIds.size} 个模板由控制面 ${fleet.controller} 管理，这里只能查看和用来投稿，修改请到控制面。本机自己的模板不受影响。`
                : `本机已加入控制面 ${fleet.controller}；控制面下发的投稿模板会由控制面 ${fleet.controller} 管理，这里只能查看。`
            }
          />
        ) : null}
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
                  margin: '8px 2px',
                  flexGrow: 1,
                  height: '100%',
                  borderRadius: 12,
                  border: '1px solid var(--semi-color-border)',
                }}
                bodyStyle={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 12,
                  padding: '16px 18px',
                }}
              >
                {/* 模板名称:优先完整展示,占满剩余宽度,超出才尾部省略 */}
                <Text
                  ellipsis={{ showTooltip: true }}
                  title={item.template_name}
                  style={{ flex: 1, minWidth: 0, fontSize: 14, fontWeight: 600 }}
                >
                  {item.template_name}
                </Text>
                {managedIds.has(item.id) ? (
                  <Tooltip content={managedHint}>
                    <Tag size="small" color="violet" style={{ flexShrink: 0 }}>
                      托管
                    </Tag>
                  </Tooltip>
                ) : null}
                {(canSubmit || canEditTemplates) && (
                  <ButtonGroup style={{ flexShrink: 0 }} theme="borderless">
                    {[
                      canSubmit && (
                        <Button
                          key="send"
                          icon={<IconSendStroked />}
                          aria-label="投稿"
                          onClick={() => showDialog(item)}
                        />
                      ),
                      canEditTemplates && (
                        <Button
                          key="edit"
                          icon={<IconEdit2Stroked />}
                          aria-label="编辑"
                          disabled={managedIds.has(item.id)}
                          title={managedIds.has(item.id) ? managedHint : undefined}
                          onClick={() => router.push(`/upload-manager/edit?id=${item.id}`)}
                        />
                      ),
                      canEditTemplates && (
                        <Popconfirm
                          key="delete"
                          title="确定是否要删除？"
                          content="此操作将不可逆"
                          margin={50}
                          disabled={managedIds.has(item.id)}
                          onConfirm={async () => await onConfirm(item.id)}
                        >
                          <Button
                            theme="borderless"
                            icon={<IconDeleteStroked />}
                            aria-label="删除"
                            disabled={managedIds.has(item.id)}
                            title={managedIds.has(item.id) ? managedHint : undefined}
                          />
                        </Popconfirm>
                      ),
                    ].filter(Boolean)}
                  </ButtonGroup>
                )}
              </Card>
            </List.Item>
          )}
        />
      </div>
    </>
  )
}
