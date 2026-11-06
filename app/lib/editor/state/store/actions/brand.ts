// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type BrandKit, type BrandScope, type BrandSkill, type BrandActive, type BrandImported, type TemplateInfo } from "@/app/lib/editor/contracts";
import { insertBrief } from "@/app/lib/editor/core/templates/index";
import { isTransportError } from "@/app/lib/editor/state/connection";
import { set, state, currentEditorStore } from "@/app/lib/editor/state/store/core";
import { call, plainError } from "@/app/lib/editor/state/store/server";
import type {EditorCommands} from '@/app/lib/editor/commands';
import { nextPanelSeq } from "@/app/lib/editor/state/store/agentUi";
import {selectBrandFiles,releaseBrandFiles} from '@/app/lib/editor/browser-files';

const templateRequests = new WeakMap<object, symbol>();
type BrandMutation='brand.saveKit'|'brand.deleteKit'|'brand.saveSkill'|'brand.deleteSkill'|'brand.setActive';
async function changeBrand<K extends BrandMutation>(command:K,input:EditorCommands[K]['input']){
  const owner=currentEditorStore();
  if(!owner||state.connection!=='connected'||!state.biliupCanEdit)return false;
  owner.set({brandError:undefined});
  const result=await call(command,input);
  if(currentEditorStore()!==owner)return false;
  if(result.ok)owner.set({brand:result.value,brandError:undefined});
  else owner.set({brandError:plainError(result.error)});
  return result.ok;
}

export const brandActions = {
  // ---- brand kits and skills (D186) ----
  /** Reads the brand state again (the project's kits and skills, the user's, and its choice). */
  async brandGet() {
    const owner=currentEditorStore();
    if(!owner)return;
    const result = await call("brand.get", {});
    if(currentEditorStore()!==owner)return;
    if (result.ok) set({ brand: result.value });
    else if (!isTransportError(result.error)) set({ brandError: plainError(result.error) });
  },
  /** Each brand change answers with the whole state; true when it was taken. */
  async brandSaveKit(kit: BrandKit) {
    return changeBrand("brand.saveKit", { kit });
  },
  async brandDeleteKit(id: string, scope: BrandScope) {
    return changeBrand("brand.deleteKit", { id, scope });
  },
  async brandSaveSkill(skill: BrandSkill) {
    return changeBrand("brand.saveSkill", { skill });
  },
  async brandDeleteSkill(id: string, scope: BrandScope) {
    return changeBrand("brand.deleteSkill", { id, scope });
  },
  async brandSetActive(active: BrandActive) {
    return changeBrand("brand.setActive", { kit: active.kit, skills: [...active.skills] });
  },
  /** Copies a file into a kit's folder; its name there, or undefined (the reason in brandError). */
  async brandAddFile(kit: string, scope: BrandScope, path: string): Promise<string | undefined> {
    const owner=currentEditorStore();
    if(!owner||state.connection!=='connected'||!state.biliupCanEdit){releaseBrandFiles(path);return;}
    const result = await call("brand.addFile", { kit, scope, path });
    if(currentEditorStore()!==owner)return;
    if (!result.ok) return void set({ brandError: plainError(result.error) });
    set({ brand: result.value.state, brandError: undefined });
    return result.value.name;
  },
  /** D198: logos and fonts from a file, a folder or an https address into a kit's folder; what came in, or undefined (the reason in brandError). */
  async brandImport(kit: string, scope: BrandScope, source: string): Promise<ReadonlyArray<BrandImported> | undefined> {
    const owner=currentEditorStore();
    if(!owner||state.connection!=='connected'||!state.biliupCanEdit){releaseBrandFiles(source);return;}
    const result = await call("brand.import", { kit, scope, source });
    if(currentEditorStore()!==owner)return;
    if (!result.ok) return void set({ brandError: plainError(result.error) });
    const {failed,skipped}=result.value;
    set({ brand: result.value.state, brandError: [failed?.map(f=>`${f.name}: ${f.error}`).join('\n'),skipped?`本次最多导入 24 个文件，另有 ${skipped} 个未导入`:undefined].filter(Boolean).join('\n')||undefined });
    return result.value.files;
  },
  /** Browser-selected files/folders are uploaded to the same kit on local or remote servers. */
  async pickBrandSource(directory=false): Promise<string | undefined> {
    return (await selectBrandFiles(true,directory))[0];
  },
  /** Browser-selected logos, examples and references for upload to the kit. */
  async pickBrandFiles(): Promise<ReadonlyArray<string>> {
    return selectBrandFiles();
  },
  async pickBrandReference():Promise<string|undefined>{
    return (await selectBrandFiles(false,false,'.svg,.png,.jpg,.jpeg,.webp,.gif,.mp4,.mov,.webm',false))[0];
  },
  dismissBrandError: () => set({ brandError: undefined }),
  /**
   * D232 (issue #60): calls a skill in the composer: /name at the start of the draft (once), the
   * Agent tab open and the caret at the end, like Add to chat (D141). Nothing is sent.
   */
  composeSkill(name: string) {
    const draft = state.agentDraft;
    const call = `/${name}`;
    const has = new RegExp(`(^|\\s)${call.replace(/[-/]/g, "\\$&")}(\\s|$)`).test(draft);
    set({ agentDraft: has ? draft : `${call} ${draft.trimStart()}`, agentFocus: state.agentFocus + 1, panelRequest: { panel: "agent", seq: nextPanelSeq() } });
  },
  // ---- templates (issue #111) ----
  /** Reads the templates again (the / menu and the Skills tab show them). */
  async templatesGet() {
    const owner = currentEditorStore();
    if (!owner) return;
    const result = await call("templates.list", {});
    if (currentEditorStore() !== owner) return;
    if (result.ok) set({ templates: result.value });
  },
  /** Saves one of the user's templates; true when it was taken (the reason in brandError otherwise). */
  async templatesSave(template: TemplateInfo) {
    const owner = currentEditorStore();
    if (!owner || state.connection !== 'connected' || !state.biliupCanEdit) return false;
    const result = await call("templates.save", { template });
    if (currentEditorStore() !== owner) return false;
    if (result.ok) set({ templates: result.value, brandError: undefined });
    else set({ brandError: plainError(result.error) });
    return result.ok;
  },
  async templatesDelete(id: string) {
    const owner = currentEditorStore();
    if (!owner || state.connection !== 'connected' || !state.biliupCanEdit) return false;
    const result = await call("templates.delete", { id });
    if (currentEditorStore() !== owner) return false;
    if (result.ok) set({ templates: result.value, brandError: undefined });
    else set({ brandError: plainError(result.error) });
    return result.ok;
  },
  /**
   * A template filled from the open project, put into the composer at `at` (the draft as it is when
   * the fill comes back; the end when left out), the Agent tab open. Nothing is sent: the user edits
   * the brief first. False when it couldn't be filled (the reason in brandError).
   */
  async useTemplate(id: string, at?: number) {
    const owner = currentEditorStore();
    if (!owner) return false;
    const before = owner.getSnapshot().agentDraft;
    const request = Symbol(id);
    templateRequests.set(owner, request);
    const result = await call("templates.fill", { id });
    if (currentEditorStore() !== owner || templateRequests.get(owner) !== request) return false;
    templateRequests.delete(owner);
    if (!result.ok) {
      set({ brandError: plainError(result.error) });
      return false;
    }
    const current = owner.getSnapshot(), draft = current.agentDraft;
    const position = at !== undefined && draft === before ? at : draft.length;
    owner.set({ agentDraft: insertBrief(draft, result.value.text, position), agentFocus: current.agentFocus + 1, brandError: undefined, panelRequest: { panel: "agent", seq: nextPanelSeq() } });
    return true;
  },
};
