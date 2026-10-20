import type { MosaicConfig } from './api-streamer'

/**
 * Values used by the room-level "local material" shortcut.
 *
 * The uploader and downloader live in the room override patch, while the
 * postprocessor is a top-level room field. Keeping that split here prevents
 * the UI from accidentally serializing either value into the wrong object.
 */
export const LOCAL_MATERIAL_MOSAIC_CONFIG: MosaicConfig = {
  enabled: false,
  regions: [],
}

export const LOCAL_MATERIAL_OVERRIDE = {
  downloader: 'mesio' as const,
  uploader: 'Noop' as const,
  filtering_threshold: 0,
  mosaic_config: LOCAL_MATERIAL_MOSAIC_CONFIG,
}

export type LocalMaterialPresetResult = {
  override: Record<string, unknown>
  postprocessor: []
}

/** Apply the shortcut while retaining unrelated room override keys. */
export function applyLocalMaterialPreset(
  currentOverride: Record<string, unknown> = {},
): LocalMaterialPresetResult {
  return {
    override: {
      ...currentOverride,
      ...LOCAL_MATERIAL_OVERRIDE,
      // Do not share the exported object with form state or later mutations.
      mosaic_config: {
        enabled: false,
        regions: [],
      },
    },
    postprocessor: [],
  }
}
