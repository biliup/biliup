import { AGENT_IMAGE_MAX, AGENT_IMAGE_TYPES, AGENT_IMAGES_MAX, type AgentImage, type AgentMention } from '@/app/lib/editor/contracts';
import { mentionCandidates } from '@/app/lib/editor/core/text/mentions';
import { editorText } from '@/app/lib/editor/i18n';
import type { EditorStore } from '@/app/lib/editor/project-store';
import { projectOf } from '@/app/lib/editor/state/store/core';
import type { FileImporter, FileImportResult } from '@/app/lib/editor/state/store/server';

const IMAGE_TYPES: Record<string, AgentImage['mimeType']> = {
  png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp',
};
function imageType(file: File): AgentImage['mimeType'] | undefined {
  if ((AGENT_IMAGE_TYPES as ReadonlyArray<string>).includes(file.type)) return file.type as AgentImage['mimeType'];
  if (!file.type || file.type === 'application/octet-stream') return IMAGE_TYPES[file.name.split('.').pop()?.toLowerCase() ?? ''];
  return undefined;
}
async function readImage(file: File, mimeType: AgentImage['mimeType']): Promise<AgentImage> {
  if (file.size > AGENT_IMAGE_MAX) throw new Error(editorText('{0} is over 5 MB.', { 0: file.name }));
  if (!file.size) throw new Error(editorText('{0} is empty.', { 0: file.name }));
  const url = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(new Error(editorText("Couldn't read {0}.", { 0: file.name })));
    reader.onabort = () => reject(new Error(editorText("Couldn't read {0}.", { 0: file.name })));
    reader.readAsDataURL(file);
  });
  return { name: file.name || editorText('Pasted image'), mimeType, data: url.slice(url.indexOf(',') + 1) };
}

function addImportReferences(owner: EditorStore, dir: string, result: FileImportResult) {
  const state = owner.getSnapshot();
  if (state.snapshot?.dir !== dir) return;
  const candidates = mentionCandidates(projectOf(state.snapshot));
  const mentions = [...state.agentMentions];
  let draft = state.agentDraft;
  for (const reference of result.references) {
    const candidate = candidates.find(item => item.kind === reference.kind && item.id === reference.id);
    if (!candidate) continue;
    const old = mentions.find(mention => mention.kind === reference.kind && mention.id === reference.id);
    if (old && draft.includes(`@${old.label}`)) continue;
    const base = (old?.label ?? candidate.label).replace(/[\r\n\t]+/g, ' ').trim() || editorText('Media');
    let label = base;
    let suffix = 2;
    while (mentions.some(mention => mention.label === label && (mention.kind !== reference.kind || mention.id !== reference.id))) label = `${base} (${suffix++})`;
    const mention: AgentMention = { kind: reference.kind, id: reference.id, label };
    if (!old) mentions.push(mention);
    else mentions[mentions.indexOf(old)] = mention;
    draft += `${draft && !/\s$/.test(draft) ? ' ' : ''}@${label} `;
  }
  if (result.fonts.length) draft += `${draft && !/\s$/.test(draft) ? ' ' : ''}${editorText('Imported fonts: {0}', { 0: result.fonts.join(', ') })} `;
  if (draft !== state.agentDraft) owner.set({ agentDraft: draft, agentMentions: mentions, agentFocus: state.agentFocus + 1 });
}

/** The original project owns async reads/uploads and the resulting draft, even after switching tabs. */
export async function attachComposerFiles(files: ReadonlyArray<File>, owner: EditorStore, importer: FileImporter | undefined): Promise<string[]> {
  const dir = owner.getSnapshot().snapshot?.dir;
  if (!dir) return [editorText('Open a project first.')];
  const problems: string[] = [];
  const media: File[] = [];
  if (files.length > 100) problems.push(editorText('Import up to 100 files at a time. The remaining files were not added.'));
  for (const file of files.slice(0, 100)) {
    if (owner.getSnapshot().snapshot?.dir !== dir) return problems;
    const mime = imageType(file);
    if (!mime) { media.push(file); continue; }
    if (owner.getSnapshot().agentImages.length >= AGENT_IMAGES_MAX) {
      problems.push(editorText('At most {0} images per message.', { 0: AGENT_IMAGES_MAX }));
      continue;
    }
    try {
      const image = await readImage(file, mime);
      const state = owner.getSnapshot();
      if (state.snapshot?.dir !== dir) return problems;
      if (state.agentImages.length < AGENT_IMAGES_MAX) owner.set({ agentImages: [...state.agentImages, image] });
    } catch (error) {
      problems.push(error instanceof Error ? error.message : String(error));
    }
  }
  if (media.length) {
    if (!importer) problems.push(editorText('Open a project first.'));
    else {
      const result = await importer(media);
      if (result) {
        addImportReferences(owner, dir, result);
        problems.push(...result.failed.map(file => `${file.path}: ${file.message}`));
      } else problems.push(editorText('The import ended before the files were added.'));
    }
  }
  return [...new Set(problems)];
}
