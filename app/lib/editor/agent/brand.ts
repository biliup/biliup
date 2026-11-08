// Adapted from Uncut brand/store.ts and agentPlan.ts (MIT); see licenses/uncut/LICENSE.
import type {BrandState,BrandKit,BrandStyle} from "../contracts";
import {describeBrandFile} from '../brand-import';

export function agentBrandPlan(state:BrandState,args:Record<string,unknown>){
  let base=typeof args.kit==='string'?state.kits.find(k=>k.id===args.kit):undefined;
  if(typeof args.kit==='string'&&!base)throw new Error('品牌不存在，请先用 brand_read 读取列表');
  const named=typeof args.name==='string'?args.name.trim():'';
  if(!base&&named)base=state.kits.find(k=>k.name.trim().toLowerCase()===named.toLowerCase());
  const fresh=!base&&!!named;
  if(!base&&!fresh)base=state.kits.find(k=>k.id===state.active.kit);
  if(!base&&!fresh)throw new Error('请提供已有品牌 kit，或用 name 创建品牌');
  const stem=slug(named);let id=stem;for(let i=2;state.kits.some(k=>k.id===id);i++)id=`${stem}-${i}`;
  const start:BrandKit=base??{id,name:named.slice(0,80),scope:args.scope==='project'?'project':'global',logos:{},colors:[],fonts:[],tone:'',taglines:[],urls:[],dos:[],donts:[],examples:[]};
  const kit=mergeKit(start,args);if('problem'in kit)throw new Error(kit.problem);
  return {base:base??null,kit,fresh,changes:base?kitChanges(base,kit):[]};
}
export function agentBrandImports(kit:BrandKit,names:readonly string[]){
  const logos={...kit.logos},fonts=[...kit.fonts];
  for(const name of names){
    const f=describeBrandFile(name);
    if(f.kind==='logo'&&f.slot&&!logos[f.slot])logos[f.slot]=name;
    if(f.kind==='font'&&!fonts.some(font=>font.file===name)){
      const family=f.family??name,i=fonts.findIndex(font=>font.family.toLowerCase()===family.toLowerCase()&&!font.file);
      if(i>=0)fonts[i]={...fonts[i],file:name};else fonts.push({family,role:fonts.length?'body':'headline',file:name});
    }
  }
  if(fonts.length>8)throw new Error('一个品牌最多使用 8 个字体，请明确选择需要的字体');
  return {...kit,logos,fonts};
}
export async function agentBrandPicture(src:string,slot:string){
  if(!/^data:image\/(png|jpeg|webp|gif|svg\+xml);base64,/.test(src))throw new Error('品牌图片来源无效');
  const image=new Image();image.src=src;
  let timer:ReturnType<typeof setTimeout>|undefined;
  try{await Promise.race([image.decode(),new Promise<never>((_,reject)=>{timer=setTimeout(()=>reject(new Error('品牌图片读取超时')),10000)})]);}
  finally{clearTimeout(timer)}
  if(!image.naturalWidth||!image.naturalHeight)throw new Error('品牌图片尺寸无效');
  const canvas=document.createElement('canvas');canvas.width=640;canvas.height=360;
  const ctx=canvas.getContext('2d');if(!ctx)throw new Error('品牌图片画布不可用');
  ctx.fillStyle=slot==='light'?'#ffffff':slot==='dark'?'#111111':'#808080';ctx.fillRect(0,0,640,360);
  const ratio=Math.min(512/image.naturalWidth,252/image.naturalHeight),w=image.naturalWidth*ratio,h=image.naturalHeight*ratio;
  ctx.drawImage(image,(640-w)/2,(360-h)/2,w,h);
  return {jpeg:canvas.toDataURL('image/jpeg',0.9).split(',')[1]};
}
export const HEX = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

export const slug = (name: string) =>
  name
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48) || "kit";

export const normHex = (hex: string): string | undefined => {
  if (!HEX.test(hex.trim())) return undefined;
  const h = hex.trim().slice(1).toLowerCase();
  return `#${h.length === 3 ? [...h].map((c) => c + c).join("") : h}`;
};

const str = (v:unknown,max=2000) => typeof v === "string" ? v.slice(0,max) : "";
export function normStyle(raw: unknown): BrandStyle | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const s = raw as Record<string, unknown>;
  const palette = (Array.isArray(s.palette) ? s.palette : [])
    .flatMap((p) => {
      const hex = p && typeof p === "object" ? normHex(str((p as Record<string, unknown>).hex, 16)) : typeof p === "string" ? normHex(p) : undefined;
      if (!hex) return [];
      const role = p && typeof p === "object" ? str((p as Record<string, unknown>).role, 60) : "";
      return [{ hex, ...(role ? { role } : {}) }];
    })
    .slice(0, 16);
  const notes = str(s.notes, 4000);
  const source = str(s.source, 400);
  return { ...(source ? { source } : {}), palette, type: str(s.type), shotLengths: str(s.shotLengths), transitions: str(s.transitions), camera: str(s.camera), grain: str(s.grain), ...(notes ? { notes } : {}) };
}

export function mergeKit(base: BrandKit, args: Record<string, unknown>): BrandKit | { problem: string } {
  const out: { -readonly [K in keyof BrandKit]: BrandKit[K] } = { ...base };
  if (typeof args.name === "string" && args.name.trim()) out.name = args.name.trim().slice(0, 80);
  if (Array.isArray(args.colors)) {
    const colors = args.colors.flatMap((c) => {
      const o = (c && typeof c === "object" ? c : {}) as Record<string, unknown>;
      const hex = typeof o.hex === "string" ? normHex(o.hex) : undefined;
      return hex ? [{ hex, role: typeof o.role === "string" && o.role.trim() ? o.role.trim().slice(0, 60) : "accent", ...(typeof o.name === "string" && o.name.trim() ? { name: o.name.trim().slice(0, 60) } : {}) }] : [];
    });
    if (args.colors.length && !colors.length) return { problem: "colors need hex values like #0b1d3a." };
    out.colors = colors;
  }
  if (Array.isArray(args.fonts)) {
    out.fonts = args.fonts.flatMap((f) => {
      if (!f || typeof f !== "object" || typeof (f as Record<string, unknown>).family !== "string") return [];
      const o = f as Record<string, unknown>;
      const family = String(o.family).trim().slice(0, 80);
      if (!family) return [];
      // D198: `file` is a kit file, a path or an address (brand_write brings it in); a family that had a file keeps it.
      const file = typeof o.file === "string" && o.file.trim() ? o.file.trim().slice(0, 2000) : base.fonts.find((x) => x.family.toLowerCase() === family.toLowerCase() && x.file)?.file;
      return [{ family, role: String(o.role ?? "body").slice(0, 40), ...(typeof o.weight === "number" ? { weight: o.weight } : {}), ...(file ? { file } : {}) }];
    });
  }
  if (typeof args.tone === "string") out.tone = args.tone.slice(0, 4000);
  if (typeof args.about === "string") {
    const about = args.about.trim().slice(0, 2000);
    if (about) out.about = about;
    else delete out.about;
  }
  for (const k of ["taglines", "urls", "dos", "donts"] as const) if (Array.isArray(args[k])) out[k] = (args[k] as Array<unknown>).filter((x): x is string => typeof x === "string" && x.trim() !== "").slice(0, 40);
  if (args.style !== undefined) {
    const style = normStyle(args.style);
    if (!style) return { problem: "style is { palette, type, shotLengths, transitions, camera, grain, notes? }." };
    if (!style.palette.length) return { problem: "style.palette needs hex colours like #0b1d3a." };
    out.style = style;
  }
  return out;
}

/**
 * D198: what a brand_write would change in a kit's colours and fonts that it had already (a
 * field the kit had nothing in yet isn't a change), each as its lines before and after. A font
 * file added to a family that's there isn't a change of the fonts.
 */
export function kitChanges(before: BrandKit, after: BrandKit): Array<{ field: string; before: Array<string>; after: Array<string> }> {
  const out: Array<{ field: string; before: Array<string>; after: Array<string> }> = [];
  const colour = (c: BrandKit["colors"][number]) => `${c.hex} ${c.role}${c.name ? ` (${c.name})` : ""}`;
  const font = (f: BrandKit["fonts"][number]) => `${f.family} for ${f.role}${f.weight ? ` ${f.weight}` : ""}`;
  const differ = (a: ReadonlyArray<string>, b: ReadonlyArray<string>) => a.length !== b.length || a.some((x, i) => x.toLowerCase() !== b[i]!.toLowerCase());
  if (before.colors.length && differ(before.colors.map(colour), after.colors.map(colour))) out.push({ field: "Colours", before: before.colors.map(colour), after: after.colors.map(colour) });
  if (before.fonts.length && differ(before.fonts.map(font), after.fonts.map(font))) out.push({ field: "Fonts", before: before.fonts.map(font), after: after.fonts.map(font) });
  return out;
}

