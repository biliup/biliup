export const MAX_LOG_LINES = 2_000
export const LOG_TRUNCATED_MESSAGE = '日志文件被截断，重新加载...'

/** Reset on a server-side truncation marker and retain only a bounded tail. */
export function appendLogMessage(previous: string[], message: string): string[] {
  const next = message.startsWith(LOG_TRUNCATED_MESSAGE) ? [message] : [...previous, message]
  return next.length > MAX_LOG_LINES ? next.slice(-MAX_LOG_LINES) : next
}
