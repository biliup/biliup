// Adapted from Uncut export/exporter/frames.ts, MIT; see licenses/uncut/LICENSE.
// Pure frame planner: filesystem/process operations are yielded to the Rust host, never executed from model code.
import {type Project,subFrameTimes,itemMotionBlur,layout,visualsAt,isSvg,SOLID,isDrawnComponent,textHtml,solidHtml,componentHtml,graphicDocument,graphicInfo,graphicLocal,censusOf,SMALL_TEXT,MAX_GRAPHIC_BYTES,type CensusRun,type SoftRun,irisRadius,maskGeq,clipFx,needsPicturePass,hasOverlay,fxOverlayHtml,isPlainBox,baseBox,fitRect,colourLut,cubeText,moveBox,canvasBox} from '../core'
import {clipShader,shaderParamBytes,shaderModuleCode,shaderFrameBytes,shaderTime} from '../core/visual/shaders'
import {mediaToken,sourceFramePick,svgClipHtml,untaggedFilter,blurFilters,placementOf,fxGraph,boxGraph} from './exportPlan'
import {graphicWithAssets,type RenderAssets} from '../core/visual/renderAssets'
import {canvasBackground} from '../core'
import {planMasks,maskGraph} from './plan/masks'
import {maskRegions} from '../core/timeline/masks'
import {fxSkips} from '../core/visual/fx'
export type FrameResult={frame:number;jpeg:string;notes?:readonly string[];small?:readonly CensusRun[];measured?:number;unseen?:number;soft?:readonly SoftRun[];emptyPreview?:number}
type FrameInput={project:Project;paths:Record<string,string>;graphics:Record<string,string>;assets:RenderAssets;directory:string;frames:readonly number[];width:number;motionBlur?:boolean;census?:boolean;untagged?:Record<string,number>;budgetMs?:number}
type Reply={value?:unknown;error?:string}
let deliver:((state:unknown)=>void)|undefined,accept:((reply:Reply)=>void)|undefined,busy=false
function request(step:Record<string,unknown>):Promise<any>{
  return new Promise((resolve,reject)=>{accept=reply=>{accept=undefined;reply.error?reject(new Error(reply.error)):resolve(reply.value)};const send=deliver;deliver=undefined;if(!send)throw new Error('画面协议状态无效');send({step})})
}
export function frameStart(input:FrameInput){
  if(busy)throw new Error('画面渲染仍在运行');busy=true
  return new Promise(resolve=>{deliver=resolve;void planFrames(input).then(frames=>finish({done:frames}),error=>finish({error:String(error)}))})
}
function finish(result:unknown){busy=false;accept=undefined;const send=deliver;deliver=undefined;send?.(result)}
export function frameNext(reply:Reply){if(!busy||!accept)throw new Error('没有待处理的画面步骤');return new Promise(resolve=>{deliver=resolve;accept!(reply)})}
async function planFrames(input:FrameInput){
  const {project,frames,width}=input
  const deadline=input.budgetMs===undefined?Infinity:Date.now()+Math.max(0,input.budgetMs)
  if(!Array.isArray(frames)||!frames.length||frames.length>24||frames.some(f=>!Number.isSafeInteger(f)||f<0))throw new Error('取帧范围无效')
  const {starts,duration}=layout(project);if(duration<=0)throw new Error('时间轴为空')
  const {width:cw,height:ch}=project.canvas
  const temp=input.directory,css='',signal=undefined,bin='ffmpeg'
  const join=(...parts:string[])=>parts.join('/')
  const mediaPath=(media:Project['media'][number])=>{const path=input.paths[media.id];if(!path)throw new Error('素材不可读：'+media.id);return path}
  const graphics=new Map(Object.entries(input.graphics).map(([id,html])=>[id,graphicWithAssets(html,input.assets)]))
  const luts=new Map<string,string>()
  const target={...project,mediaUrl:(id:string)=>project.media.find(m=>m.id===id)?.kind==='image'?mediaToken(id):undefined}
  const rs={render:(value:Record<string,unknown>)=>request({kind:'raster',...value})}
  const run=(_bin:string,args:readonly string[],_signal:unknown)=>request({kind:'ffmpeg',args}) as Promise<{code:number;stderr:string}>
  const motionBlur=input.motionBlur===true,census=input.census===true
    const W = Math.max(2, Math.round(width / 2) * 2);
    const H = Math.max(2, Math.round((W * project.canvas.height) / project.canvas.width / 2) * 2);
    if(W<2||H<2||W*H>16_777_216)throw new Error('画面尺寸超出限制');
    const background=join(temp,'background.png');
    const sized={};
    await rs.render({width:cw,height:ch,html:`<div style="position:absolute;inset:0;background:${canvasBackground(project)}"></div>`,out:background});
    const untagged=new Map(Object.entries(input.untagged??{}));
    const draw = async (html: string, png: string) => rs.render({ width: cw, height: ch, css, html: html, out: png, ...sized });
    const out: Array<FrameResult> = [];
    for (const requested of frames) {
      if(Date.now()>=deadline)break
      const f = Math.max(0, Math.min(duration - 1, Math.floor(requested)));
      const notes: Array<string> = [];
      const small: Array<CensusRun> = [];
      const soft: Array<SoftRun> = [];
      let measured = 0;
      let unseen = 0;
      let emptyPreview = 0;
      const inputs: Array<string> = ["-i", background];
      const graph: Array<string> = [`[0:v]scale=${W}:${H},format=rgba[b0]`];
      let base = "b0";
      let k = 0;
      for (const v of visualsAt(project, starts, f)) {
        const media = v.item.media ? project.media.find((m) => m.id === v.item.media) : undefined;
        let input: Array<string> | undefined;
        // The source frame on screen at that moment, picked like the export picks it (exportPlan.ts).
        const pick = sourceFramePick(v.source, project.canvas.fps, v.item.type === "clip" && v.item.hold === true && !v.item.loop);
        const local = f - (starts.get(v.item.id) ?? 0);
        const svg = v.item.type === "clip" && isSvg(media) && !!v.box && !maskRegions(v.item).length;
        if ((v.item.type === "clip"||v.item.type==='danmaku') && media && media.kind === "video") input = [...pick.seekArgs, "-i", mediaPath(media)];
        else if (svg) {
          // An SVG (D76): drawn by Chromium in its box, as the export does.
          const png = join(temp, `w${W}-f${f}-${k}.png`);
          await draw(svgClipHtml(v.item, v.box!, mediaToken(media!.id), v.opacity), png);
          input = ["-f", "image2", "-pattern_type", "none", "-i", png];
        } else if (v.item.type === "clip" && media && media.kind === "image") input = ["-f", "image2", "-pattern_type", "none", "-i", mediaPath(media)];
        else if ((v.item.type === "text" && typeof v.item.text === "string") || (v.item.type === "component" && v.item.component === SOLID) || isDrawnComponent(v.item)) {
          const png = join(temp, `w${W}-f${f}-${k}.png`);
          const html =
            v.item.type === "text"
              ? textHtml(v.item, project, { opacity: v.opacity, motion: v.motion })
              : v.item.component === SOLID
                ? solidHtml(v.item, project, { opacity: v.opacity })
                : componentHtml(v.item, target, local, { opacity: v.opacity, motion: v.motion, fps: project.canvas.fps, ...(v.box ? { box: v.box } : {}) });
          await draw(html, png);
          input = ["-f", "image2", "-pattern_type", "none", "-i", png];
        } else if (v.item.type === "graphic" && typeof v.item.src === "string") {
          // A graphic (D78) at this frame; its opacity goes on below, like a clip's.
          const html = graphics.get(v.item.src);
          if (html === undefined) {
            notes.push(`the graphic "${v.item.name ?? v.item.id}" isn't drawn: ${v.item.src} couldn't be read (missing, outside graphics/, over ${MAX_GRAPHIC_BYTES / 1024} KB or a name it doesn't take)`);
            continue;
          }
          const png = join(temp, `w${W}-f${f}-${k}-g.png`);
          const at = graphicLocal(v.item, local);
          // With blur: the export's sub-frames (planExport's: a held frame, before its start or past its end, stays sharp).
          const blur = motionBlur ? itemMotionBlur(v.item, project, true) : undefined;
          const ts = blur ? subFrameTimes(at, project.canvas.fps, blur, at !== (v.item.in ?? 0) + local) : undefined;
          // The census only for a graphic seen whole and mostly opaque (a zoomed or fading one's words aren't at their drawn size).
          const counted = census && v.opacity >= 0.35 && v.zoom === undefined && !v.box;
          const reply = await rs.render({ kind: "graphic", width: cw, height: ch, doc: graphicDocument(html, project.canvas,30000), t: at / project.canvas.fps, ...(ts && ts.length > 1 ? { ts } : {}), info: graphicInfo(v.item, project.canvas, at), out: png, name: `"${v.item.name ?? v.item.id}" (${v.item.src})`, ...(counted ? { census: SMALL_TEXT } : {}), ...sized });
          const got = counted && reply ? censusOf(reply.census) : undefined;
          if (census && !got && v.opacity >= 0.35) unseen++;
          if (got) {
            measured += got.measured;
            unseen += got.unseen ?? 0;
            emptyPreview = Math.max(emptyPreview, got.emptyPreview ?? 0);
            small.push(...got.small);
            soft.push(...(got.soft ?? []));
          }
          input = ["-f", "image2", "-pattern_type", "none", "-i", png];
        }
        if (!input) continue;
        // A wipe transition shows the left X pixels (even, as the export's crop does).
        const shown = v.wipe === undefined ? W : 2 * Math.floor((W * v.wipe) / 2);
        if (shown <= 0) continue;
        const fade = (v.item.type === "clip" && !svg) || v.item.type === "graphic" || v.item.type==='danmaku' ? `,colorchannelmixer=aa=${v.opacity.toFixed(4)}` : "";
        const tag = media ? untaggedFilter(untagged.get(mediaPath(media))) : "";
        let picked = (v.item.type === "clip"||v.item.type==='danmaku') && media?.kind === "video" ? `${tag}${pick.filter},` : "trim=end_frame=1,setpts=0,";
        const masks=media&&!svg?planMasks(v.item,media,project.media,mediaPath,starts.get(v.item.id)??0):undefined;
        if(masks){
          const masked=join(temp,`w${W}-f${f}-${k}-masked.png`);
          const pass=maskGraph(masks,'maskSource','masked',project.canvas.fps,f,1);
          const graph=[`[0:v:0]${picked}setpts=0[maskSource]`,...pass.graph];
          const result=await run(bin,['-hide_banner','-nostdin','-y','-v','error',...input,...pass.inputs,'-filter_complex',graph.join(';'),'-map','[masked]','-frames:v','1',masked],signal);
          if(result.code!==0)throw new Error(result.stderr);
          input=['-f','image2','-pattern_type','none','-i',masked];
          picked='trim=end_frame=1,setpts=0,';
        }
        // An iris (D79): alpha times the disc at the canvas's centre, for a layer whose top-left is at (ox, oy).
        // A reveal shape (D133): alpha times core's coverage, the same way.
        const iris = (ox: number, oy: number) =>
          v.iris !== undefined
            ? `,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='alpha(X,Y)*clip(${irisRadius(v.iris, { width: W, height: H }).toFixed(4)}-hypot(X+${ox + 0.5}-${W / 2},Y+${oy + 0.5}-${H / 2})+0.5,0,1)'`
            : v.mask
              ? `,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='alpha(X,Y)*clip(${maskGeq(v.mask.shape, v.mask.p.toFixed(6), W, H, `(X+${ox + 0.5})`, `(Y+${oy + 0.5})`)}+0.5,0,1)'`
              : "";
        // A blur dissolve (D133): gblur at the frame's sigma (output px).
        const blur = v.blur ? `,${blurFilters((v.blur * H) / ch)}` : "";
        // A clip's looks and effects (D136): picture effects before its box (which it then always has), overlays after it.
        const effects = v.item.type === "clip" && media && !fxSkips(media) ? clipFx(v.item) : undefined;
        // D213: a shader effect is a picture pass, drawn first with the rasteriser's WebGPU as the export does.
        const shader = v.item.type === "clip" && media && !fxSkips(media) ? clipShader(v.item) : undefined;
        const picture = needsPicturePass(effects) || effects?.blur !== undefined || !!shader;
        const overlay = async () => {
          if (!effects || !hasOverlay(effects) || !v.box) return;
          const png = join(temp, `w${W}-f${f}-${k}-fx.png`);
          await draw(fxOverlayHtml(effects, v.box, local), png);
          k++;
          inputs.push("-f", "image2", "-pattern_type", "none", "-i", png);
          graph.push(`[${k}:v:0]scale=${W}:${H},format=rgba,colorchannelmixer=aa=${v.opacity.toFixed(4)}${blur}${iris(0, 0)}${shown < W ? `,crop=w=${shown}:h=${H}:x=0:y=0,pad=${W}:${H}:0:0:color=black@0` : ""}[l${k}]`);
          graph.push(`[${base}][l${k}]overlay=format=auto[b${k}]`);
          base = `b${k}`;
        };
        if (v.box && media && !svg && (picture || !isPlainBox(v.box, project.canvas))) {
          // A box (D75; moved by a push or zoom, D79), made as the export makes it; a wipe or iris reveals the canvas.
          const p = placementOf(v.box, W / cw, H / ch, baseBox(v.item, project.canvas, media.probe));
          const visible = Math.min(p.w, shown - p.x);
          if (visible <= 0) continue;
          let source = input;
          let pick = picked;
          if (shader) {
            // The source frame at the effects' size, drawn by the shader, then the rest as for any clip.
            const fit = fitRect(baseBox(v.item, project.canvas, media.probe), v.box.fit, media.probe);
            const sw = v.box.scale !== 1 ? Math.max(1, Math.round((fit.w * W) / cw)) : p.cw;
            const sh = v.box.scale !== 1 ? Math.max(1, Math.round((fit.h * H) / ch)) : p.ch;
            const raw = join(temp, `w${W}-f${f}-${k}-src.rgba`);
            const drawn = join(temp, `w${W}-f${f}-${k}-shader.rgba`);
            const d = await run(bin, ["-hide_banner", "-nostdin", "-y", "-v", "error", ...input, "-filter_complex", `[0:v:0]${picked}scale=w=${sw}:h=${sh}:out_range=pc,format=rgba[o]`, "-map", "[o]", "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgba", raw], signal);
            if (d.code !== 0) throw new Error(d.stderr);
            const params = shaderParamBytes(shader.def.params, shader.values);
            try {
              await rs.render({ kind: "shader", width: sw, height: sh, code: shaderModuleCode(shader.def), frame: [...new Float32Array(shaderFrameBytes(sw, sh, shaderTime(f, starts.get(v.item.id) ?? 0, project.canvas.fps), false))], ...(params ? { params: [...new Float32Array(params)] } : {}), input: raw, out: drawn, name: `the shader effect ${shader.def.name}` });
              source = ["-f", "rawvideo", "-pix_fmt", "rgba", "-s", `${sw}x${sh}`, "-i", drawn];
              pick = "trim=end_frame=1,setpts=0,";
            } catch (error) {
              // Never silently different (D213): the frame shows the clip plain and says why.
            }
          }
          k++;
          inputs.push(...source);
          if (picture && (effects || shader)) {
            const lut = effects?.colour ? colourLut(effects.colour) : undefined;
            if (lut && !luts.has(lut.key)) {
              const file = join(temp, `lut-${lut.key}.cube`);
              await request({kind:'write',path:file,text:cubeText(lut)});
              luts.set(lut.key, file);
            }
            // The effects' size as the export's: the picture's fit before any zoom (its placement's when unzoomed).
            const fit = fitRect(baseBox(v.item, project.canvas, media.probe), v.box.fit, media.probe);
            const zoomed = v.box.scale !== 1;
            const lf = { fx: effects ?? {}, start: starts.get(v.item.id) ?? 0, w: zoomed ? Math.max(1, Math.round((fit.w * W) / cw)) : p.cw, h: zoomed ? Math.max(1, Math.round((fit.h * H) / ch)) : p.ch, k: H / ch, ...(lut ? { lut } : {}) };
            graph.push(`[${k}:v:0]${pick}null[e${k}]`, ...fxGraph(lf, `e${k}`, `s${k}`, project.canvas.fps, f, luts), ...boxGraph([p], `s${k}`, `r${k}`, project.canvas.fps));
          } else graph.push(`[${k}:v:0]${pick}null[s${k}]`, ...boxGraph([p], `s${k}`, `r${k}`, project.canvas.fps));
          graph.push(`[r${k}]format=rgba${fade}${blur}${visible < p.w ? `,crop=w=${visible}:h=${p.h}:x=0:y=0:exact=1` : ""}${iris(p.x, p.y)}[l${k}]`);
          graph.push(`[${base}][l${k}]overlay=x=${p.x}:y=${p.y}:format=auto[b${k}]`);
          base = `b${k}`;
          await overlay();
          continue;
        }
        k++;
        inputs.push(...input);
        const full = `[${k}:v:0]${picked}scale=${W}:${H}:force_original_aspect_ratio=decrease,pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:color=black@0,format=rgba${fade}`;
        if (!v.box && (v.shift !== undefined || v.shiftY !== undefined || v.zoom !== undefined)) {
          // Text, a background or a graphic pushed or zoomed (D79): the canvas as its box, moved.
          const p = placementOf(moveBox(canvasBox(project.canvas), v.shift, v.zoom, v.shiftY), W / cw, H / ch, canvasBox(project.canvas));
          graph.push(`${full}[s${k}]`, ...boxGraph([p], `s${k}`, `r${k}`, project.canvas.fps));
          graph.push(`[r${k}]format=rgba${blur}[l${k}]`);
          graph.push(`[${base}][l${k}]overlay=x=${p.x}:y=${p.y}:format=auto[b${k}]`);
          base = `b${k}`;
          continue;
        }
        graph.push(`${full}${blur}${iris(0, 0)}${shown < W ? `,crop=w=${shown}:h=${H}:x=0:y=0,pad=${W}:${H}:0:0:color=black@0` : ""}[l${k}]`);
        graph.push(`[${base}][l${k}]overlay=format=auto[b${k}]`);
        base = `b${k}`;
        await overlay();
      }
      graph.push(`[${base}]format=yuvj420p[out]`);
      const file = join(temp, `w${W}-frame-${f}.jpg`);
      const r = await run(bin, ["-hide_banner", "-nostdin", "-y", "-v", "error", ...inputs, "-filter_complex", graph.join(";"), "-map", "[out]", "-frames:v", "1", "-q:v", "4", "-update", "1", file], signal);
      if (r.code !== 0) throw new Error(r.stderr);
      small.sort((a, b) => b.share - a.share);
      out.push({ frame: f, jpeg: file, ...(notes.length ? { notes } : {}), ...(census ? { small: small.slice(0, 40), measured, ...(unseen ? { unseen } : {}), ...(emptyPreview ? { emptyPreview } : {}), ...(soft.length ? { soft: soft.sort((a, b) => b.share - a.share).slice(0, 8) } : {}) } : {}) });
    }
    return out;
}
