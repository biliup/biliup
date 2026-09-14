'use client'
import React, { useEffect, useRef, useState } from 'react'
import { Form, Button, Toast, Notification, Typography } from '@douyinfe/semi-ui'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { IconPlusCircle } from '@douyinfe/semi-icons'
import { registerMediaQuery, responsiveMap } from '../../../lib/utils'
import { sendRequest, StudioEntity } from '../../../lib/api-streamer'
import useSWRMutation from 'swr/mutation'
import { useRouter } from 'next/navigation'
import TemplateFields from '../../../ui/TemplateFields'
import PageHeader from '../../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'

export default function Add() {
  const { Paragraph } = Typography
  const { trigger } = useSWRMutation('/v1/upload/streamers', sendRequest)
  const router = useRouter()
  const api = useRef<FormApi>()
  const [labelPosition, setLabelPosition] = useState<'top' | 'left' | 'inset'>('inset')
  useEffect(() => {
    const unRegister = registerMediaQuery(responsiveMap.lg, {
      match: () => {
        setLabelPosition('left')
      },
      unmatch: () => {
        setLabelPosition('top')
      },
    })
    return () => unRegister()
  }, [])

  const handleCreate = async () => {
    const values = await api.current?.validate()
    if (!values) return
    try {
      const studioEntity: StudioEntity = {
        template_name: values.template_name,
        user_cookie: values.user_cookie,
        copyright: values.copyright,
        id: values.id,
        copyright_source: values.copyright_source ?? '',
        tid: values.tid[1],
        cover_path: values.cover_path ?? '',
        title: values.title ?? '',
        description: values.description ?? '',
        dynamic: values.dynamic ?? '',
        tags: values.tags ?? [],
        dolby: values.sound?.includes('dolby') ? 1 : 0,
        hires: values.sound?.includes('hires') ? 1 : 0,
        up_selection_reply: values.interaction?.includes('up_selection_reply') ? 1 : 0,
        up_close_reply: values.interaction?.includes('up_close_reply') ? 1 : 0,
        up_close_danmu: values.interaction?.includes('up_close_danmu') ? 1 : 0,
        charging_pay: values.charging_pay ? 1 : 0,
        no_reprint: values.no_reprint ? 1 : 0,
        is_only_self: values.is_only_self ? 1 : 0,
        mission_id: values.mission_id,
        dtime: values.isDtime ? values?.dtime : null,
        credits: values.credits,
        uploader: values.uploader,
        extra_fields: values.extra_fields ?? '',
      }
      const result = await trigger(studioEntity)
      Toast.success('创建成功')
      router.push('/upload-manager')
    } catch (e: any) {
      Notification.error({
        title: '创建失败',
        content: <Paragraph style={{ maxWidth: 450 }}>{e.message}</Paragraph>,
        style: { width: 'min-content' },
      })
      throw e
    }
  }

  return (
    <>
      <PageHeader
        icon={<IconPlusCircle size="large" />}
        title="新建投稿模板"
        description="配置投稿模板,保存后可用于上传录制文件"
        actions={
          <Button onClick={handleCreate} type="primary" icon={<IconPlusCircle />} theme="solid">
            创建模板
          </Button>
        }
      />
      <div className={dc.content}>
        <div className={dc.card} style={{ padding: '28px 32px 40px' }}>
          <Form
            autoScrollToError
            onSubmit={handleCreate}
            component={TemplateFields}
            getFormApi={(formApi) => (api.current = formApi)}
            labelWidth="180px"
            labelPosition={labelPosition}
          />
        </div>
      </div>
    </>
  )
}
