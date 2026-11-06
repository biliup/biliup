// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { transitionSeconds } from "@/app/ui/editor/timeline/transitionLength";
import { type AgentUiPanel } from "@/app/lib/editor/contracts";
import { playback, getPlayback } from "@/app/ui/editor/preview/playback";
import { planSplit, findItem, planSetTransition, nearestCut, type TrackFlag, planTrackFlag, planGrouping, planLayer, planRippleDelete, planSpeed, planRename } from "@/app/ui/editor/timeline/edit";
import { planResetPosition } from "@/app/ui/editor/timeline/clip";
import { planPutOnMain, planDetachAudio, planReplaceMedia, takenIds } from "@/app/lib/editor/core/timeline/arrange";
import { planSetLook, planEffect, lookInfo, effectInfo, fxSkips } from "@/app/lib/editor/core/visual/fx";
import { planShader, planShaderValues } from "@/app/lib/editor/core/visual/shaders";
import { planForAll } from "@/app/ui/editor/timeline/multi";
import { type TextPreset, planAddText, planDropMedia, type ProjectModel, uniqueId } from "@/app/lib/editor/state/project";
import { type BackgroundProps, planAddBackground } from "@/app/ui/editor/preview/text";
import { planAddComponent, planAddGraphic } from "@/app/ui/editor/preview/components";
import { planRemoveGainKey } from "@/app/ui/editor/audio/sound";
import { planRemoveMedia } from "@/app/lib/editor/core/timeline/mediaPlan";
import { currentEditorStore, set, state, projectOf } from "@/app/lib/editor/state/store/core";
import { addPlanned, timelineEdit, timelineCommand, itemEdit, shaderDefOf } from "@/app/lib/editor/state/store/edits";
import { nextPanelSeq } from "@/app/lib/editor/state/store/agentUi";
import {RECORDING_MASK,DANMAKU_TOOL,planRecordingEffect} from '@/app/lib/editor/recording-effects';
import { planAddMarker } from '@/app/ui/editor/timeline/edit/markers';
import { editorGestureOrigin } from '@/app/ui/editor/workspace/pointerGesture';
import { linkedSelection, planDeleteMany } from '@/app/ui/editor/timeline/clipboard';
import { editorText } from '@/app/lib/editor/i18n';
import {captureEditorFocus} from '@/app/ui/editor/preview/focus';

export const timelineActions = {
  addMarker: (frame = Math.round(getPlayback().frame)) => {
    const origin = editorGestureOrigin();
    if (origin) return itemEdit(project => planAddMarker(project, frame), { dir: origin.dir, baseRevision: origin.baseRevision });
  },
  /** Open a panel as the agent's ui_show does. */
  openPanel: (panel: AgentUiPanel) => set({ panelRequest: { panel, seq: nextPanelSeq() } }),
  /** Text tab preset → a text item at the playhead (see planAddText). */
  /** Text tab → a preset at the playhead (or the dropped frame), placed where it was dropped on the canvas (D80). */
  addText: (preset: TextPreset, opts: { at?: { x: number; y: number }; frame?: number } = {}) => addPlanned((project) => planAddText(project, preset, opts.frame ?? getPlayback().frame, opts.at)),
  /** A planned move or trim from the timeline (see edit.ts). */
  timelineEdit,
  /** A text or background change from the Inspector or the preview (see text.ts). */
  itemEdit,
  /** Elements tab → a component at the playhead, or where it was dropped (see planAddComponent, D76). */
  addComponent: (id: string, opts: { at?: { x: number; y: number }; frame?: number; props?: Record<string, unknown> } = {}) => {
    const props = opts.props;
    return addPlanned((project) => planAddComponent(project, id, opts.frame ?? getPlayback().frame, { ...(opts.at ? { at: opts.at } : {}), ...(props ? { props } : {}) }));
  },
  /** Elements tab → a full-frame background (see planAddBackground): over the whole edit, or behind the selected item (D220). */
  addBackground: (props: BackgroundProps, behindSelected = false) => addPlanned((project) => planAddBackground(project, props, getPlayback().frame, behindSelected ? getPlayback().selected : undefined)),
  /** S: split at the playhead (see planSplit); the part after the playhead is selected. */
  async split(itemId?: string, at?: number) {
    if (!state.snapshot) return;
    const { selected, frame } = getPlayback();
    const plan = planSplit(projectOf(state.snapshot), itemId ?? selected, at ?? frame);
    return timelineCommand(plan, () => playback.select("newId" in plan ? plan.newId : undefined));
  },
  /** Delete leaves the item's gap. A selected transition or volume key is deleted on its own. */
  async deleteSelected(ids?: ReadonlyArray<string>) {
    if (!state.snapshot) return;
    const project = projectOf(state.snapshot);
    const { selected, also, cut, gainKey } = getPlayback();
    if (!ids && cut !== undefined) {
      const placed = findItem(project, cut);
      if (placed?.item.transitionIn) return timelineCommand(planSetTransition(project, cut, null));
      return;
    }
    // A selected volume key (D83): ⌫ removes the key, not its item.
    if (!ids && selected !== undefined && gainKey !== undefined) {
      return timelineCommand(planRemoveGainKey(project, selected, gainKey), () => playback.selectGainKey(selected, undefined));
    }
    const selectedIds = ids ?? (selected === undefined ? [] : [selected, ...also]);
    return timelineCommand(planDeleteMany(project, linkedSelection(project, selectedIds)), () => playback.select(undefined));
  },
  /**
   * Put a transition on a cut (D73): `cut` (the id of the clip after it), else the selected cut;
   * with `nearest`, the cut nearest the playhead. The cut is selected afterwards, so the Inspector
   * shows it. Replaces a transition already there (keeping its length, clamped).
   */
  async applyTransition(module: string, { cut, nearest = false }: { cut?: string; nearest?: boolean } = {}) {
    if (!state.snapshot) return false;
    const project = projectOf(state.snapshot);
    const target = cut ?? (nearest ? nearestCut(project, getPlayback().frame)?.item.id : getPlayback().cut);
    if (target === undefined) {
      set({ notice: { text: nearest ? "There's no cut yet: put two clips on the main track first." : "Pick a cut first: click between two clips on the main track, drag the transition onto a cut, or use + for the cut nearest the playhead." } });
      return false;
    }
    // D133: the panel's length for all, when one is chosen (else the transition's default).
    const seconds = transitionSeconds();
    const plan = planSetTransition(project, target, { module, ...(seconds ? { duration: Math.round(seconds * project.canvas.fps) } : {}) });
    if ('ops' in plan && !plan.ops.length) { playback.selectCut(target); return false; }
    return timelineCommand(plan, () => playback.selectCut(target));
  },
  /** Change (or with null remove) the transition at the cut before main item `itemId`. */
  setTransition: (itemId: string, next: { module: string; duration?: number } | null, origin?: { baseRevision?: number; dir?: string }) =>
    itemEdit(project => planSetTransition(project, itemId, next), origin),
  toggleTrack: (trackId: string, flag: TrackFlag) => itemEdit(project => planTrackFlag(project, trackId, flag)),
  /** Media dropped on a lane (T1, D122): at the drop frame on that lane when it takes it (see planDropMedia). */
  dropMedia: (mediaId: string, at: { frame: number; track?: string | undefined }) => addPlanned((project) => planDropMedia(project, mediaId, at)),
  /** Detach audio (T3, D123): the clip's sound to its own audio item, the clip muted; the new item is selected. */
  /** D215: a picture off Main (or a take, from any of its parts) goes on the main track, its sounds linked under it. */
  async putOnMain(itemId: string) {
    if (!state.snapshot) return;
    const plan = planPutOnMain(projectOf(state.snapshot), itemId);
    return timelineCommand(plan, () => playback.select("itemId" in plan ? plan.itemId : undefined));
  },
  async detachAudio(itemId: string) {
    if (!state.snapshot) return;
    const project = projectOf(state.snapshot);
    const plan = planDetachAudio(project, itemId, takenIds(project));
    return timelineCommand(plan, () => playback.select("itemId" in plan ? plan.itemId : undefined));
  },
  /**
   * Effects tab (D136): "look:<id>" puts a look on the clip ("look:" takes it off), "effect:<module>"
   * adds an effect; on `itemId` (a drop) or else the selected clip. With several items selected
   * (and the drop on one of them), on every selected clip, as one edit (F2, D147).
   */
  applyFx: (what: string, itemId?: string) => {
    const owner = currentEditorStore(), { selected, also } = getPlayback();
    const unchanged=captureEditorFocus();
    const selection = selected === undefined ? [] : [selected, ...also];
    const targets = itemId === undefined ? selection : selection.length > 1 && selection.includes(itemId) ? selection : [itemId];
    return itemEdit((project) => {
      const [kind, id = ""] = what.split(/:(.*)/s);
      // Shader definitions are registered in this project or stored on its clips.
      const shader = kind === "shader" ? shaderDefOf(project, id) : undefined;
      if (kind === "shader" && !shader) return { problem: "That shader effect isn't available any more." };
      const one = (p: ProjectModel, target: string) => (kind === "look" ? planSetLook(p, target, id || undefined) : kind === "shader" ? planShader(p, target, shader) : [RECORDING_MASK,DANMAKU_TOOL].includes(id)?planRecordingEffect(p,target,id):planEffect(p, target, id));
      const applicable=targets.filter(target=>{
        const item=findItem(project,target)?.item,media=item?.media?project.media.find(media=>media.id===item.media):undefined;
        if(item?.type!=='clip'||!media||media.kind==='audio')return false;
        if(id===DANMAKU_TOOL)return media.kind==='video';
        return id===RECORDING_MASK||!fxSkips(media);
      });
      if(id===DANMAKU_TOOL&&applicable.some(target=>{const media=findItem(project,target)?.item.media;const source=media?state.biliupSources?.[media]:undefined;return source?.kind!=='recording'||!source.hasDanmaku}))return {problem:'This recording has no danmaku file.'};
      if (applicable.length <= 1) return one(project, applicable[0] ?? targets[0] ?? "");
      if (shader) return planForAll(project, applicable, one, (n) => editorText('Apply {0} to {1} clips',{0:shader.name,1:n}));
      const look = lookInfo(id)?.name ?? id;
      const effect = id===RECORDING_MASK?'Mask':id===DANMAKU_TOOL?'Danmaku':effectInfo(id)?.name??id;
      return planForAll(project, applicable, one, (n) => kind==='look'&&!id?editorText('Remove filters from {0} clips',{0:n}):editorText('Apply {0} to {1} clips',{0:editorText(kind==='look'?look:effect),1:n}));
    }).then(changed=>{
      if(changed&&currentEditorStore()===owner&&unchanged()&&what===`effect:${DANMAKU_TOOL}`&&state.snapshot){
        const layers=projectOf(state.snapshot).tracks.flatMap(t=>t.items).filter(item=>item.type==='danmaku'&&item.linkedTo&&targets.includes(item.linkedTo));
        if(layers.length)playback.selectMany(layers.map(item=>item.id));
      }
      return changed;
    });
  },
  /** Change a clip's shader effect's values (D213): one edit. */
  setShaderValues: (itemId: string, values: Readonly<Record<string, number | string | boolean>>) => itemEdit((project) => planShaderValues(project, itemId, values)),
  /** Speed (G9, D125): presets and custom, keep pitch or not; the item's length follows. */
  setSpeed: (itemId: string, speed: number, keepPitch?: boolean) => {
    if (!state.snapshot) return Promise.resolve(false);
    return timelineCommand(planSpeed(projectOf(state.snapshot), itemId, speed, keepPitch, getPlayback().linkOverlays));
  },
  /** Rename a timeline item (T7): its own name, shown on the timeline and in the Inspector; "" goes back to the media's. */
  renameItem: (itemId: string, name: string, origin?: {dir?:string;baseRevision?:number}) => itemEdit(project => planRename(project, {kind: 'item', id: itemId}, name), origin),
  renameTrack: (trackId: string, name: string, origin?: {dir?:string;baseRevision?:number}) => itemEdit(project => planRename(project, {kind: 'track', id: trackId}, name), origin),
  /** G4: a media item's display name (its file stays where it is), shown everywhere it appears. */
  renameMedia: (mediaId: string, name: string, origin?: {dir?:string;baseRevision?:number}) => itemEdit(project => planRename(project, {kind: 'media', id: mediaId}, name), origin),
  /** F8 (D167): a media out of the project with every item using it, in one edit (core planRemoveMedia); its file stays on disk. */
  removeMedia: (mediaId: string, origin: { dir: string; baseRevision: number }) => {
    const snapshot = state.snapshot;
    if (!snapshot || snapshot.dir !== origin.dir) return Promise.resolve(false);
    if (snapshot.revision !== origin.baseRevision) {
      set({ notice: { text: 'The project changed. Try the action again.' } });
      return Promise.resolve(false);
    }
    return itemEdit(project => planRemoveMedia(project, mediaId), origin);
  },
  /** Group or ungroup items (F19, D157): see planGrouping. */
  grouping: (ids: ReadonlyArray<string>, group: boolean) => itemEdit((project) => planGrouping(project, ids, group)),
  /** Bring forward / Send backward (F15, D151): one layer up or down (see planLayer). */
  layer: (itemId: string, dir: -1 | 1) => itemEdit((project) => planLayer(project, itemId, dir)),
  /** Reset position (F15, D151): see planResetPosition. */
  resetPosition: (itemId: string) => itemEdit((project) => planResetPosition(project, itemId)),
  /** Ripple delete (T7): delete and close the gaps on their tracks. */
  async rippleDelete(ids: ReadonlyArray<string>) {
    if (!state.snapshot) return;
    const project = projectOf(state.snapshot);
    return timelineCommand(planRippleDelete(project, linkedSelection(project, ids)), () => playback.select(undefined));
  },
  /** Mute or lock one item (T7): mute silences a clip or audio item (D123), lock keeps hands and the agent off it. */
  toggleItem: (itemId: string, flag: "mute" | "locked") =>
    itemEdit((project) => {
      const item = findItem(project, itemId)?.item;
      if (!item) return { problem: "That item isn't in the project any more." };
      const on = !item[flag];
      return { ops: [{ op: "update_item", item: itemId, set: { [flag]: on ? true : null } }], summary: `${flag === "mute" ? (on ? "Mute" : "Unmute") : on ? "Lock" : "Unlock"} ${item.name ?? itemId}` };
    }),
  /** Replace media (T7): the clip plays another media file of the project from its start, as long as it was (or as the file is). */
  replaceMedia: (itemId: string, mediaId: string) =>
    itemEdit(project => planReplaceMedia(project, itemId, mediaId)),
  /** Add an empty track (T7): video and text tracks go on top (or just above `near`), audio at the bottom (or just below `near`). */
  addTrack: (kind: "video" | "text" | "audio", near?: string) =>
    itemEdit((project) => {
      const base = editorText(kind === "audio" ? "Audio" : kind === "text" ? "Text" : "Video");
      const used = new Set(project.tracks.map((t) => t.name));
      let name = base;
      for (let n = 2; used.has(name); n++) name = `${base} ${n}`;
      const at = near !== undefined ? project.tracks.findIndex((t) => t.id === near) : -1;
      const nearTrack = at >= 0 ? project.tracks[at] : undefined;
      const sameSide = nearTrack && (nearTrack.kind === "audio") === (kind === "audio");
      const index = sameSide ? (kind === "audio" ? at + 1 : at) : kind === "audio" ? project.tracks.length : 0;
      const id = uniqueId(project, `${kind}-track`);
      return { ops: [{ op: "add_track", track: { id, kind, name }, index }], summary: `Add ${name} track` };
    }),
  /** Delete a track and what's on it (T7), as one edit. */
  deleteTrack: (trackId: string) =>
    itemEdit((project) => {
      const track = project.tracks.find((t) => t.id === trackId);
      if (!track) return { problem: "That track isn't in the project any more." };
      if (track.role === "main") return { problem: "The main track can't be deleted." };
      return { ops: [{ op: "delete_track", track: trackId }], summary: `Delete track ${track.name}` };
    }),
  /** Show in Media (T7): the Media tab with the tile scrolled into view and flashed. */
  showInMedia(mediaId: string) {
    set({ panelRequest: { panel: "media" as AgentUiPanel, seq: nextPanelSeq() }, mediaFocus: { id: mediaId, seq: nextPanelSeq() } });
  },
};
