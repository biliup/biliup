import type { Item, Project } from '@/app/lib/editor/core/timeline/types';

const positions = new WeakMap<HTMLMediaElement, { key: string; cycle: number }>();

/** Timeline loops use registered source frames, not the container/proxy's possibly different duration. */
export function syncMediaLoop(element: HTMLMediaElement, project: Pick<Project, 'media' | 'canvas'>, item: Item, local: number): boolean {
  element.loop = false;
  const previous = positions.get(element);
  const media = project.media.find(media => media.id === item.media);
  const frames = item.loop ? media?.probe?.frames : undefined;
  if (!frames) {
    positions.delete(element);
    return previous !== undefined;
  }
  const cycle = Math.floor(((item.in ?? 0) + local * (item.speed ?? 1)) / frames);
  const key = JSON.stringify([item.id, media!.path, element.currentSrc || element.src, frames, project.canvas.fps, item.in ?? 0, item.speed ?? 1]);
  positions.set(element, { key, cycle });
  return !previous || previous.key !== key || previous.cycle !== cycle;
}
