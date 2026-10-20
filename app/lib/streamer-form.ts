import { formListToHookStep, hookStepListToForm } from './postprocessor'
import type { FleetRoom, RoomInput } from './use-fleet'

const ROOM_KEYS = [
  'url',
  'remark',
  'filename_prefix',
  'time_range',
  'format',
  'override',
  'preprocessor',
  'segment_processor',
  'downloaded_processor',
  'opt_args',
  'excluded_keywords',
] as const

/** 录制表单（与本机「直播管理」共用 `StreamerEditor`）的值 → 房间 */
export function roomFromForm(values: Record<string, any>): RoomInput {
  const room: Record<string, unknown> = {}
  for (const key of ROOM_KEYS) room[key] = values[key] ?? null
  room.postprocessor = values.postprocessor ? formListToHookStep(values.postprocessor) : null
  room.template_id = values.upload_streamers_id ?? null
  room.required_labels = Array.isArray(values.required_labels) ? values.required_labels : []
  return room as unknown as RoomInput
}

export function roomToForm(room: FleetRoom) {
  return {
    ...room,
    upload_streamers_id: room.template_id ?? undefined,
    postprocessor: room.postprocessor ? hookStepListToForm(room.postprocessor) : room.postprocessor,
  } as any
}

