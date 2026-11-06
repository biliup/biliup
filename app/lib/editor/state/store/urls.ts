// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { API_BASE } from '@/app/lib/api-streamer';
import {renderAssetsKey} from '@/app/lib/editor/render-assets';
import type { AppStateShape } from "@/app/lib/editor/state/store/types";
import type {Media,Project} from '@/app/lib/editor/core/timeline/types';
import { state } from "@/app/lib/editor/state/store/core";

/** A short, stable tag for a file path (FNV-1a), so a relinked file gets a different URL. */
const pathTag = (path: string) => {
  let h = 0x811c9dc5;
  for (let i = 0; i < path.length; i++) h = Math.imul(h ^ path.charCodeAt(i), 0x01000193);
  return (h >>> 0).toString(36);
};
const graphicBindings = new WeakMap<Project,string>();
const graphicBindingsKey = (project:Project) => {
  let key=graphicBindings.get(project);
  if(key===undefined){key=pathTag(JSON.stringify(project.media.map(media=>[media.id,media.path,media.kind])));graphicBindings.set(project,key)}
  return key;
};
const projectMediaUrl = (kind: 'file' | 'thumb' | 'waveform' | 'filmstrip' | 'proxy', id: string, path?: string) => {
 const project = state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];
 if (!project) return undefined;
 return API_BASE + '/v1/editor/projects/' + project + '/media/' + encodeURIComponent(id) + '/' + kind + (path ? '?v=' + pathTag(path)+'&path='+encodeURIComponent(path) : '');
};
/** A brand kit file served through the current application's authenticated API. */
export const brandFileUrl = (kit: string, name: string, path?: string) => {
  const project=state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];if(!project)return undefined;
  if(path?.startsWith(`/v1/editor/projects/${project}/brand/`))return API_BASE+path;
  const scope=state.brand?.kits.find(value=>value.id===kit)?.scope;if(!scope)return undefined;
  return `${API_BASE}/v1/editor/projects/${project}/brand/${scope}/${encodeURIComponent(kit)}/files/${encodeURIComponent(name)}`;
};
/** Immutable assistant evidence is scoped to the current saved project. */
export const agentImageUrl = (name: string) => {
  const project=state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];
  return project?`${API_BASE}/v1/editor/projects/${project}/agent/images/${encodeURIComponent(name)}`:undefined;
};
/** The poster for a media item, scoped to the saved project. */
export const thumbUrl = (id: string, path?: string) => projectMediaUrl('thumb', id, path);
/** D205: a poster from the user's media index. */
export const recentThumbUrl = (id: string) => {
  const project=state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];
  return project?`${API_BASE}/v1/editor/projects/${project}/recent/${encodeURIComponent(id)}/thumb`:undefined;
};
/** The original file of a media item, streamed by the server with byte ranges (never file://). */
export const mediaUrl = (id: string, path?: string) => projectMediaUrl('file', id, path);
/** D212: a media item's preview proxy (tagged with the key of the file it was made from). */
export const proxyUrl = (id: string, key: string) => {
  const url = projectMediaUrl('proxy', id);
  return url ? `${url}?v=${encodeURIComponent(key)}` : undefined;
};
/** D212: a media item's proxy state in the open project. */
export const proxyOf = (s: AppStateShape, media: Pick<Media,'id'|'path'>) => s.proxies.find(p => p.mediaId === media.id && p.path === media.path);
/** A sound of the bundled sound-effects pack (D134), for the Sound effects tab's preview. */
export const sfxUrl = (file: string) => `/editor-sfx/${encodeURIComponent(file)}`;
/** A file of the user's sound folders (D208), by its id (the server knows its path). */
export const folderSoundUrl = (id: string) => {
 const project=state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];
 return project?`${API_BASE}/v1/editor/projects/${project}/library/sounds/${encodeURIComponent(id)}/file`:undefined;
};
/** An online library hit's preview (D134), streamed through the server (only hits of a recent search). */
export const libraryPreviewUrl = (source: string, id: string) => `${API_BASE}/v1/editor/library/preview/${encodeURIComponent(source)}/${encodeURIComponent(id)}`;
/** The peaks of a media item's sound, made and cached by the server (D54). */
export const waveformUrl = (id: string, path?: string) => projectMediaUrl('waveform', id, path);
/** Issue #53: a video's filmstrip sprite for the timeline, made and cached by the server. */
export const filmstripUrl = (id: string, path?: string) => projectMediaUrl('filmstrip', id, path);
/** A graphic item's document (D78), by its src ("graphics/x.html"), for the preview's sandboxed iframe. */
export const graphicUrl = (src: string, shown: Project) => {
  const project=state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];
  if(!project||!/^graphics\/[A-Za-z0-9][A-Za-z0-9._-]{0,80}\.html$/.test(src))return undefined;
  const {canvas}=shown;
  // A graphic caches biliup.frame() results inside its iframe. Changing source bindings must
  // replace that iframe too, even when its immutable HTML document is unchanged.
  const bindings=graphicBindingsKey(shown);
  return `${API_BASE}/v1/editor/projects/${project}/graphics/${encodeURIComponent(src.slice(9))}?canvas=${canvas.width}x${canvas.height}&fps=${canvas.fps}&media=${bindings}&brand=${pathTag(renderAssetsKey(state.snapshot?.dir,state.brand,state.projectFonts))}`;
};
/** D290: a picture of a media item for a graphic in the preview (its frame at t seconds, fitting w × h); fetched by this page, never shown to the graphic. */
export const graphicMediaUrl = (media: Pick<Media,'id'|'path'>, t: number, w?: number, h?: number, preview?: boolean) => {
  const project=state.snapshot?.dir.match(/^biliup:(\d+)$/)?.[1];
  if(!project)return undefined;
  const u = new URL(`${API_BASE}/v1/editor/projects/${project}/graphic-media/${encodeURIComponent(media.id)}`,location.href);
  u.searchParams.set('path',media.path);
  u.searchParams.set("t", String(t));
  if (w) u.searchParams.set("w", String(Math.round(w)));
  if (h) u.searchParams.set("h", String(Math.round(h)));
  if (preview) u.searchParams.set("p", "1");
  return u.href;
};
