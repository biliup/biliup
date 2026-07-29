'use client'
import {
  Layout,
  Button,
  ButtonGroup,
  Popconfirm,
  Spin,
  Empty,
  Notification,
  Typography,
} from '@douyinfe/semi-ui'
import {
  IconPlusCircle,
  IconEdit2Stroked,
  IconDeleteStroked,
  IconWrench,
  IconVideoListStroked,
  IconClock,
  IconLayers,
} from '@douyinfe/semi-icons'
import useStreamers from '../../lib/use-streamers'
import TemplateModal from '../../ui/TemplateModal'
import OverrideModal from '../../ui/OverrideModal'
import { LiveStreamerEntity, put, requestDelete, sendRequest, fetcher, StreamerInfo } from '../../lib/api-streamer'
import useSWRMutation from 'swr/mutation'
import useSWR from 'swr'
import { PauseButton } from '@/app/ui/StreamerActions/PauseButton'
import PageHeader from '../components/PageHeader'
import { uploadStatusTag, platformName } from '@/app/lib/status'
import styles from './page.module.scss'

const { Text } = Typography

export default function StreamersPage() {
  const { Content } = Layout
  const { streamers, isLoading } = useStreamers()
  const { trigger: deleteStreamers } = useSWRMutation('/v1/streamers', requestDelete)
  const { trigger: updateStreamers } = useSWRMutation('/v1/streamers', put)
  const { trigger } = useSWRMutation('/v1/streamers', sendRequest)
  const { data: infos } = useSWR<StreamerInfo[]>('/v1/streamer-info', fetcher)

  // url -> 最新一次录制的直播标题（作为卡片主角「直播间标题」）
  const titleByUrl = new Map<string, { title: string; date: number }>()
  ;(infos ?? []).forEach((i) => {
    if (i.url && i.title) {
      const cur = titleByUrl.get(i.url)
      if (!cur || i.date > cur.date) titleByUrl.set(i.url, { title: i.title, date: i.date })
    }
  })

  const onConfirm = async (id: number) => {
    await deleteStreamers(id)
  }

  const handleEntityPostprocessor = (values: any) => {
    if (values?.postprocessor) {
      values.postprocessor = values.postprocessor.map(
        (element: { [key: string]: string } | string) => {
          if (element === 'rm') {
            return { cmd: 'rm' }
          } else if (typeof element === 'object' && !element.cmd) {
            const [key, value] = Object.entries(element)[0]
            return { cmd: key, value: value }
          }
          return element
        }
      )
    }
    return values
  }

  const handleOk = async (values: any) => {
    if (values?.postprocessor) {
      values.postprocessor = values.postprocessor.map(
        ({ cmd, value }: { cmd: string; value: string }) => (cmd === 'rm' ? 'rm' : { [cmd]: value })
      )
    }
    try {
      await trigger(values)
    } catch (e: any) {
      Notification.error({
        title: '创建失败',
        content: <Typography.Paragraph style={{ maxWidth: 450 }}>{e.message}</Typography.Paragraph>,
        style: { width: 'min-content' },
      })
      throw e
    }
  }

  const handleUpdate = async (values: any) => {
    delete values.status
    delete values.statusTag
    delete values.upload_status
    if (values?.postprocessor) {
      values.postprocessor = values.postprocessor.map(
        ({ cmd, value }: { cmd: string; value: string }) => (cmd === 'rm' ? 'rm' : { [cmd]: value })
      )
    }
    try {
      await updateStreamers(values)
    } catch (e: any) {
      Notification.error({
        title: '更新失败',
        content: <Typography.Paragraph style={{ maxWidth: 450 }}>{e.message}</Typography.Paragraph>,
        style: { width: 'min-content' },
      })
      throw e
    }
  }

  const renderCard = (item: LiveStreamerEntity) => {
    const isLive = item.status === 'Working'
    const label = item.remark ? `[${item.remark}]` : '[未命名]'
    const hero = titleByUrl.get(item.url)?.title || item.remark || item.url
    return (
      <div key={item.id} className={`${styles.card} ${isLive ? styles.rec : ''}`}>
        <div className={styles.cardHead}>
          <span className={styles.cardStatus}>
            <span className={`${styles.recDot} ${isLive ? styles.dotRec : styles.dotIdle}`} />
            {label}
          </span>
          <span className={styles.cardPlat}>{platformName(item.url)}</span>
        </div>

        <div className={styles.cardName} title={hero}>
          {hero}
        </div>

        <a
          className={styles.cardSub}
          href={item.url}
          target="_blank"
          rel="noreferrer"
          title={item.url}
        >
          {item.url}
        </a>

        <div className={styles.meta}>
          {item.time_range ? (
            <span className={styles.chip}>
              <IconClock size="small" />
              {Array.isArray(item.time_range) ? item.time_range.join(' ~ ') : String(item.time_range)}
            </span>
          ) : null}
          {item.split_time ? (
            <span className={styles.chip}>
              <IconClock size="small" />
              切片 {item.split_time}s
            </span>
          ) : null}
          {item.split_size ? (
            <span className={styles.chip}>
              <IconLayers size="small" />
              分片 {item.split_size}MB
            </span>
          ) : null}
          {item.upload_status ? uploadStatusTag(item.upload_status) : null}
        </div>

        <div className={styles.cardFoot}>
          <ButtonGroup theme="borderless" className={styles.cardActions}>
            <TemplateModal onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
              <Button
                theme="borderless"
                type="primary"
                icon={<IconEdit2Stroked />}
                aria-label="编辑"
              />
            </TemplateModal>
            <PauseButton streamer={item} />
            <Popconfirm
              title="确定是否要删除？"
              content="此操作将不可逆"
              onConfirm={() => onConfirm(item.id)}
            >
              <Button
                theme="borderless"
                type="danger"
                icon={<IconDeleteStroked />}
                aria-label="删除"
              />
            </Popconfirm>
            <OverrideModal onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
              <Button
                theme="borderless"
                type="tertiary"
                icon={<IconWrench />}
                aria-label="高级"
              />
            </OverrideModal>
          </ButtonGroup>
        </div>
      </div>
    )
  }

  return (
    <>
      <PageHeader
        title="直播管理"
        description="管理需要录制的直播间，支持新增、编辑与删除"
        icon={<IconVideoListStroked size="large" />}
        actions={
          <TemplateModal onOk={handleOk}>
            <Button icon={<IconPlusCircle />} theme="solid">
              新建
            </Button>
          </TemplateModal>
        }
      />
      <Content className={styles.content}>
        {isLoading ? (
          <div className={styles.center}>
            <Spin size="large" />
          </div>
        ) : streamers && streamers.length > 0 ? (
          <div className={styles.grid}>{streamers.map(renderCard)}</div>
        ) : (
          <div className={styles.center}>
            <Empty description="还没有监控任何直播间，点击右上角「新建」开始" />
          </div>
        )}
      </Content>
    </>
  )
}
