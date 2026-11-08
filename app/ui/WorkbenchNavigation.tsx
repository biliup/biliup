'use client'
import { Suspense, useState, type MouseEvent, type ReactNode } from 'react'
import Link from 'next/link'
import { usePathname, useSearchParams, useRouter } from 'next/navigation'
import useSWR from 'swr'
import { ChevronDown, Plus, Clapperboard, Ellipsis } from 'lucide-react'
import { fetcher } from '@/app/lib/api-streamer'
import { projectsUrl, type ProjectSummary } from '@/app/lib/editor/api'
import { projectEditorHref } from '@/app/lib/workbench'
import { useMe } from '@/app/lib/use-me'
import styles from '@/app/(app)/layout.module.scss'

type Props = { icon: ReactNode; collapsed: boolean; expand: () => void; onNavigate: () => void }

export default function WorkbenchNavigation(props: Props) {
  return <Suspense fallback={<Link className={styles.item} href="/workbench/projects">
    <span className={styles.itemIcon}>{props.icon}</span>{!props.collapsed && '剪辑台'}
  </Link>}><ProjectNavigation {...props} /></Suspense>
}

function ProjectNavigation({ icon, collapsed, expand, onNavigate }: Props) {
  const pathname=usePathname(), active = pathname === '/workbench', projectsPage=pathname==='/workbench/projects'
  const params = useSearchParams(), router = useRouter(), { can } = useMe()
  const [open, setOpen] = useState(true)
  const { data, error: loadError } = useSWR<ProjectSummary[]>(can('file.view') ? projectsUrl+'?limit=6' : null, fetcher, { refreshInterval: 10000 })
  const projects = (data??[]).slice(0,5)

  function navigate(href: string) {
    // Query-only navigation must retain the workbench's live session owner.
    if (active&&href.startsWith('/workbench?')) window.history.pushState(null, '', href)
    else router.push(href)
    onNavigate()
  }
  function create() {
    const query = new URLSearchParams(active ? params.toString() : '')
    query.set('new', '1')
    navigate('/workbench?' + query)
  }
  function openProject(event: MouseEvent<HTMLAnchorElement>) {
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.button) return
    event.preventDefault()
    navigate(event.currentTarget.getAttribute('href')!)
  }

  return <div>
    <div className={styles.workbenchHeading}>
      <button type="button" className={[styles.item, styles.navDisclosure, active||projectsPage ? styles.itemActive : ''].join(' ')} title="剪辑台"
        onClick={() => { if (collapsed) { expand(); setOpen(true) } else navigate('/workbench/projects') }}>
        <span className={styles.itemIcon}>{icon}</span>{!collapsed && <span>剪辑台</span>}
      </button>
      {!collapsed && can('clip.edit') && <button className={styles.newProjectButton} title="新建工程" aria-label="新建工程" onClick={create}><Plus size={16} /></button>}
      {!collapsed && <button className={styles.newProjectButton} aria-label={open ? '收起工程列表' : '展开工程列表'} aria-expanded={open} aria-controls="workbench-projects" onClick={() => setOpen(!open)}>
        <ChevronDown size={14} className={styles.navChevron} data-open={open} />
      </button>}
    </div>
    {!collapsed && open && <div id="workbench-projects" className={styles.subnav}>
      {projects.map(project => {
        const selected = active && params.get('project') === String(project.id)
        return <Link key={project.id} href={projectEditorHref(project.id)}
          onClick={openProject} title={project.name} aria-current={selected ? 'page' : undefined}
          className={[styles.subnavItem, styles.projectNavItem, selected ? styles.subnavActive : ''].join(' ')}>
          <Clapperboard size={14} /><span>{project.name}</span>
        </Link>
      })}
      {loadError && <span role="alert">工程加载失败</span>}
      {data && !projects.length && <span className={styles.emptyProjects}>暂无工程</span>}
      {(data?.length??0)>5&&<Link href="/workbench/projects" onClick={onNavigate} aria-current={projectsPage?'page':undefined} className={[styles.subnavItem,styles.projectNavItem,projectsPage?styles.subnavActive:''].join(' ')}><Ellipsis size={14}/><span>更多工程</span></Link>}
    </div>}
  </div>
}
