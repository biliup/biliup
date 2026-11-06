// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type LibraryState, type SoundFolderInfo, type LibrarySearchResult } from "@/app/lib/editor/contracts";
import { isTransportError } from "@/app/lib/editor/state/connection";
import { getPlayback, playback } from "@/app/ui/editor/preview/playback";
import { type LibraryQuery } from "@/app/lib/editor/core/audio/library";
import { prepareLibrarySearch, finishLibrarySearch } from '@/app/lib/editor/library-search';
import { planAddAtPlayhead, planAddMedia } from "@/app/lib/editor/state/project";
import { set, state, online, currentEditorStore, projectOf } from "@/app/lib/editor/state/store/core";
import { addPlanned, revealItem } from "@/app/lib/editor/state/store/edits";
import { call, plainError } from "@/app/lib/editor/state/store/server";
import {chooseSoundFolder,releaseSoundFolder} from '@/app/lib/editor/sound-library';
import {captureEditorFocus} from '@/app/ui/editor/preview/focus';
import type {EditorCommands} from '@/app/lib/editor/commands';

export const mediaActions = {
  /** Other projects' registered media, paged in creation order. */
  async mediaRecent(input:EditorCommands['media.recent']['input']={}) {
    const result=await call('media.recent',{limit:30,...input});
    if(!result.ok)throw new Error(plainError(result.error));
    return result.value;
  },
  /** Reuse an immutable source through the ordinary project save and undo flow. */
  async mediaRecentImport(id:string) {
    const result=await call('media.recentImport',{id});
    if(!result.ok)throw new Error(plainError(result.error));
    return result.value;
  },
  async cancelProxy(mediaId:string){
    const owner=currentEditorStore()
    const result=await call('media.proxyCancel',{mediaId})
    if(!result.ok&&currentEditorStore()===owner)set({notice:{text:plainError(result.error),transport:isTransportError(result.error)}})
  },
  async retryProxy(mediaId:string){
    const owner=currentEditorStore()
    const result=await call('media.proxyRetry',{mediaId})
    if(!result.ok&&currentEditorStore()===owner)set({notice:{text:plainError(result.error),transport:isTransportError(result.error)}})
  },
  /** A generated result → the timeline at the playhead (see planAddAtPlayhead, D99). */
  addAtPlayhead: (mediaId: string) => addPlanned((project) => planAddAtPlayhead(project, mediaId, getPlayback().frame)),
  /** Media tile → timeline (see planAddMedia); the playhead moves to the new item. */
  addMedia: (mediaId: string) => addPlanned((project) => planAddMedia(project, mediaId, getPlayback().frame)),
  /** The sound library's sources and limit (D134). */
  async libraryState(): Promise<LibraryState | undefined> {
    const result = await call("library.state", {});
    return result.ok ? result.value : undefined;
  },
  /** Read the deployment's uploaded sound folders. */
  async soundFolders(): Promise<ReadonlyArray<SoundFolderInfo> | { readonly error: string }> {
    const result = await call("library.folders", {});
    return result.ok ? result.value : { error: plainError(result.error) };
  },
  /** Upload a browser-selected folder into this editor's sound library; undefined when cancelled. */
  async addSoundFolder(target?:Pick<SoundFolderInfo,'id'|'version'>): Promise<ReadonlyArray<SoundFolderInfo> | { readonly error: string } | undefined> {
    const owner=currentEditorStore();
    if(!owner||!state.snapshot||!online()||!state.biliupCanEdit)return undefined;
    const selection=await chooseSoundFolder();
    if (!selection) return undefined;
    try{
      if(currentEditorStore()!==owner)return undefined;
      if(!online()||!state.biliupCanEdit)return {error:'当前工程不可编辑'};
      const result = await call("library.folderAdd", { selection, ...(target?{target}:{}) });
      return result.ok ? result.value : { error: plainError(result.error) };
    }finally{releaseSoundFolder(selection)}
  },
  async removeSoundFolder(id: string): Promise<ReadonlyArray<SoundFolderInfo> | { readonly error: string }> {
    const result = await call("library.folderRemove", { id });
    return result.ok ? result.value : { error: plainError(result.error) };
  },
  /** An online search (D134): the result, or the error in words. */
  async searchLibrary(query: LibraryQuery): Promise<LibrarySearchResult | { readonly error: string }> {
    const prepared=prepareLibrarySearch(query);
    const result = await call("library.search", { kind: prepared.kind, query: prepared.query, ...(prepared.source ? { source: prepared.source } : {}), ...(prepared.license ? { license: prepared.license } : {}), ...(prepared.length ? { length: prepared.length } : {}), ...(prepared.songs ? { songs: true } : {}), ...(prepared.page ? { page: prepared.page } : {}) });
    return result.ok ? finishLibrarySearch(result.value,query) : { error: plainError(result.error) };
  },
  /**
   * Library sound → the timeline (D134): the server downloads it into the project's media/ (or
   * copies a bundled sound effect, id sfx:<id>) unless it's there already, then the media (with its
   * credits) and the item go in as one edit, at the playhead on a free audio track.
   */
  async addLibrarySound(id: string, music: boolean) {
    const owner=currentEditorStore();
    if (!owner || !state.snapshot || !online() || !state.biliupCanEdit || owner.getSnapshot().libraryAdding) return;
    const frame=Math.max(0,Math.floor(getPlayback().frame)),fps=projectOf(state.snapshot).canvas.fps;
    const unchanged=captureEditorFocus();
    owner.set({ libraryAdding: id });
    try {
      const result = await call("library.add", { id, frame,fps,music });
      if (!result.ok) {
        owner.set({notice:{text:plainError(result.error),transport:isTransportError(result.error)}});
        return;
      }
      if(unchanged()){
        const snapshot=owner.getSnapshot().snapshot;
        if(!snapshot||!projectOf(snapshot).tracks.some(track=>track.items.some(item=>item.id===result.value.itemId)))return;
        playback.pause();playback.select(result.value.itemId);playback.seek(result.value.frame);
        revealItem(result.value.itemId);
      }
    } finally {
      owner.set({ libraryAdding: undefined });
    }
  },
};
