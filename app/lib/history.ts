/** 「历史记录」页 `/history` 的 Tab，记在查询串 `?tab=` 里，刷新和分享都停在同一个 Tab */
export type HistoryTab = 'sessions' | 'files' | 'monitor'

const TABS: readonly HistoryTab[] = ['sessions', 'files', 'monitor']

/** 回看从场次进，所以默认打开「直播场次」 */
export const DEFAULT_HISTORY_TAB: HistoryTab = 'sessions'

export function parseHistoryTab(raw: string | null): HistoryTab {
  return TABS.find((tab) => tab === raw) ?? DEFAULT_HISTORY_TAB
}

export const historyHref = (tab: HistoryTab = DEFAULT_HISTORY_TAB) => `/history?tab=${tab}`
