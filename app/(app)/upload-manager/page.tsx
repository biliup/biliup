'use client'
import {
  Button,
  ButtonGroup,
  List,
  Popconfirm,
  Notification,
  Typography,
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
import { pairLabel, pairPeer, useMe } from '../../lib/use-me'
import dc from '@/app/ui/data-card.module.scss'
import { FormDialog } from '@/app/ui/shell'

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
  const managedHint = fleet
    ? fleet.local
      ? '这是 Fleet 投稿模板，请到「节点 › 投稿模板」修改'
      : `由控制面 ${fleet.controller} 管理，请到控制面修改`
    : undefined
  // 一主一备里的节点：配对里的模板照常修改，改动与对端双向同步；配对里的直播间在用，删除仍返回 409
  const pair = fleet?.pair
  const pairedIds = new Set(pair?.templates ?? [])
  const pairHint = pair
    ? `在两台之间同步：这里的修改会同步到${pairPeer(pair)} ${fleet?.controller}，那边的修改也会同步过来`
    : undefined
  const pairDeleteHint = '配对里的直播间在用这个模板，不能在这里删除'

  const handleAddLinkClick = (event: React.MouseEvent) => {
    if (biliUsers.length === 0) {
      event.preventDefault()
      if (canManageAccounts) change()
      Notification.info({
        title: 'B 站账号列表为空',
        position: 'top',
        content: canManageAccounts
          ? '请先在右侧抽屉里添加账号'
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
    try {
      await sendRequest('/v1/uploads', {
        arg: {
          files: selectFiles.map(String),
          template_id: selectEntity?.id,
        },
      })
    } catch (e: any) {
      Notification.error({ title: '投稿失败', content: e?.message ?? String(e) })
      throw e
    }
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
        <Button onClick={change} type="tertiary" icon={<IconUserListStroked />}>
          B 站账号
        </Button>
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
      <FormDialog
        size="lg"
        title="选择要投稿的文件"
        visible={visibleModal}
        okText={selectFiles.length > 0 ? `投稿 ${selectFiles.length} 个文件` : '投稿'}
        okDisabled={selectFiles.length === 0}
        onOk={handleOk}
        onCancel={() => setVisibleModal(false)}
      >
        <Text type="tertiary" size="small" ellipsis={{ showTooltip: true }} style={{ display: 'block', marginBottom: 12 }}>
          投稿模板：{selectEntity?.template_name}
        </Text>
        <Transfer
          style={{ height: 'min(416px, calc(100dvh - 280px))', minWidth: 0 }}
          dataSource={data}
          draggable
          value={transferData}
          onChange={handleTransferChange}
        />
      </FormDialog>

      <PageHeader
        icon={<IconCloudStroked size="large" />}
        title="投稿管理"
        description={
          canEditTemplates ? '管理上传模板,选择录制文件一键投稿' : '查看已配置的上传模板'
        }
        actions={canEditTemplates || canManageAccounts ? actions : undefined}
      />
      <div className={dc.content}>
        {pair && fleet ? (
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            style={{ marginBottom: 12 }}
            description={
              `本机与 ${fleet.controller} 组成一主一备：标着「${pairLabel(pair)}」的 ${pairedIds.size} 个模板是配对里的直播间在用的，在两台之间双向同步，在哪台修改都可以，同一项两边都改过时以后改的为准。B 站账号也在两台之间同步，在哪台登录都行。` +
              (managedIds.size > 0
                ? `标着「托管」的 ${managedIds.size} 个模板由控制面 ${fleet.controller} 管理，这里只能查看和用来投稿。`
                : '')
            }
          />
        ) : fleet ? (
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            style={{ marginBottom: 12 }}
            description={
              fleet.local
                ? managedIds.size > 0
                  ? `标着「托管」的 ${managedIds.size} 个模板是随房间分派到本机的 Fleet 投稿模板，这里只能查看和用来投稿，修改请到「节点 › 投稿模板」。这里自己的模板不受影响。`
                  : '已启用「本机」节点：Fleet 投稿模板会随房间分派到这里并标为「托管」，请到「节点 › 投稿模板」修改。'
                : managedIds.size > 0
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
                {pair && pairedIds.has(item.id) ? (
                  <Tooltip content={pairHint}>
                    <Tag size="small" color="green" style={{ flexShrink: 0 }}>
                      {pairLabel(pair)}
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
                          disabled={managedIds.has(item.id) || pairedIds.has(item.id)}
                          onConfirm={async () => await onConfirm(item.id)}
                        >
                          <Button
                            theme="borderless"
                            icon={<IconDeleteStroked />}
                            aria-label="删除"
                            disabled={managedIds.has(item.id) || pairedIds.has(item.id)}
                            title={
                              managedIds.has(item.id)
                                ? managedHint
                                : pairedIds.has(item.id)
                                  ? pairDeleteHint
                                  : undefined
                            }
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
