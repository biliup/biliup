'use client'
import React, { useEffect } from 'react'
import { useFormApi } from '@douyinfe/semi-ui'

/**
 * 放在 Semi Form 的最后一个子节点：前面的字段都已注册（Semi 在 layout effect 里注册并填上字段自带的 initValue），
 * 这时的表单值就是「没动过」的基准，提交时与它比较判断哪些字段真的改了。
 */
export default function FormSnapshot({ snapshotRef }: { snapshotRef: React.MutableRefObject<Record<string, unknown>> }) {
  const api = useFormApi()
  useEffect(() => {
    snapshotRef.current = JSON.parse(JSON.stringify(api.getValues() ?? {}))
  }, [api, snapshotRef])
  return null
}
