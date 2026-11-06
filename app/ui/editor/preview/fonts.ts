// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The bundled fonts (D77): DM Sans and DM Mono, inlined by Vite as data: URLs and declared with
// core's fontFaceCss, the same rules the export rasteriser's page uses, so text in either face
// measures the same in the preview and the export. Every face is loaded at start, so the first
// frame that uses one isn't drawn with the fallback.
import { fontFaceCss, FONT_FACES, type FontFile } from "@/app/lib/editor/core/visual/render";
import {editorText} from '@/app/lib/editor/i18n';

const dmSans = '/editor-fonts/DMSans-Variable.woff2';
const dmMonoRegular = '/editor-fonts/DMMono-Regular.woff2';
const dmMonoMedium = '/editor-fonts/DMMono-Medium.woff2';

const URLS: Record<FontFile, string> = { "DMSans-Variable.woff2": dmSans, "DMMono-Regular.woff2": dmMonoRegular, "DMMono-Medium.woff2": dmMonoMedium };

/** Declare the bundled fonts and start loading them; resolves once they're ready. */
let loaded: Promise<unknown> | undefined;
export function loadFonts(): Promise<unknown> {
  if (loaded) return loaded;
  const style = document.createElement("style");
  style.dataset.fonts = "biliup";
  style.textContent = fontFaceCss((file) => URLS[file]);
  document.head.append(style);
  return loaded = Promise.all(FONT_FACES.map((f) => document.fonts.load(`${f.weight.split(" ")[0]} 16px "${f.family}"`).catch(() => [])));
}

// D198: the project's brand kit's font files, for texts and components in the preview (graphics
// get them inlined in their document by the server; the export's rasteriser in its CSS). Fetched
// from the token-protected /media/brand route and added as FontFace objects from their bytes (the
// page's CSP allows only data: font URLs; a FontFace made from bytes loads nothing).
const kitFaces = new Map<string, FontFace>();
let kitEpoch=0;

/** Load together, then publish in the server's font order, just as the export CSS does. */
export async function syncKitFonts(wanted: ReadonlyArray<{ readonly key: string; readonly family: string; readonly weight?: string | undefined; readonly style?:string;readonly url: string }>,signal?:AbortSignal): Promise<string[]> {
  const epoch=++kitEpoch,errors:string[]=[];
  const faces=await Promise.all(
    wanted
      .map(async (w) => {
        const known=kitFaces.get(w.key);if(known)return known;
        try {
          const res = await fetch(w.url,{credentials:'include',signal});
          if (!res.ok) throw new Error('字体文件不可读取');
          const face = new FontFace(w.family, await res.arrayBuffer(), { weight: w.weight??"100 1000",style:w.style??'normal' });
          await face.load();
          return face;
        } catch {if(epoch===kitEpoch&&!signal?.aborted)errors.push(editorText("Couldn't load font: {0}",{0:w.family}))}
      }),
  );
  if(epoch!==kitEpoch||signal?.aborted)return [];
  // Brand faces precede project faces. Fetch completion must not reverse that precedence.
  for(const face of kitFaces.values())document.fonts.delete(face);
  kitFaces.clear();
  for(const [index,face] of faces.entries())if(face){kitFaces.set(wanted[index]!.key,face);document.fonts.add(face)}
  return errors;
}
