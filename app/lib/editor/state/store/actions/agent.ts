import {editorText} from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { AGENT_IMAGES_MAX, type AgentMention, type AgentImage, type AgentAccess, type AgentAnswer, type StoryboardBeat, type Brief, type Storyboard, type IntendedFinding, type TakeKind, takeUploadName } from "@/app/lib/editor/contracts";
import { playback, getPlayback } from "@/app/ui/editor/preview/playback";
import { findItem } from "@/app/ui/editor/timeline/edit";
import { exportSpans } from "@/app/ui/editor/panels/exportSpans";
import { momentLabel, itemName, mentionCandidates, effectMentionId, type MentionKind } from "@/app/lib/editor/core/text/mentions";
import { agentPrefs, DEFAULT_AGENT_PREFS, skipStoryboardSetting } from "@/app/lib/editor/state/agentPrefs";
import { set, state, online, projectOf, currentEditorStore } from "@/app/lib/editor/state/store/core";
import { call, plainError, uploadOne } from "@/app/lib/editor/state/store/server";
import type {EditorCommands} from "@/app/lib/editor/commands";
import { nextPanelSeq } from "@/app/lib/editor/state/store/agentUi";
import { actions } from "@/app/lib/editor/state/store/actions";
import { queuedProjectChange } from '@/app/lib/editor/state/store/edits';
import type { EditorStore } from '@/app/lib/editor/project-store';

type AgentRequest = EditorCommands['agent.send']['input'];
const sending = new WeakSet<EditorStore>();
const keptMentions = (prompt: string, mentions: ReadonlyArray<AgentMention>) => mentions.filter((m, i) =>
  prompt.includes(`@${m.label}`) && mentions.findIndex(x => x.kind === m.kind && x.id === m.id && x.label === m.label) === i);

async function sendRequest(owner: EditorStore, request: AgentRequest): Promise<boolean> {
  sending.add(owner);
  owner.set({agentError:undefined});
  playback.pause();
  try {
    const result = await call('agent.send', request);
    if (!result.ok) owner.set({agentError:plainError(result.error)});
    return result.ok;
  } catch (error) {
    owner.set({agentError:error instanceof Error?error.message:String(error)});
    return false;
  } finally { sending.delete(owner); }
}

/** A failed submission remains one complete request, independent of a later composer draft. */
async function sendQueued(owner: EditorStore, request: AgentRequest): Promise<boolean> {
  owner.set({agentQueued:undefined});
  const sent = await sendRequest(owner, request);
  if (!sent) owner.set({agentQueued:{request,paused:true}});
  return sent;
}

function agentMessage(prompt:string,mentions:ReadonlyArray<AgentMention>,images:ReadonlyArray<AgentImage>,opts:{readonly access?:AgentAccess}={}):EditorCommands['agent.send']['input'] {
    const { frame, selected, markIn, markOut, also } = getPlayback();
    // export_video's range choices, as the Export dialog would read them now.
    const ranges = state.snapshot ? exportSpans(projectOf(state.snapshot), { markIn, markOut, selected, also }) : {};
    // Only the chips whose @label is still in the prompt.
    const kept = keptMentions(prompt, mentions);
    // The composer's choices for this project (D184): model, effort, access.
    const provider=state.agent?.provider;
    const prefs = state.snapshot ? agentPrefs(state.snapshot.dir,provider??'') : DEFAULT_AGENT_PREFS;
    // Queue the displayed default too: changing service settings must not silently
    // choose a different model for a message already submitted by the user.
    const model=prefs.model??state.agent?.model;
    return {
        prompt,
        ...(provider ? {provider} : {}),
        frame: Math.floor(frame),
        ...(selected ? { selected } : {}),
        ...(kept.length ? { mentions: kept } : {}),
        ...(model ? { model } : {}),
        ...(prefs.effort ? { effort: prefs.effort } : {}),
        access: opts.access ?? prefs.access,
        ...(images.length ? { images: [...images] } : {}),
        // Settings › Services › Agent, remembered in this browser.
        ...(skipStoryboardSetting() ? { skipStoryboard: true } : {}),
        ...(ranges.marks || ranges.selection ? { ranges } : {}),
      };

}

export const agentActions = {
  async agentMore():Promise<boolean>{
    const owner=currentEditorStore();
    owner?.set({agentError:undefined});
    const result=await call("agent.more",{});
    if(!result.ok)owner?.set({agentError:plainError(result.error)});
    return result.ok;
  },
  async agentSend(prompt:string,mentions:ReadonlyArray<AgentMention>=[],images:ReadonlyArray<AgentImage>=[],opts:{readonly access?:AgentAccess}={}):Promise<boolean>{
    const owner=currentEditorStore();
    if(!owner||!online()||sending.has(owner)||owner.getSnapshot().agentSending)return false;
    return sendRequest(owner,agentMessage(prompt,mentions,images,opts));
  },
  async agentSubmitDraft():Promise<boolean>{
    const owner=currentEditorStore(), current=owner?.getSnapshot();
    if(!owner||!current||!online()||!current.biliupCanEdit||current.agent?.configured!==true||current.agentSending||sending.has(owner))return false;
    const text=current.agentDraft.trim();
    if(!text)return false;
    const previous=current.agentQueued?.request;
    const prompt=previous?`${previous.prompt}\n\n${text}`:text;
    const images=[...(previous?.images??[]),...current.agentImages];
    if(images.length>AGENT_IMAGES_MAX){
      owner.set({agentError:editorText('At most {0} images per message.',{0:AGENT_IMAGES_MAX})});
      return false;
    }
    const mentions=keptMentions(prompt,[...(previous?.mentions??[]),...current.agentMentions]);
    const request:AgentRequest=previous?{...previous,prompt,mentions,images}:agentMessage(prompt,mentions,images);
    owner.set({agentDraft:'',agentMentions:[],agentImages:[],agentError:undefined});
    if(current.agent.running){owner.set({agentQueued:{request}});return true;}
    return sendQueued(owner,request);
  },
  async agentRetryQueued():Promise<boolean>{
    const owner=currentEditorStore(), current=owner?.getSnapshot();
    if(!owner||!current?.agentQueued?.paused||!online()||!current.biliupCanEdit||current.agent?.configured!==true||current.agent.running||current.agentSending||sending.has(owner))return false;
    return sendQueued(owner,current.agentQueued.request);
  },
  agentEditQueued(){
    const owner=currentEditorStore(),current=owner?.getSnapshot();
    if(!owner||!current?.agentQueued||current.agentSending||sending.has(owner)||current.agentDraft.trim()||current.agentImages.length)return;
    const request=current.agentQueued.request;
    owner.set({agentQueued:undefined,agentDraft:request.prompt,agentMentions:request.mentions??[],agentImages:request.images??[],agentFocus:current.agentFocus+1});
  },
  async agentCancel(){
    const owner=currentEditorStore();
    const result=await call('agent.cancel',{});
    if(!result.ok)owner?.set({agentError:plainError(result.error)});
  },
  async agentRefresh(){const result=await call('agent.subscribe',{});return result.ok;},
  /** The composer's text and chips (D140). */
  /** D202: the pill's jump: the playhead to where the agent changed things (playback carries on from there), its item selected. */
  agentJump: () => {
    const change = state.agentChange;
    if (!change) return;
    set({ agentChange: undefined });
    playback.seek(change.from);
    if (change.select !== undefined) playback.select(change.select);
  },
  dismissAgentChange: () => set({ agentChange: undefined }),
  setAgentDraft: (agentDraft: string, agentMentions?: ReadonlyArray<AgentMention>) => set({ agentDraft, ...(agentMentions ? { agentMentions } : {}) }),
  agentClearQueued: () => set({agentQueued:undefined}),
  /** The composer's attached images (D184). */
  setAgentImages: (agentImages: ReadonlyArray<AgentImage>) => set({ agentImages }),
  /** Add a moment (D183) or the In/Out range of the timeline to the chat, as a chip with its timecode. */
  addMomentToChat(frame: number) {
    const snap = state.snapshot;
    if (!snap) return;
    const label = momentLabel(Math.max(0, Math.floor(frame)), projectOf(snap).canvas.fps);
    actions.addToChat({ kind: "moment", id: `frame:${Math.floor(frame)}`, label, frame: Math.max(0, Math.floor(frame)) });
  },
  addRangeToChat(start: number, end: number) {
    const snap = state.snapshot;
    if (!snap || end <= start) return;
    const fps = projectOf(snap).canvas.fps;
    const label = `${momentLabel(start, fps)}–${momentLabel(end, fps)}`;
    actions.addToChat({ kind: "range", id: `range:${start}-${end}`, label, range: { start: Math.floor(start), end: Math.floor(end) } });
  },
  /**
   * Issue #118: an area of the picture (canvas pixels, dragged over the preview) at a frame or over
   * a stretch, as a chip; the agent gets the rectangle and what's drawn in it (core regionLine).
   */
  addRegionToChat(area: { x: number; y: number; w: number; h: number }, at: { readonly frame: number } | { readonly start: number; readonly end: number }) {
    const snap = state.snapshot;
    if (!snap) return;
    const fps = projectOf(snap).canvas.fps;
    const box = { x: Math.round(area.x), y: Math.round(area.y), w: Math.max(1, Math.round(area.w)), h: Math.max(1, Math.round(area.h)) };
    const frame = "frame" in at ? Math.max(0, Math.floor(at.frame)) : undefined;
    const range = "frame" in at ? undefined : { start: Math.floor(at.start), end: Math.floor(at.end) };
    const time = range ? `${momentLabel(range.start, fps)}–${momentLabel(range.end, fps)}` : momentLabel(frame!, fps);
    // Two areas at the same time get their own labels: the prompt's @label picks the chip.
    const taken = new Set(state.agentMentions.filter((m) => m.kind === "region").map((m) => m.label));
    let label = `Area ${time}`;
    for (let n = 2; taken.has(label); n++) label = `Area ${n} ${time}`;
    const span = range ? `${range.start}-${range.end}` : `${frame}`;
    actions.addToChat({ kind: "region", id: `region:${box.x},${box.y},${box.w},${box.h}@${span}`, label, area: box, ...(range ? { range } : { frame: frame! }) });
  },
  /** Add to chat (D141) for a timeline item: its name, with the stretch of the timeline it covers. */
  addItemToChat(itemId: string, frame?: number) {
    const snap = state.snapshot;
    const placed = snap ? findItem(projectOf(snap), itemId) : undefined;
    if (!snap || !placed) return;
    actions.addToChat({ kind: "item", id: itemId, label: itemName(projectOf(snap) as never, placed.item as never), range: { start: Math.round(placed.start), end: Math.round(placed.start + placed.item.duration) }, ...(frame !== undefined ? { frame: Math.floor(frame) } : {}) });
  },
  /**
   * D264 (#77): Add to chat for anything the composer's @ picker offers (media, tracks, effects,
   * transitions, markers): the same label and id the picker would give, so the agent gets the same
   * <context> line either way. False when it's no longer in the project.
   */
  addRefToChat(kind: MentionKind, id: string): boolean {
    const snap = state.snapshot;
    const c = snap ? mentionCandidates(projectOf(snap) as never).find((x) => x.kind === kind && x.id === id) : undefined;
    if (!c) return false;
    actions.addToChat({ kind: c.kind, id: c.id, label: c.label, ...(c.frame !== undefined ? { frame: c.frame } : {}) });
    return true;
  },
  addMediaToChat: (mediaId: string) => actions.addRefToChat("media", mediaId),
  addTrackToChat: (trackId: string) => actions.addRefToChat("track", trackId),
  addEffectToChat: (itemId: string, module: string) => actions.addRefToChat("effect", effectMentionId(itemId, module)),
  addTransitionToChat: (itemId: string) => actions.addRefToChat("transition", itemId),
  /** D264: words picked in the Transcript, as the stretch of the timeline they're heard over; the chip quotes them. */
  addWordsToChat(start: number, end: number, words: string) {
    const snap = state.snapshot;
    if (!snap || end <= start) return;
    const fps = projectOf(snap).canvas.fps;
    const text = words.replace(/[\r\n<>@]+/g, " ").replace(/\s+/g, " ").trim();
    const quoted = text.length > 40 ? `${text.slice(0, 39).trimEnd()}…` : text;
    const label = quoted ? `"${quoted}" ${momentLabel(start, fps)}` : `${momentLabel(start, fps)}–${momentLabel(end, fps)}`;
    actions.addToChat({ kind: "range", id: `words:${Math.floor(start)}-${Math.floor(end)}`, label, range: { start: Math.floor(start), end: Math.floor(end) } });
  },
  /** Add to chat (D141): a chip at the end of the composer, the Agent tab open, the caret there. */
  addToChat(mention: AgentMention) {
    const draft = state.agentDraft;
    const sep = draft && !/\s$/.test(draft) ? " " : "";
    set({
      agentDraft: `${draft}${sep}@${mention.label} `,
      // The same thing twice is one chip: what each add knew (a frame, a range) is kept.
      agentMentions: [
        ...state.agentMentions.filter((m) => !(m.kind === mention.kind && m.id === mention.id && m.label === mention.label)),
        { ...state.agentMentions.find((m) => m.kind === mention.kind && m.id === mention.id && m.label === mention.label), ...mention },
      ],
      agentFocus: state.agentFocus + 1,
      panelRequest: { panel: "agent", seq: nextPanelSeq() },
    });
  },
  /** Allow or Don't on the running turn's approval card (D99). */
  /** Whether the server accepted the answer; failures leave the card available to retry. */
  async agentApprove(id: string, allow: boolean, extra: { readonly answer?: AgentAnswer; readonly beats?: ReadonlyArray<StoryboardBeat>; readonly note?: string } = {}): Promise<boolean> {
    const owner=currentEditorStore();
    const current=()=>currentEditorStore()===owner&&owner?.getSnapshot().agent?.approval?.id===id;
    if(!owner||!current()||!owner.getSnapshot().biliupCanEdit||owner.getSnapshot().connection!=='connected')return false;
    owner.set({agentError:undefined});
    try {
      const result = await call("agent.approve", { id, allow, ...(extra.answer ? { answer: extra.answer } : {}), ...(extra.beats ? { beats: [...extra.beats] } : {}), ...(extra.note ? { note: extra.note } : {}) });
      if(current()) {
        if(!result.ok)owner.set({agentError:plainError(result.error)});
        else if(!result.value.accepted)owner.set({agentError:editorText('This confirmation has ended. Check the latest status.')});
      }
      return result.ok && result.value.accepted;
    } catch(error) {
      if(current())owner.set({agentError:editorText("Couldn't submit the response: {0}",{0:error instanceof Error?error.message:String(error)})});
      return false;
    }
  },
  /** D188: the user's own brief or storyboard (approved on the Plan card); false with the reason in the Agent tab when it didn't save. */
  async agentPlanSave(input: { readonly brief?: Brief; readonly storyboard?: Storyboard; readonly intended?: ReadonlyArray<IntendedFinding> }, revision=state.agent?.plan?.revision): Promise<boolean> {
    const owner=currentEditorStore();
    if(!owner||!owner.getSnapshot().biliupCanEdit||owner.getSnapshot().connection!=='connected'||owner.getSnapshot().agentPlanSaving)return false;
    if(revision===undefined||revision!==owner.getSnapshot().agent?.plan?.revision){
      owner.set({agentError:editorText('The plan changed. Reopen it before saving your changes.')});
      return false;
    }
    owner.set({agentPlanSaving:true,agentError:undefined});
    try {
      const result = await call("agent.planSave", { revision, ...(input.brief ? { brief: input.brief } : {}), ...(input.storyboard ? { storyboard: input.storyboard } : {}), ...(input.intended ? { intended: [...input.intended] } : {}) });
      if (!result.ok && currentEditorStore()===owner) owner.set({ agentError: plainError(result.error) });
      return result.ok;
    } catch(error) {
      if(currentEditorStore()===owner)owner.set({agentError:editorText("Couldn't save the plan: {0}",{0:error instanceof Error?error.message:String(error)})});
      return false;
    } finally {
      owner.set({agentPlanSaving:false});
    }
  },
  /** D217: a review finding the user marked Intended (the critic stops raising it), or unmarked. */
  async agentMarkIntended(finding: Omit<IntendedFinding, "at">, on = true): Promise<boolean> {
    const now = state.agent?.plan?.intended ?? [];
    const same = (x: IntendedFinding) => x.kind === finding.kind && x.from === finding.from && x.to === finding.to;
    const next = on ? [...now.filter((x) => !same(x as IntendedFinding)), { ...finding, at: new Date().toISOString() }] : now.filter((x) => !same(x as IntendedFinding));
    return actions.agentPlanSave({ intended: next });
  },
  /** D144: the take an allowed screen_record recorded (uploaded), or why there's none. */
  agentScreenTake: (id: string, answer: { readonly path?: string; readonly source?: string; readonly problem?: string },dir?:string):ReturnType<typeof call<'agent.screenTake'>> => {
    if(dir!==undefined&&state.snapshot?.dir!==dir)return Promise.resolve({ok:false,error:{_tag:'NotConnected',message:editorText('Open the original project to save this recording.')}});
    return call('agent.screenTake',{id,...answer});
  },
  /** D144: a take's file up to the active server's media/uploads/ (the agent's screen_record imports it). */
  async uploadTake(blob: Blob, kind: TakeKind, agentId?:string,dir=state.snapshot?.dir,device?:string): Promise<{ readonly path: string } | { readonly message: string }> {
    if(dir!==state.snapshot?.dir)return {message:editorText('Open the original project to save this recording.')};
    const ext = blob.type.includes("mp4") ? "mp4" : "webm";
    return uploadOne(new File([blob], takeUploadName(kind, ext), { type: blob.type }),{kind,dir,agentId,device});
  },
  async agentUndoTurn(turnId: string) {
    const owner=currentEditorStore();
    if (!owner || owner.getSnapshot().agentUndoing) return;
    owner.set({ agentError: undefined, agentUndoing: turnId });
    try {
      const error = await queuedProjectChange(() => call("agent.undoTurn", { turnId }), { inline: true });
      if (currentEditorStore() === owner && error) owner.set({agentError:plainError(error)});
      return error;
    } finally { owner.set({ agentUndoing: undefined }); }
  },
  /** D295: New chat: the next message starts a fresh model conversation. */
  async agentNewThread() {
    const owner=currentEditorStore();
    owner?.set({ agentError: undefined });
    const result = await call("agent.newThread", {});
    if (!result.ok) owner?.set({ agentError: plainError(result.error) });
  },
  dismissAgentError: () => set({ agentError: undefined }),
};
