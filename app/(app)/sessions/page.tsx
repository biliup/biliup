'use client'
import { Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import Link from 'next/link'
import { LibraryBig } from 'lucide-react'
import SessionsTab from '../workbench/SessionsTab'
import { parseWorkbenchNumber } from '@/app/lib/workbench'
import library from '../workbench/library.module.scss'

export default function SessionsPage() { return <Suspense><Sessions/></Suspense> }
function Sessions() {
  const params = useSearchParams(), streamerId = parseWorkbenchNumber(params.get('streamer'), 1)
  return <div className={library.content}>
    <header className={library.topbar}>
      <h1><LibraryBig aria-hidden="true" size={20}/>媒体库</h1>
      {streamerId && <Link className={library.allLink} href="/sessions">查看全部场次</Link>}
    </header>
    <SessionsTab key={streamerId ?? 'all'} streamerId={streamerId}/>
  </div>
}
