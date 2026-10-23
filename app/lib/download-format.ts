/** Match the runtime's container support, including Twitch's streamlink routing. */
export function downloadFormatPolicy(downloader: unknown, url: string = '') {
  let twitch = false
  try {
    const host = new URL(url).hostname.toLowerCase()
    twitch = host === 'twitch.tv' || host.endsWith('.twitch.tv')
  } catch { /* The URL field validates incomplete input separately. */ }
  if (['ffmpeg', 'ffmpeg-external', 'ffmpeg-internal'].includes(String(downloader)) && !(downloader === 'ffmpeg' && twitch)) {
    return { formats: ['mp4', 'flv', 'ts', 'mkv'], help: 'FFmpeg 会将视频实际转封装为所选格式。留空时沿用来源格式。' }
  }
  if (downloader === 'ytarchive') {
    return { formats: ['mp4', 'mkv'], help: 'ytarchive 支持 MP4 和 MKV，留空默认 MP4；仅用于 YouTube 直播。' }
  }
  return {
    formats: [],
    help: downloader === 'ffmpeg' && twitch
      ? 'Twitch 的 FFmpeg 选项实际通过 streamlink 录制，按实际容器保存，不能指定后缀。'
      : downloader === 'sync-downloader'
        ? '边录边传固定使用 Matroska（.mkv），不能指定后缀。'
        : '当前下载方式按实际容器保存。选择支持转封装的下载器后才能设置后缀。',
  }
}

export function savedDownloadFormat(format: unknown, downloader: unknown, url: string, known = true) {
  // An unavailable node/global configuration must not erase an existing value.
  if (!known) return format ?? null
  const value = typeof format === 'string' ? format.trim().replace(/^\.+/, '').toLowerCase() : ''
  return downloadFormatPolicy(downloader, url).formats.includes(value) ? value : null
}
