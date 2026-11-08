// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Where the pointer is on the timeline's lanes: the lane (track) under a y, the frame under an x.
import type React from "react";
import { fitsTrack, type NewTrackSide } from "@/app/ui/editor/timeline/edit";
import type { Item, ProjectModel, Track } from "@/app/lib/editor/state/project";

export interface LaneGeometryCtx {
  readonly project: ProjectModel;
  readonly lanesRef: React.RefObject<HTMLDivElement | null>;
  readonly laneOrder: ReadonlyArray<string>;
  readonly ppf: number;
}

export type LaneGeometry = ReturnType<typeof makeLaneGeometry>;

export function makeLaneGeometry(ctx: LaneGeometryCtx) {
  const { project, lanesRef, laneOrder, ppf } = ctx;
  const laneEls = () => [...(lanesRef.current?.querySelectorAll<HTMLElement>("[data-testid=lane]") ?? [])];
  /** The lane under a y position: its track, or past the first / last lane. */
  const laneHit = (clientY: number): { track?: Track; side?: NewTrackSide } => {
    const lanes = laneEls();
    if (!lanes.length) return {};
    const first = lanes[0]!.getBoundingClientRect();
    const last = lanes.at(-1)!.getBoundingClientRect();
    if (clientY < first.top) return { side: "top" };
    if (clientY >= last.bottom) return { side: "bottom" };
    for (const lane of lanes) {
      const r = lane.getBoundingClientRect();
      if (clientY >= r.top && clientY < r.bottom) return { track: project.tracks.find((t) => t.id === lane.dataset.track) };
    }
    return {};
  };
  /** The track of the lane under a y position, if an item of this kind may go there. */
  const laneAt = (clientY: number, item: Item): string | undefined => {
    const { track } = laneHit(clientY);
    return track && fitsTrack(item, track) && !track.locked ? track.id : undefined;
  };
  const laneIndexAt = (clientY: number) => {
    const { track } = laneHit(clientY);
    return track ? laneOrder.indexOf(track.id) : -1;
  };

  const frameAt = (clientX: number) => {
    const rect = lanesRef.current!.getBoundingClientRect();
    return (clientX - rect.left) / ppf;
  };
  return { laneEls, laneHit, laneAt, laneIndexAt, frameAt };
}
