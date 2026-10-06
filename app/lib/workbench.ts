/** 「剪辑台」页 `/workbench` 的 Tab，记在查询串 `?tab=` 里，刷新和分享都停在同一个 Tab */
export type WorkbenchTab = 'sessions' | 'files' | 'monitor'

const TABS: readonly WorkbenchTab[] = ['sessions', 'files', 'monitor']

/** 回看从场次进，所以默认打开「直播场次」 */
export const DEFAULT_WORKBENCH_TAB: WorkbenchTab = 'sessions'

export function parseWorkbenchTab(raw: string | null): WorkbenchTab {
  return TABS.find((tab) => tab === raw) ?? DEFAULT_WORKBENCH_TAB
}

export const workbenchHref = (tab: WorkbenchTab = DEFAULT_WORKBENCH_TAB) => `/workbench?tab=${tab}`
