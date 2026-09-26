'use client'
import { useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { Spin } from '@douyinfe/semi-ui'
import { historyHref } from '@/app/lib/history'

/** 「直播历史」已并进「历史记录」的「直播场次」Tab；旧地址（书签、外部链接）在这里换过去 */
export default function JobRedirect() {
  const router = useRouter()
  useEffect(() => {
    router.replace(historyHref('sessions'))
  }, [router])
  return (
    <div style={{ padding: '80px 0', textAlign: 'center' }}>
      <Spin size="large" />
    </div>
  )
}
