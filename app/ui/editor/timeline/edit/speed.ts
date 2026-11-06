import { planSetSpeed } from '@/app/lib/editor/core/timeline/arrange';
import type { ProjectModel } from '@/app/lib/editor/state/project';
import { findItem, type Plan } from './base';
import { withOverlayRipple } from './ripple';

/** Playback settings are captured by the caller, never read while a queued edit executes. */
export function planSpeed(project: ProjectModel, itemId: string, speed: number, keepPitch: boolean | undefined, linkOverlays: boolean): Plan | {readonly problem: string} {
  const plan = planSetSpeed(project, itemId, speed, keepPitch);
  if ('problem' in plan || !linkOverlays || findItem(project, itemId)?.track.role !== 'main') return plan;
  return withOverlayRipple(project, plan);
}
