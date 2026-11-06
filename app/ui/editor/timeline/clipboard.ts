// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Editor shortcuts' planners (G3, D118): edit points for ↑/↓, deleting several items, and the
// in-app clipboard (⌘C/⌘X/⌘V, ⌘D). Pure: they turn the project and a gesture into edit ops the
// server validates, like edit.ts.
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { keepMainTimes } from "@/app/lib/editor/core/timeline/arrange";
import {usedMediaIds} from '@/app/lib/editor/core/timeline/mediaPlan';
import {editorText} from '@/app/lib/editor/i18n';

import { layoutOf, uniqueId, type Item, type ProjectModel, type Track } from "@/app/lib/editor/state/project";
import { draftOf, findItem, fitsTrack, homeKind, lockOf, type Plan } from "@/app/ui/editor/timeline/edit";

/** Every frame where something starts or ends (0 included), sorted: where ↑/↓ jump. */
export function editPoints(project: ProjectModel): Array<number> {
  const { starts } = layoutOf(project);
  const points = new Set<number>([0]);
  for (const t of project.tracks)
    for (const i of t.items) {
      if (i.type === "captions") continue;
      const s = starts.get(i.id) ?? 0;
      points.add(s);
      points.add(s + i.duration);
    }
  return [...points].sort((a, b) => a - b);
}

/** The edit point before (-1) or after (1) `frame`; undefined at the ends. */
export function nextEditPoint(project: ProjectModel, frame: number, direction: 1 | -1): number | undefined {
  const points = editPoints(project);
  const f = Math.round(frame);
  return direction > 0 ? points.find((p) => p > f) : points.findLast((p) => p < f);
}

/** Delete several items in one edit (D224: they leave their space, on main too; a transition into a clip that now follows a gap goes). */
export function planDeleteMany(project: ProjectModel, ids: ReadonlyArray<string>): Plan | { readonly problem: string } {
  const placed = [...new Set(ids)].map((id) => findItem(project, id)).filter((p) => p !== undefined);
  if (!placed.length) return { problem: "Select an item to delete it." };
  for (const p of placed) {
    const locked = lockOf(p);
    if (locked) return { problem: locked };
  }
  const gone = new Set(placed.map((p) => p.item.id));
  const ops: Array<Op> = [];
  for (const p of placed) {
    if (p.track.role !== "main") continue;
    // The next clip that stays now follows a gap (or another clip): its transition goes.
    const next = p.track.items.slice(p.index + 1).find((i) => !gone.has(i.id));
    if (next?.transitionIn && !ops.some((o) => o.op === "set_transition" && o.item === next.id)) ops.push({ op: "set_transition", item: next.id, transition: null });
  }
  for (const p of placed) ops.push({ op: "delete_item", item: p.item.id });
  const main = project.tracks.find((t) => t.role === "main");
  if (main && placed.some((p) => p.track.id === main.id)) ops.push(...keepMainTimes(main, gone));
  return { ops, summary: placed.length === 1 ? `Delete ${placed[0]!.item.id}` : `Delete ${placed.length} items` };
}

/** What ⌘C keeps: the items (as they were), their tracks and where they started relative to the first. */
export interface Clip {
  readonly item: Item;
  readonly track: string;
  readonly kind: Track["kind"];
  readonly purpose?: Track["purpose"];
  readonly main: boolean;
  /** Source stacking order, including when its track is deleted after Copy. */
  readonly trackOrder: number;
  readonly belowMain: boolean;
  readonly offset: number;
}

export function linkedSelection(project:ProjectModel,ids:ReadonlyArray<string>):string[]{
  // Cutting a leader removes its linked parts too, so copying it must retain them.
  // Groups and links can lead to each other; expand the selection to a fixed point.
  const wanted=new Set(ids),items=project.tracks.flatMap(track=>track.items);
  for(let changed=true;changed;){
    changed=false;
    const groups=new Set(items.filter(item=>wanted.has(item.id)&&item.group).map(item=>item.group));
    for(const item of items)if(!wanted.has(item.id)&&((item.group&&groups.has(item.group))||(item.linkedTo&&wanted.has(item.linkedTo)))){
      wanted.add(item.id);changed=true;
    }
  }
  return [...wanted];
}

export function copyItems(project: ProjectModel, ids: ReadonlyArray<string>): Array<Clip> {
  const placed = linkedSelection(project,ids).map((id) => findItem(project, id)).filter((p) => p !== undefined && p.item.type !== "captions");
  if (!placed.length) return [];
  const first = Math.min(...placed.map((p) => p!.start));
  const mainAt = project.tracks.findIndex(track => track.role === "main");
  return placed
    .sort((a, b) => a!.start - b!.start)
    .map((p) => {
      const trackOrder = project.tracks.indexOf(p!.track);
      return { item: structuredClone(p!.item) as Item, track: p!.track.id, kind: p!.track.role === "main" ? homeKind(p!.item) : p!.track.kind, ...(p!.track.purpose ? {purpose:p!.track.purpose} : {}), main: p!.track.role === "main", trackOrder, belowMain: mainAt >= 0 && trackOrder > mainAt, offset: p!.start - first };
    });
}

const NEW_TRACK_NAMES: Readonly<Record<Track["kind"], string>> = { video: "Video", text: "Text", audio: "Audio" };

/**
 * Paste clips at `frame` (⌘V) in one edit. Keep each copied track together, reusing its original
 * track only when the whole pasted range fits. Otherwise create an adjacent lane in source
 * stacking order instead of scattering clips among arbitrary empty tracks. A main-track clip is
 * inserted on the main track after the clip under the playhead (the rest moves along). Linked
 * layers follow the copied leader's actual position. Spacing within the selection is retained,
 * including gaps between selected main clips. Internal transitions stay with copied adjacent
 * clips; new boundary cuts have no inherited transition. A copied layer without its leader
 * is independent. Item and group identities are always new.
 */
export function planPaste(project: ProjectModel, clips: ReadonlyArray<Clip>, frame: number): (Plan & { readonly ids: ReadonlyArray<string> }) | { readonly problem: string } {
  if (!clips.length) return { problem: "Copy an item first (⌘C)." };
  const copiedTracks=clips.map(clip=>({id:clip.track,name:clip.track,kind:clip.kind,items:[clip.item]}));
  if([...usedMediaIds({tracks:copiedTracks})].some(id=>!project.media.some(media=>media.id===id)))return {problem:editorText('A copied source is no longer in this project. Restore it or copy the clips again.')};
  const { starts } = layoutOf(project);
  const at = Math.max(0, Math.round(frame));
  const taken = new Set(project.tracks.flatMap(track=>track.items.flatMap(item=>item.group?[item.group]:[])));
  const ops: Array<Op> = [];
  const ids: Array<string> = [];
  const copiedIds=new Map<string,string>();
  for(const clip of clips){
    const id=uniqueId(project,clip.item.id.replace(/-\d+$/, '')||'item',taken);
    taken.add(id);ids.push(id);copiedIds.set(clip.item.id,id);
  }
  const groups = new Map<string, string>();
  const copies=new Map(clips.map(clip=>{
    const id=copiedIds.get(clip.item.id)!;
    const {transitionIn:_t,start:_s,gap:_g,group:oldGroup,linkedTo:oldLink,...rest}=clip.item;
    if(oldGroup&&!groups.has(oldGroup)){
      const group=uniqueId(project,'group',taken);taken.add(group);groups.set(oldGroup,group);
    }
    const linkedTo=oldLink&&clips.some(parent=>parent.item.id===oldLink&&parent.item.type==='clip')?copiedIds.get(oldLink):undefined;
    return [clip.item.id,{...rest,id,...(oldGroup?{group:groups.get(oldGroup)!}:{}),...(linkedTo?{linkedTo}:{})} as Item] as const;
  }));
  const main = project.tracks.find((t) => t.role === "main");
  let mainIndex = main ? main.items.filter((i) => (starts.get(i.id) ?? 0) < at).length : 0;
  const following = main?.items[mainIndex];
  const onMain=new Set<string>();
  let previousMain: Clip | undefined;
  // Place the main items first. Their insertion point can differ from the playhead.
  for(const clip of clips){
    const item=copies.get(clip.item.id)!;
    if(clip.main&&main&&!main.locked&&fitsTrack(item,main)){
      const gap = previousMain ? Math.max(0, clip.offset - previousMain.offset - previousMain.item.duration) : 0;
      const transitionIn = previousMain && gap === 0 ? clip.item.transitionIn : undefined;
      ops.push({op:'insert_item',track:main.id,index:mainIndex++,item:{...item,...(gap>0?{gap}:{}),...(transitionIn?{transitionIn}:{})}});onMain.add(clip.item.id);
      previousMain = clip;
    }
  }
  if (onMain.size && following?.transitionIn) ops.push({op:'set_transition',item:following.id,transition:null});
  // Existing linked layers can move when Main ripples; find free space after that move.
  let projected=draftOf(project,ops);
  if('problem' in projected)return projected;
  const firstMain=clips.find(clip=>onMain.has(clip.item.id));
  if(firstMain){
    const firstId=copiedIds.get(firstMain.item.id)!;
    const leading=firstMain.offset-layoutOf(projected.project).starts.get(firstId)!;
    if(leading>0){
      // A grouped title can begin before its video. Reserve that space at the
      // beginning of Main instead of clipping the title or separating the group.
      const index=ops.findIndex(op=>op.op==='insert_item'&&op.item.id===firstId);
      const op=ops[index]!;
      if(op.op==='insert_item')ops[index]={...op,item:{...op.item,gap:leading}};
      projected=draftOf(project,ops);
      if('problem' in projected)return projected;
    }
  }
  const projectedStarts=layoutOf(projected.project).starts;
  const base=firstMain?projectedStarts.get(copiedIds.get(firstMain.item.id)!)!-firstMain.offset:at;
  const placed=new Map(clips.filter(clip=>onMain.has(clip.item.id)).map(clip=>[clip.item.id,projectedStarts.get(copiedIds.get(clip.item.id)!)!]));
  const positioned: Array<{clip: Clip; item: Item; from: number}> = [];
  const pending=clips.filter(clip=>!onMain.has(clip.item.id));
  while(pending.length){
    // A linked camera/audio/danmaku layer follows its copied leader, not the original.
    const index=pending.findIndex(clip=>!copies.get(clip.item.id)!.linkedTo||placed.has(clip.item.linkedTo!));
    if(index<0)return {problem:editorText('The copied clips contain a circular link. Unlink them before copying.')};
    const clip=pending.splice(index,1)[0]!,item=copies.get(clip.item.id)!;
    const parent=item.linkedTo?clips.find(parent=>parent.item.id===clip.item.linkedTo):undefined;
    const from=parent?placed.get(parent.item.id)!+clip.offset-parent.offset:base+clip.offset;
    if(from<0)return {problem:editorText('There is not enough room before the pasted clip. Paste later in the timeline.')};
    positioned.push({clip,item,from});
    placed.set(clip.item.id,from);
  }
  const lanes = new Map<string, typeof positioned>();
  for (const entry of positioned) {
    const key = JSON.stringify([entry.clip.track, entry.clip.kind, entry.clip.purpose]);
    const lane = lanes.get(key);
    if (lane) lane.push(entry);
    else lanes.set(key, [entry]);
  }
  const tracks: Array<Track> = [...projected.project.tracks];
  let below: string | undefined;
  // Bottom to top: a new lane can be inserted before the already placed lower lane.
  // Neither the order of clicking a selection nor its clips' start times determines z-order.
  for (const lane of [...lanes.values()].sort((a,b) => b[0]!.clip.trackOrder - a[0]!.clip.trackOrder)) {
    const clip = lane[0]!.clip;
    const mainAt = tracks.findIndex(track => track.role === "main");
    const min = mainAt >= 0 && clip.belowMain ? mainAt + 1 : 0;
    const max = Math.min(below ? tracks.findIndex(track => track.id === below) : tracks.length, mainAt >= 0 && !clip.belowMain ? mainAt : tracks.length);
    const own = projected.project.tracks.find(track => track.id === clip.track && track.role !== "main");
    const ownAt = own ? tracks.indexOf(own) : -1;
    const fits = (track: Track) => !track.locked && track.purpose === clip.purpose && lane.every(({item,from}) => fitsTrack(item,track) && track.items.every(existing => {
      const start = projectedStarts.get(existing.id) ?? 0;
      return start + existing.duration <= from || from + item.duration <= start;
    }));
    let track = own && ownAt >= min && ownAt < max && fits(own) ? own : undefined;
    if (!track) {
      const tid = uniqueId(project, clip.kind, taken);
      taken.add(tid);
      const count = tracks.filter((t) => t.kind === clip.kind && t.purpose === clip.purpose).length;
      const name = `${editorText(clip.purpose === "subtitles" ? "Subtitles" : NEW_TRACK_NAMES[clip.kind])} ${count + 1}`;
      const preferred = ownAt >= 0 ? ownAt : mainAt >= 0 ? (clip.belowMain ? tracks.length : mainAt) : tracks.length;
      const index = Math.max(min, Math.min(preferred, max));
      const created = { id: tid, kind: clip.kind, name, ...(clip.purpose ? {purpose:clip.purpose} : {}) };
      ops.push({ op: "add_track", track: created, index });
      track = { ...created, items: [] };
      tracks.splice(index, 0, track);
    }
    for (const {item,from} of lane) ops.push({ op: "insert_item", track: track.id, item: { ...item, start: from } as Item });
    below = track.id;
  }
  const result=draftOf(project,ops);
  if('problem' in result)return result;
  return { ops, summary: ids.length === 1 ? `Paste ${ids[0]}` : `Paste ${ids.length} items`, ids };
}
