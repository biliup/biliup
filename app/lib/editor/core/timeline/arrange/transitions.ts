import { applyEdit, createSession } from '../apply-ops';
import type { Item, Op, Project } from '../types';
import { transitionLimits } from '../../visual/transitions';

/** Keep each affected cut's transition, shortening only when its new source or timing requires it.
 * The caller validates the complete edit (including linked layers and locks) before committing. */
export function fitChangedTransitions(project: Project, ops: ReadonlyArray<Op>): ReadonlyArray<Op> {
  const next = applyEdit(createSession(project), { actor: 'user', ops }).project;
  const before = new Map(project.tracks.flatMap(track => track.items.map(item => [item.id, item] as const)));
  const sources = new Map(project.media.map(media => [media.id, media]));
  const nextSources = new Map(next.media.map(media => [media.id, media]));
  const changed = (item: Item) => {
    const old = before.get(item.id);
    const source = item.media ? sources.get(item.media) : undefined;
    const nextSource = item.media ? nextSources.get(item.media) : undefined;
    return !old || old.media !== item.media || old.in !== item.in || old.duration !== item.duration
      || old.speed !== item.speed || old.loop !== item.loop || old.hold !== item.hold || old.gap !== item.gap
      || source?.path !== nextSource?.path || source?.probe?.frames !== nextSource?.probe?.frames;
  };
  const fitted: Op[] = [...ops];
  for (const track of next.tracks) if (track.role === 'main') track.items.forEach((item, index) => {
    const transition = item.transitionIn, previous = track.items[index - 1];
    if (!transition || !changed(item) && (!previous || !changed(previous))) return;
    const limits = transitionLimits(next, item.id, transition.module);
    if ('problem' in limits) fitted.push({ op: 'set_transition', item: item.id, transition: null });
    else if (transition.duration > limits.max) fitted.push({ op: 'set_transition', item: item.id, transition: { ...transition, duration: limits.max } });
  });
  return fitted;
}
