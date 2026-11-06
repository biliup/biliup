import type { LibraryQuery } from './core/audio/library'
import type { LibrarySearchResult } from './contracts'
import { expandQuery, rankSounds } from './core/audio/search'

/** The editor and assistant send the same provider query and rank the same response. */
export function prepareLibrarySearch(query: LibraryQuery): LibraryQuery {
  return { ...query, query: expandQuery(query.query).online }
}

export function finishLibrarySearch(result: LibrarySearchResult, query: Pick<LibraryQuery, 'query' | 'kind'>): LibrarySearchResult {
  const expanded = expandQuery(query.query)
  return {
    ...result,
    hits: rankSounds(result.hits, expanded.text, query.kind),
    ...(expanded.alias ? { alias: expanded.alias.name } : {}),
    ...(expanded.note ? { note: expanded.note } : {}),
    ...(expanded.online !== expanded.text ? { searched: expanded.online } : {}),
  }
}
