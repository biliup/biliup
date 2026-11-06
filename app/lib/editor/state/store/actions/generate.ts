// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type KeyProvider } from "@/app/lib/editor/contracts";
import { copyText } from "@/app/lib/editor/clipboard";
import { set, currentEditorStore } from "@/app/lib/editor/state/store/core";
import { call, plainError } from "@/app/lib/editor/state/store/server";
import type { GenerateDraft } from "@/app/lib/editor/state/store/types";
import { actions } from "@/app/lib/editor/state/store/actions";
import { nextPanelSeq } from "@/app/lib/editor/state/store/agentUi";
import {generationRunning} from '@/app/lib/editor/generation';

let draftSeq = 0;
const catalogRequests=new WeakMap<object,symbol>();

export const generateActions = {
  // ---- generators and keys (D99) ----
  async listGenerators() {
    const owner = currentEditorStore();
    if(!owner)return;
    const request=Symbol();catalogRequests.set(owner,request);owner.set({generatorsError:undefined});
    try{
      const result = await call("generate.list", {});
      if(currentEditorStore()!==owner||catalogRequests.get(owner)!==request)return;
      if(result.ok)owner.set({generators:result.value,generatorsError:undefined});
      else owner.set({generatorsError:plainError(result.error)});
    }catch(error){
      if(currentEditorStore()===owner&&catalogRequests.get(owner)===request)owner.set({generatorsError:error instanceof Error?error.message:String(error)});
    }finally{if(catalogRequests.get(owner)===request)catalogRequests.delete(owner);}
  },
  async startGenerate(draft: GenerateDraft) {
    const owner = currentEditorStore();
    if (!owner) return;
    const before=owner.getSnapshot();
    if(before.connection!=='connected'||before.generateStarting||!before.biliupCanEdit||generationRunning(before.generateJobs,draft))return;
    owner.set({ generateError: undefined, generateStarting: true });
    try {
      const result = await call("generate.start", { generator: draft.generator, prompt: draft.prompt.trim(), ...(draft.kind ? { kind: draft.kind } : {}), ...(draft.size ? { size: draft.size } : {}), ...(draft.seconds !== undefined ? { seconds: draft.seconds } : {}) });
      if (currentEditorStore() !== owner) return;
      if (!result.ok) {
        owner.set({ generateError: plainError(result.error) });
        void actions.listGenerators();
      }
    } catch(error) {
      if(currentEditorStore()===owner)owner.set({generateError:error instanceof Error?error.message:String(error)});
    } finally {
      owner.set({ generateStarting: false });
    }
  },
  async cancelGenerate(jobId: string) {
    const owner=currentEditorStore();
    set({generateError:undefined});
    const result=await call("generate.cancel", { jobId });
    if(!result.ok&&currentEditorStore()===owner)set({generateError:plainError(result.error)});
  },
  async importGenerated(jobId:string) {
    const owner=currentEditorStore();
    set({generateError:undefined});
    const result=await call("generate.import",{jobId});
    if(!result.ok&&currentEditorStore()===owner)set({generateError:plainError(result.error)});
  },
  /** "Edit prompt" on a generated result: the Generate tab with its generator, prompt and settings. */
  editPrompt(draft: GenerateDraft) {
    set({ generateDraft: { ...draft, seq: ++draftSeq }, generateError: undefined, panelRequest: { panel: "generate", seq: nextPanelSeq() } });
  },
  async listKeys() {
    const owner=currentEditorStore();
    set({ keysError: undefined });
    const result = await call("keys.list", {});
    if(currentEditorStore()!==owner)return;
    if (result.ok) set({ keys: result.value });
    else set({ keysError: plainError(result.error) });
  },
  /** Stores a key (it goes to the server once and never comes back). Returns whether it was saved. */
  async setKey(provider: KeyProvider, key: string) {
    const owner=currentEditorStore();
    set({ keysError: undefined });
    const result = await call("keys.set", { provider, key });
    if(currentEditorStore()!==owner)return result.ok;
    if (!result.ok) {
      set({ keysError: plainError(result.error) });
      return false;
    }
    set({ keys: result.value });
    void actions.listGenerators();
    void actions.checkSetup();
    void actions.agentRefresh();
    return true;
  },
  async removeKey(provider: KeyProvider) {
    const owner=currentEditorStore();
    set({ keysError: undefined });
    const result = await call("keys.remove", { provider });
    if(currentEditorStore()!==owner)return;
    if (result.ok) set({ keys: result.value });
    else set({ keysError: plainError(result.error) });
    void actions.listGenerators();
    void actions.checkSetup();
    void actions.agentRefresh();
  },
  /** Settings › Keys. */
  openKeys() {
    actions.openSettings("services", "keys");
  },
  copy: (text: string) => copyText(text),
};
