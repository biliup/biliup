'use client'
import {
  Button,
  Notification,
  Typography,
  Transfer,
  Empty,
  Spin,
  Banner,
} from '@douyinfe/semi-ui'
import {
  IconCloudStroked,
  IconPlusCircle,
  IconUserListStroked,
} from '@douyinfe/semi-icons'
import { useRef, useState } from 'react'
import Link from 'next/link'
import { fetcher, FileList, requestDelete, sendRequest, StudioEntity } from '../../lib/api-streamer'
import useSWR from 'swr'
import { useRouter } from 'next/navigation'
import UserList from '../../ui/UserList'
import useSWRMutation from 'swr/mutation'
import { useBiliUsers, useTidNames } from '../../lib/use-streamers'
import PageHeader from '../components/PageHeader'
import { pairLabel, pairPeer, useMe } from '../../lib/use-me'
import dc from '@/app/ui/data-card.module.scss'
import tc from '@/app/ui/template-card.module.scss'
import TemplateCard from '@/app/ui/TemplateCard'
import { FormDialog } from '@/app/ui/shell'
import { SubmissionLock } from '@/app/lib/submission-lock'
import { errorMessage } from '@/app/lib/use-fleet'

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
  // 分区名来自 /bili/archive/pre，与投稿表单的分区选择器同一张表；拿不到时卡片只显示 tid
  const tidNames = useTidNames()
  const { me, can } = useMe()
  const canManageAccounts = can('account.manage')
  const canEditTemplates = can('template.edit')
  const canSubmit = can('upload.submit')
  // 本机加入了控制面时，控制面下发的投稿模板只读（后端对它们的改删返回 409）
  const fleet = me?.fleet_node
  const managedIds = new Set(fleet?.templates ?? [])
  const managedHint = fleet
    ? fleet.local
      ? '这是 Fleet 投稿模板，请到「任务平台 › 节点管理 › 投稿模板」修改'
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
  const [submitting, setSubmitting] = useState(false)
  const submissionLock = useRef(new SubmissionLock()).current
  const [selectFiles, setSelectFiles] = useState<(string | number)[]>([])
  const [selectEntity, setSelectEntity] = useState<StudioEntity>()
  const showDialog = (entity: StudioEntity) => {
    if (submissionLock.isLocked) return
    setSelectEntity(entity)
    setVisibleModal(true)
  }
  const handleOk = async () => {
    if (!submissionLock.acquire()) return
    setSubmitting(true)
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
    } finally {
      submissionLock.release()
      setSubmitting(false)
    }
    setVisibleModal(false)
  }
  const handleCancel = () => {
    if (submissionLock.isLocked) return
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
        closable={!submitting}
        okText={selectFiles.length > 0 ? `投稿 ${selectFiles.length} 个文件` : '投稿'}
        okDisabled={selectFiles.length === 0 || submitting}
        onOk={handleOk}
        onCancel={handleCancel}
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
                  ? `标着「托管」的 ${managedIds.size} 个模板是随房间分派到本机的 Fleet 投稿模板，这里只能查看和用来投稿，修改请到「任务平台 › 节点管理 › 投稿模板」。这里自己的模板不受影响。`
                  : '已启用「本机」节点：Fleet 投稿模板会随房间分派到这里并标为「托管」，请到「任务平台 › 节点管理 › 投稿模板」修改。'
                : managedIds.size > 0
                  ? `标着「托管」的 ${managedIds.size} 个模板由控制面 ${fleet.controller} 管理，这里只能查看和用来投稿，修改请到控制面。本机自己的模板不受影响。`
                  : `本机已加入控制面 ${fleet.controller}；控制面下发的投稿模板会由控制面 ${fleet.controller} 管理，这里只能查看。`
            }
          />
        ) : null}
        {isLoading ? (
          <div className={tc.loading}>
            <Spin />
            <Text type="tertiary" size="small">
              正在读取投稿模板…
            </Text>
          </div>
        ) : templates && templates.length > 0 ? (
          <div className={tc.grid}>
            {templates.map((item) => {
              const paired = !!pair && pairedIds.has(item.id)
              const tid = tidNames.get(item.tid)
              return (
                <TemplateCard
                  key={item.id}
                  template={item}
                  // 没有 template.edit 权限时拿不到账号列表（useBiliUsers 按这个权限门控），
                  // 这时不能回落到 user_cookie —— 那是 biliup 机器上的凭据文件路径
                  account={
                    canEditTemplates
                      ? biliUsers.find((user) => user.value === item.user_cookie)?.name
                      : undefined
                  }
                  tidName={tid?.name}
                  tidHint={[
                    tid ? `投稿分区：${tid.path}` : '投稿分区 tid',
                    item.tid_v2 ? `tid_v2 ${item.tid_v2}` : null,
                  ]
                    .filter(Boolean)
                    .join('；')}
                  canSubmit={canSubmit}
                  canEdit={canEditTemplates}
                  managed={managedIds.has(item.id)}
                  managedHint={managedHint}
                  pairLabel={paired ? pairLabel(pair!) : undefined}
                  pairHint={pairHint}
                  deleteHint={pairDeleteHint}
                  onSend={() => showDialog(item)}
                  onEdit={() => router.push(`/upload-manager/edit?id=${item.id}`)}
                  onDelete={() => onConfirm(item.id)}
                />
              )
            })}
          </div>
        ) : (
          <div className={tc.empty}>
            <Empty
              image={
                <IconPlusCircle
                  size="extra-large"
                  style={{ color: 'var(--semi-color-text-3)' }}
                />
              }
              title={error ? '投稿模板加载失败' : '还没有投稿模板'}
              description={
                error
                  ? // fetcher 抛出的是响应体原文，后端返回 JSON 时要取里面的 message，
                    // 不能直接把 {"message":"..."} 印到页面上
                    errorMessage(error) || '请稍后重试'
                  : canEditTemplates
                    ? '投稿模板决定录完的视频投到哪个账号、用什么标题和分区。新建后这里会列出它的账号、类型、分区和默认标签。'
                    : '管理员添加投稿模板后会显示在这里'
              }
            />
            {canEditTemplates && !error ? (
              <Link href="/upload-manager/add" prefetch={false} onClick={handleAddLinkClick}>
                <Button theme="solid" icon={<IconPlusCircle />}>
                  新建模板
                </Button>
              </Link>
            ) : null}
          </div>
        )}
      </div>
    </>
  )
}
