'use client'
import { useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { Spin } from '@douyinfe/semi-ui'

/** #1797 时本机密钥是单独的页面；现在是空间配置页上的抽屉，旧链接转过去并打开它 */
export default function Page() {
  const router = useRouter()
  useEffect(() => {
    router.replace('/dashboard?secrets=1')
  }, [router])
  return (
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <Spin size="large" />
    </div>
  )
}
