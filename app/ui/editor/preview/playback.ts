// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The playhead, the playback clock and the timeline's view state (selection, zoom). Kept out of
// the app store on purpose: during playback the frame changes 60 times a second, and only the
// preview, the timecode and the playhead line should hear about it, not every panel.
//
// Frame f is the moment f / fps (D38). The wall clock drives playback, so a project with no
// media (titles only) plays too. While a video is on screen the clock follows it: if decoding
// starts late, the clock is re-anchored to the video instead of seeking the video (from the
// phase 2 spike).
import { useSyncExternalStore } from "react";

import { layoutOf, type ProjectModel } from "@/app/lib/editor/state/project";
import {booleanPreference} from '@/app/lib/editor/browser-preference';
import type {EditorDocument} from '@/app/lib/editor/project';

/**
 * A local preview of an edit in progress (a canvas drag, a slider, typing): the project after
 * the planned ops, drawn instead of `base` until the edit is sent. Ignored once the open project
 * is no longer `base` (a newer snapshot arrived), so it can never show a stale project.
 */
export type PreviewDraft = {
  readonly base: ProjectModel;
  readonly project: ProjectModel;
} & ({readonly kind?: undefined} | {
  /** History carries the source bindings saved with its timeline, including removed media. */
  readonly kind: 'history';
  readonly sources: EditorDocument['sources'];
});

export interface PlaybackState {
  /** Fractional while playing; whole frames when paused. */
  readonly frame: number;
  readonly playing: boolean;
  /** Frames of the project: playback runs over [0, frames - 1] and stops on the last frame. */
  readonly frames: number;
  /**
   * Where the last item ends (0 when empty). End and seeking can put the playhead here, one past
   * the last frame (nothing is drawn there), so whatever is added at the playhead goes after
   * everything: an end card after the last clip, not over its last frame.
   */
  readonly end: number;
  readonly fps: number;
  readonly selected: string | undefined;
  /**
   * A selected cut on the main track (D73): the id of the clip after it, whose transitionIn the
   * transition is. Exclusive with `selected`; the Inspector shows the cut's transition.
   */
  readonly cut: string | undefined;
  /** Timeline zoom as a multiple of "fit the whole project". */
  readonly zoom: number;
  /** Moves and trims snap to the playhead, item edges and 0 (hold ⌘ while dragging to skip). */
  readonly snapping: boolean;
  /** F11 (D156): overlays follow the footage under them when a main clip's speed or length changes. */
  readonly linkOverlays: boolean;
  readonly draft: PreviewDraft | undefined;
  /** What the bottom panel shows (D32): the timeline or the mixer (D83). */
  readonly bottom: "timeline" | "mixer";
  /** A selected volume key (D83) of the selected item: its index among the item's keys in frame order. ⌫ removes it. */
  readonly gainKey: number | undefined;
  /**
   * Playback speed and direction (J/K/L, G3): 1 is normal play; 2, 4, 8 faster; negative plays
   * backwards. At any rate but 1 the preview seeks its videos per frame and stays silent.
   */
  readonly rate: number;
  /** In and out points (I/O, G3), frames; drawn on the ruler. */
  readonly markIn: number | undefined;
  readonly markOut: number | undefined;
  /** More items selected along with `selected` (⌘A, G3): deleted, copied and duplicated together. */
  readonly also: ReadonlyArray<string>;
  /**
   * Issue #118 (skimming, as in iMovie): the frame under the pointer while it moves over the
   * timeline, drawn in the preview instead of the playhead's while paused; undefined when the
   * pointer leaves (the playhead's frame is drawn again). The playhead itself doesn't move.
   */
  readonly hover: number | undefined;
  /** Issue #118: a media tile's video skimmed in the preview (its seconds), while the pointer is on its thumbnail. */
  readonly hoverMedia: { readonly media: string; readonly seconds: number } | undefined;
  /** Skimming on or off (the timeline toolbar's switch), remembered in this browser. */
  readonly skimming: boolean;
}

const snapping=booleanPreference('timeline.snapping');
const linkOverlays=booleanPreference('timeline.linkOverlays');
const skimming=booleanPreference('timeline.skimming');
let state: PlaybackState = { frame: 0, playing: false, frames: 1, end: 0, fps: 30, selected: undefined, cut: undefined, zoom: 1, snapping: snapping.get(), linkOverlays: linkOverlays.get(), draft: undefined, bottom: "timeline", gainKey: undefined, rate: 1, markIn: undefined, markOut: undefined, also: [], hover: undefined, hoverMedia: undefined, skimming: skimming.get() };
// History borrows this player's clock without moving the editable timeline or its saved view.
let historyPlayback: PlaybackState | undefined;
const listeners = new Set<() => void>();
const notify = () => { for (const listener of listeners) listener(); };
const set = (patch: Partial<PlaybackState>) => {
  state = { ...state, ...patch };
  notify();
};
const subscribe = (listener: () => void) => (listeners.add(listener), () => void listeners.delete(listener));
snapping.subscribe(()=>set({snapping:snapping.get()}));
linkOverlays.subscribe(()=>set({linkOverlays:linkOverlays.get()}));
skimming.subscribe(()=>set({skimming:skimming.get(),hover:undefined,hoverMedia:undefined}));

/** A slice of the playback state; re-renders only when the selected value changes. */
export const usePlayback = <T>(select: (s: PlaybackState) => T): T => useSyncExternalStore(subscribe, () => select(state));
export const getPlayback = () => state;
export const getPreviewPlayback = () => historyPlayback ?? state;
export const usePreviewPlayback = <T>(select: (s: PlaybackState) => T): T => useSyncExternalStore(subscribe, () => select(getPreviewPlayback()));
const setClock = (patch: Partial<Pick<PlaybackState, 'frame'|'playing'|'rate'|'hover'|'hoverMedia'>>) => {
  if (historyPlayback) { historyPlayback = {...historyPlayback, ...patch}; notify(); }
  else set(patch);
};

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
/** The furthest the playhead can go: the end of the project (or its only frame when empty). */
const lastPosition = (s: { frames: number; end: number }) => Math.max(s.frames - 1, s.end);
const ZOOM_MIN = 1 / 8;
const ZOOM_MAX = 400;

// While playing: the frame at wall-clock time t0, and the video follower (registered by the
// preview) that tells where the top visible video actually is.
let clock = { f0: 0, t0: 0 };
let raf = 0;
let follower: ((frame: number) => number | undefined) | undefined;

function tick(now: number) {
  const state = getPreviewPlayback();
  if (!state.playing) return;
  // A rAF timestamp is the frame's start, which can be a little before the performance.now()
  // taken in play(); never step back before where playback started.
  let f = clock.f0 + (Math.max(0, now - clock.t0) * state.fps * state.rate) / 1000;
  if (f <= 0 && state.rate < 0) {
    setClock({ frame: 0, playing: false, rate: 1 });
    return;
  }
  const videoFrame = state.rate === 1 ? follower?.(f) : undefined;
  if (videoFrame !== undefined && Number.isFinite(videoFrame) && Math.abs(videoFrame - f) > 2) {
    f = clamp(videoFrame,0,state.frames-1);
    clock = { f0: f, t0: now };
  }
  if (state.rate > 0 && f >= state.frames - 1) {
    setClock({ frame: state.frames - 1, playing: false, rate: 1 });
    return;
  }
  setClock({ frame: f });
  raf = requestAnimationFrame(tick);
}

/** One transport for the current picture, including a temporary history preview. */
export const previewPlayback = {
  play(rate = 1) {
    const state = getPreviewPlayback();
    if (state.frames <= 1 || !Number.isFinite(rate) || rate === 0) return;
    if (state.playing && state.rate === rate) return;
    if (state.playing) {
      clock = {f0:state.frame,t0:performance.now()};
      return setClock({rate});
    }
    const frame = rate > 0 && state.frame >= state.frames-1 ? 0 : rate < 0 && state.frame <= 0 ? state.frames-1 : Math.min(state.frame,state.frames-1);
    clock = {f0:frame,t0:performance.now()};
    setClock({frame,playing:true,rate,hover:undefined,hoverMedia:undefined});
    raf = requestAnimationFrame(tick);
  },
  pause() {
    const state = getPreviewPlayback();
    if (!state.playing) return;
    cancelAnimationFrame(raf);
    setClock({playing:false,frame:Math.floor(state.frame),rate:1});
  },
  shuttle(direction:1|-1) {
    const state = getPreviewPlayback(), rate = state.playing ? state.rate : 0;
    previewPlayback.play(Math.sign(rate) === direction ? Math.min(8,Math.abs(rate)*2)*direction : direction);
  },
  toggle() {
    const state = getPreviewPlayback();
    if (state.playing) previewPlayback.pause();
    else if (state.hover !== undefined) previewPlayback.playFrom(state.hover);
    else previewPlayback.play();
  },
  playFrom(frame:number) {
    previewPlayback.pause(); previewPlayback.seek(frame); previewPlayback.play();
  },
  seek(frame:number) {
    if (!Number.isFinite(frame)) return;
    const state = getPreviewPlayback(), f = clamp(Math.round(frame),0,lastPosition(state));
    if (state.playing) clock = {f0:f,t0:performance.now()};
    setClock({frame:f,hover:undefined,hoverMedia:undefined});
  },
  step(frames:number) {
    previewPlayback.pause(); previewPlayback.seek(Math.floor(getPreviewPlayback().frame)+frames);
  },
  toStart() { previewPlayback.pause(); previewPlayback.seek(0); },
  toEnd() { previewPlayback.pause(); previewPlayback.seek(getPreviewPlayback().end); },
};

export const playback = {
  subscribe,

  endHistory() {
    if (!historyPlayback) return;
    cancelAnimationFrame(raf);
    historyPlayback = undefined;
    set({draft:undefined});
  },

  /**
   * The open project changed: the playhead stays where it is unless the project got shorter than
   * that; then it goes to the new end (so deleting an end card at the end leaves the playhead
   * where the next one goes).
   */
  setProject(frames: number, fps: number, end: number) {
    playback.endHistory();
    if (frames === state.frames && fps === state.fps && end === state.end) return;
    const ratio=fps/state.fps,last=frames-1;
    const position=clamp(state.frame*ratio,0,lastPosition({frames,end}));
    const frame=state.playing?Math.min(last,position):Math.round(position);
    const mark=(value:number|undefined)=>value===undefined?undefined:clamp(Math.round(value*ratio),0,end);
    const marks={markIn:mark(state.markIn),markOut:mark(state.markOut)};
    if (state.playing && (frames<=1 || state.rate>0&&frame>=last || state.rate<0&&frame<=0)) {
      cancelAnimationFrame(raf);
      set({frames,end,fps,frame,playing:false,rate:1,...marks,hover:undefined,hoverMedia:undefined});
    } else {
      if(state.playing)clock={f0:frame,t0:performance.now()};
      set({frames,end,fps,frame,...marks,hover:undefined,hoverMedia:undefined});
    }
  },

  /** Another project opened: back to the start, nothing selected, fit zoom. */
  reset() {
    cancelAnimationFrame(raf);
    historyPlayback = undefined;
    set({ frame: 0, frames:1, end:0, fps:30, playing: false, selected: undefined, cut: undefined, gainKey: undefined, zoom: 1, draft: undefined, rate: 1, markIn: undefined, markOut: undefined, also: [], hover: undefined, hoverMedia: undefined });
  },

  play(rate = 1) {
    playback.endHistory(); previewPlayback.play(rate);
  },

  pause: previewPlayback.pause,

  /** L: play forward, faster with each press (1, 2, 4, 8×); J the same backwards. K pauses. */
  shuttle(direction: 1 | -1) {
    playback.endHistory(); previewPlayback.shuttle(direction);
  },
  setMark(which: "in" | "out", frame: number | undefined) {
    playback.endHistory();
    const value=frame===undefined?undefined:Number.isFinite(frame)?clamp(Math.round(frame),0,state.end):undefined;
    set(which === "in" ? { markIn: value } : { markOut: value });
  },
  /** Select several items at once: the first is the one the Inspector shows. */
  selectMany(ids: ReadonlyArray<string>) {
    playback.endHistory();
    set({ selected: ids[0], also: ids.slice(1), cut: undefined, gainKey: undefined });
  },

  /** Issue #118: play from the skimmed frame (Space while the pointer is on the timeline) or the playhead. */
  toggle() { playback.endHistory(); previewPlayback.toggle(); },

  /** Issue #118: the playhead to `frame`, then play from there ("Play from here", Space while skimming). */
  playFrom(frame: number) {
    playback.endHistory(); previewPlayback.playFrom(frame);
  },

  /** Move the playhead (whole frames). While playing, playback carries on from there. */
  seek(frame: number) {
    playback.endHistory(); previewPlayback.seek(frame);
  },

  step(frames: number) {
    playback.endHistory(); previewPlayback.step(frames);
  },
  toStart() {
    playback.endHistory(); previewPlayback.toStart();
  },
  /** To the end of the project, after its last frame (where an end card goes). */
  toEnd() {
    playback.endHistory(); previewPlayback.toEnd();
  },

  /** The preview's video follower; undefined unregisters. */
  setFollower(fn: ((frame: number) => number | undefined) | undefined) {
    follower = fn;
  },

  select(id: string | undefined) {
    playback.endHistory();
    if (id !== state.selected || state.cut !== undefined || state.gainKey !== undefined || state.also.length) set({selected:id,cut:undefined,gainKey:undefined,also:[]});
  },
  /** Select the cut before main item `id` (its transition, if it has one); undefined clears it. */
  selectCut(id: string | undefined) {
    playback.endHistory();
    if (id !== state.cut || state.selected !== undefined) set({cut:id,selected:undefined,gainKey:undefined,also:[]});
  },
  /** Select item `id` and its volume key `index` (in frame order), D83; undefined selects just the item. */
  selectGainKey(id: string, index: number | undefined) {
    playback.endHistory();
    if (id !== state.selected || index !== state.gainKey || state.cut !== undefined) set({selected:id,cut:undefined,gainKey:index,also:[]});
  },

  zoomIn: () => set({ zoom: clamp(state.zoom * 1.5, ZOOM_MIN, ZOOM_MAX) }),
  zoomOut: () => set({ zoom: clamp(state.zoom / 1.5, ZOOM_MIN, ZOOM_MAX) }),
  zoomFit: () => set({ zoom: 1 }),
  /** T2: the timeline zoom as a value (⌘/pinch wheel), clamped. */
  setZoom: (zoom: number) => set({ zoom: clamp(zoom, ZOOM_MIN, ZOOM_MAX) }),
  /** Snapping on or off; remembered in this browser. */
  toggleSnapping: () => snapping.set(!snapping.get()),
  /** Overlays follow main-track speed and trim changes, or not; remembered in this browser. */
  toggleLinkOverlays: () => linkOverlays.set(!linkOverlays.get()),
  setBottom: (bottom: PlaybackState["bottom"]) => (bottom === state.bottom ? undefined : set({ bottom })),
  /**
   * Issue #118: skim to `frame` (the pointer over the timeline), or with undefined stop skimming.
   * Ignored while playing or with skimming off; whole frames, within the project.
   */
  hover(frame: number | undefined) {
    if (historyPlayback) return;
    const f = frame === undefined || state.playing || !state.skimming ? undefined : clamp(Math.round(frame), 0, lastPosition(state));
    if (f !== state.hover || state.hoverMedia) set({ hover: f, hoverMedia: undefined });
  },
  /** Issue #118: skim a media tile's video at `seconds`, or with undefined stop. */
  hoverMedia(media: string | undefined, seconds = 0) {
    if (historyPlayback) return;
    const next = media === undefined || state.playing || !state.skimming ? undefined : { media, seconds };
    if (next?.media === state.hoverMedia?.media && next?.seconds === state.hoverMedia?.seconds) return;
    set({ hoverMedia: next, hover: undefined });
  },
  /** Skimming on or off; remembered in this browser. */
  toggleSkimming: () => skimming.set(!skimming.get()),
  /** Show (or with undefined, stop showing) a local preview of an edit in progress. */
  setDraft(draft: PreviewDraft | undefined) {
    if (draft === state.draft) return;
    if (draft?.kind === 'history') {
      previewPlayback.pause();
      const {frames,end} = layoutOf(draft.project), fps = draft.project.canvas.fps;
      historyPlayback = {...state,frames,end,fps,draft,frame:clamp(Math.round(state.frame*fps/state.fps),0,frames-1),
        playing:false,rate:1,selected:undefined,also:[],cut:undefined,gainKey:undefined,
        markIn:undefined,markOut:undefined,hover:undefined,hoverMedia:undefined};
      set({draft,hover:undefined,hoverMedia:undefined});
    } else {
      if (historyPlayback) cancelAnimationFrame(raf);
      historyPlayback = undefined;
      set({draft});
    }
  },
};
