'use client'

import React, { Suspense, useRef } from 'react'
import { Form, Notification, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import {
  BiliType,
  fetcher,
  LiveStreamerEntity,
  sendRequest,
  StudioEntity,
} from '@/app/lib/api-streamer'
import TemplateFields from '@/app/ui/TemplateFields'
import useSWRMutation from 'swr/mutation'
import { useRouter, useSearchParams } from 'next/navigation'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import useSWR from 'swr'
import { useTypeTree } from '@/app/lib/use-streamers'
import { FormPage, usePageLabelPosition } from '@/app/ui/shell'

const BACK = { href: '/upload-manager', label: '投稿管理' }

const Edit = () => {
  const { Paragraph } = Typography
  const searchParams = useSearchParams()
  const { trigger } = useSWRMutation('/v1/upload/streamers', sendRequest)
  const { data, error, isLoading, mutate } = useSWR<StudioEntity>(
    () => (searchParams.get('id') ? `/v1/upload/streamers/${searchParams.get('id')}` : null),
    fetcher
  )
  const router = useRouter()
  const { typeTree, isError } = useTypeTree()
  const api = useRef<FormApi>(undefined)
  const labelPosition = usePageLabelPosition()

  // 分区树来自 /bili/archive/pre，需要一个可用的 B 站 cookie；没有账号时后端返回 500，
  // 这里必须给出可读的原因而不是空白页。
  const serverMessage = (e: any): string | undefined => {
    const raw = e?.message
    if (typeof raw !== 'string' || !raw) return undefined
    try {
      const parsed = JSON.parse(raw)
      return typeof parsed?.message === 'string' ? parsed.message : raw
    } catch {
      return raw
    }
  }
  const loadError: string | undefined = error
    ? `模板加载失败：${serverMessage(error) ?? '未知错误'}`
    : isError
      ? `分区列表加载失败：${serverMessage(isError) ?? '未知错误'}（需要至少一个可用的 B 站账号）`
      : undefined
  if (loadError) {
    return (
      <FormPage title="编辑投稿模板" back={BACK}>
        <Typography.Text type="danger">{loadError}</Typography.Text>
      </FormPage>
    )
  }
  if (isLoading || !data || !typeTree) {
    return (
      <FormPage title="编辑投稿模板" back={BACK}>
        <div style={{ padding: '64px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      </FormPage>
    )
  }

  // 本机模板接口回来的这三个互动开关是布尔值（后端 UploadStreamer 里是 Option<bool>），
  // 其余开关是 0 / 1。只认 1 时打开过的开关回显成关，一保存就被写成 0
  const flagOn = (flag: unknown) => flag === 1 || flag === true

  let uploadStreamers = {
    ...data,
    tid: [
      typeTree.find((tt: BiliType) => {
        return tt.children.some((ct) => ct.id === data?.tid)
      })?.value,
      data.tid,
    ],
    sound: (data.dolby === 1 ? ['dolby'] : []).concat(data.hires === 1 ? ['hires'] : []),
    interaction: (flagOn(data.up_close_danmu) ? ['up_close_danmu'] : [])
      .concat(flagOn(data.up_close_reply) ? ['up_close_reply'] : [])
      .concat(flagOn(data.up_selection_reply) ? ['up_selection_reply'] : []),
    charging_pay: data.charging_pay === 1,
    no_reprint: data.no_reprint === 1,
    is_only_self: data.is_only_self === 1,
    isDtime: data.dtime ? true : false,
  }

  const handleSave = async () => {
    const values = await api.current?.validate()
    try {
      const studioEntity: StudioEntity = {
        template_name: values?.template_name,
        user_cookie: values?.user_cookie,
        copyright: values?.copyright,
        id: values?.id,
        copyright_source: values?.copyright_source ?? '',
        tid: values?.tid[1],
        tid_v2: values?.tid_v2 || null,
        cover_path: values?.cover_path ?? '',
        title: values?.title ?? '',
        description: values?.description ?? '',
        dynamic: values?.dynamic ?? '',
        tags: values?.tags ?? [],
        dolby: values?.sound.includes('dolby') ? 1 : 0,
        hires: values?.sound.includes('hires') ? 1 : 0,
        up_selection_reply: values?.interaction.includes('up_selection_reply') ? 1 : 0,
        up_close_reply: values?.interaction.includes('up_close_reply') ? 1 : 0,
        up_close_danmu: values?.interaction.includes('up_close_danmu') ? 1 : 0,
        charging_pay: values?.charging_pay ? 1 : 0,
        no_reprint: values?.no_reprint ? 1 : 0,
        is_only_self: values?.is_only_self ? 1 : 0,
        mission_id: values?.mission_id,
        dtime: values?.isDtime ? values?.dtime : null,
        credits: values?.credits ?? null,
        uploader: values?.uploader ?? null,
        extra_fields: values?.extra_fields ?? '',
      }
      const result = await trigger(studioEntity)
      await mutate(result)
      Toast.success('更新成功')
      router.push('/upload-manager')
    } catch (e: any) {
      Notification.error({
        title: '保存失败',
        content: <Paragraph style={{ maxWidth: 450 }}>{e.message}</Paragraph>,
        style: { width: 'min-content' },
      })
      throw e
    }
  }

  return (
    <FormPage
      title={`编辑投稿模板「${data.template_name}」`}
      description="修改模板信息并保存"
      back={BACK}
      okText="保存模板"
      onOk={handleSave}
    >
      <Form
        initValues={uploadStreamers}
        autoScrollToError
        onSubmit={handleSave}
        component={TemplateFields}
        getFormApi={(formApi) => (api.current = formApi)}
        labelWidth="140px"
        labelPosition={labelPosition}
      />
    </FormPage>
  )
}

const EditTemplate: React.FC = () => {
  return (
    <Suspense>
      <Edit />
    </Suspense>
  )
}

export default EditTemplate
