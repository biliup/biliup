import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import {matchesEditorAsset as matches} from '@/app/lib/editor/asset-search';
import {RECORDING_MASK} from '@/app/lib/editor/recording-effects';
import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Effects tab (D136, feedback G2): Filters (colour looks) and Effects, each tile a thumbnail the
// preview's own renderer draws (preview/fx.ts, the same maths as the preview and the export) from
// the selected clip's picture at the playhead, or from a built-in sample picture when no clip is
// selected. Click a tile to put it on the selected clip, or drag it onto a clip in the timeline or
// on the canvas. Fine-tuning (intensity, Adjust, effect settings) is in the Inspector.
import { clipFx, EFFECTS, effectParams, fxOverlayInner, hasOverlay, LOOK, LOOKS, needsPicturePass, type ClipFx, type EffectInfo } from "@/app/lib/editor/core/visual/fx";
import type { ModuleUse } from "@/app/lib/editor/core/timeline/types";
import { Ban, X, Grid2X2, MessageSquareText } from "lucide-react";
import React, { useEffect, useMemo, useRef, useState } from "react";

import { FxRenderer } from "@/app/ui/editor/preview/fx";
import { previewRunner, whenRunner } from "@/app/ui/editor/preview/shader";
import { shaderFrameBytes, shaderModuleCode, shaderParamBytes, shaderValues, SHADER, type ShaderDef } from "@/app/lib/editor/core/visual/shaders";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { actions, shaderDefs, useApp, type Project } from "@/app/lib/editor/state/store";
import { findItem } from "@/app/ui/editor/timeline/edit";
import {beginEditorDrag,endEditorDrag} from '@/app/ui/editor/timeline/dnd';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/** Thumbnails are drawn this size (px), the canvas's 1080 lines scaled to its height. */
const THUMB = { width: 192, height: 108 } as const;

/** The built-in sample picture: sky, sun, hills, water, a face's tones and colour swatches. */
let sample: HTMLCanvasElement | undefined;
function samplePicture(): HTMLCanvasElement {
  if (sample) return sample;
  const c = document.createElement("canvas");
  c.width = 384;
  c.height = 216;
  const g = c.getContext("2d")!;
  const sky = g.createLinearGradient(0, 0, 0, 130);
  sky.addColorStop(0, "#3d6fb6");
  sky.addColorStop(0.65, "#f2a65a");
  sky.addColorStop(1, "#f7d38a");
  g.fillStyle = sky;
  g.fillRect(0, 0, 384, 140);
  g.fillStyle = "#fff4c4";
  g.beginPath();
  g.arc(250, 108, 26, 0, Math.PI * 2);
  g.fill();
  g.fillStyle = "#3b5d3a";
  g.beginPath();
  g.moveTo(0, 140);
  g.lineTo(70, 92);
  g.lineTo(150, 128);
  g.lineTo(230, 84);
  g.lineTo(320, 126);
  g.lineTo(384, 100);
  g.lineTo(384, 150);
  g.lineTo(0, 150);
  g.fill();
  const sea = g.createLinearGradient(0, 140, 0, 216);
  sea.addColorStop(0, "#2f6f8f");
  sea.addColorStop(1, "#0f2c3d");
  g.fillStyle = sea;
  g.fillRect(0, 140, 384, 76);
  // A face's tones and a few strong colours, so looks that treat them differently show it.
  g.fillStyle = "#e0ac8a";
  g.beginPath();
  g.arc(70, 176, 22, 0, Math.PI * 2);
  g.fill();
  ["#e63946", "#f4d35e", "#2a9d8f", "#5b4bdb", "#ffffff", "#111111"].forEach((col, i) => {
    g.fillStyle = col;
    g.fillRect(150 + i * 36, 172, 30, 30);
  });
  sample = c;
  return c;
}

/** One shared renderer draws every tile (WebGL contexts are few), each copied into its tile. */
let shared: FxRenderer | undefined | null;
const renderer = () => {
  if (shared === undefined) {
    try {
      shared = new FxRenderer();
    } catch {
      shared = null;
    }
  }
  return shared;
};

/** The selected clip's picture element in the preview, when it has a frame to read. */
function clipPicture(id: string | undefined): HTMLVideoElement | HTMLImageElement | undefined {
  if (!id) return undefined;
  const el = editorRoot().querySelector<HTMLVideoElement | HTMLImageElement>(`.pv-box[data-item="${CSS.escape(id)}"] > video, .pv-box[data-item="${CSS.escape(id)}"] > img`);
  if (el instanceof HTMLVideoElement) return el.readyState >= 2 && el.videoWidth > 0 ? el : undefined;
  if (el instanceof HTMLImageElement) return el.complete && el.naturalWidth > 0 ? el : undefined;
  return undefined;
}

type Source = { el: TexImageSource; w: number; h: number; key: string };

/** A tile's thumbnail: its effect drawn on the source, overlays (vignette, grain, bars) on top. */
function FxThumb({ fx, source, testId }: { fx: ClipFx | undefined; source: Source; testId: string }) {
  useEditorLanguage();
  const ref = useRef<HTMLCanvasElement>(null);
  const k = THUMB.height / 1080;
  useEffect(() => {
    const canvas = ref.current;
    const r = renderer();
    if (!canvas) return;
    const g = canvas.getContext("2d")!;
    canvas.width = THUMB.width;
    canvas.height = THUMB.height;
    if (r && fx && needsPicturePass(fx) && r.render(source.el, fx, THUMB.width, THUMB.height, 0.2, k)) g.drawImage(r.canvas, 0, 0);
    else g.drawImage(source.el as CanvasImageSource, 0, 0, THUMB.width, THUMB.height);
    canvas.dataset.drawn = source.key;
  }, [fx, source, k]);
  // Overlays at a 1080-line size, scaled into the tile like the canvas is.
  const overlay = fx && hasOverlay(fx) ? fxOverlayInner(fx, { x: 0, y: 0, w: 1920, h: 1080 }, 0) : "";
  const box = useRef<HTMLDivElement>(null);
  const layer = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = box.current;
    if (!el || !overlay) return;
    const fit = () => layer.current && (layer.current.style.transform = `scale(${el.clientWidth / 1920})`);
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(el);
    return () => observer.disconnect();
  }, [overlay]);
  return (
    <div className="thumb fx-thumb" data-testid={testId} ref={box}>
      <canvas ref={ref} style={fx?.blur ? { filter: `blur(${(fx.blur * k).toFixed(2)}px)` } : undefined} />
      {overlay && <div className="fx-thumb-overlay" ref={layer} dangerouslySetInnerHTML={{ __html: overlay }} />}
    </div>
  );
}

// Shader thumbnails (D213) are drawn one at a time (they share the runner's read-back buffer).
let shaderQueue: Promise<unknown> = Promise.resolve();

/** A shader effect's tile: drawn with WebGPU on the source at its defaults, 0.4 s in; "No WebGPU" without it. */
function ShaderThumb({ def, source }: { def: ShaderDef; source: Source }) {
  useEditorLanguage();
  const ref = useRef<HTMLCanvasElement>(null);
  const [problem, setProblem] = useState<string | undefined>(undefined);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const off = whenRunner(() => setTick((n) => n + 1));
    return () => void off();
  }, []);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    canvas.width = THUMB.width;
    canvas.height = THUMB.height;
    const g = canvas.getContext("2d")!;
    g.drawImage(source.el as CanvasImageSource, 0, 0, THUMB.width, THUMB.height);
    const runner = previewRunner();
    if (!runner) return;
    if ("problem" in runner) return setProblem(runner.problem);
    setProblem(undefined);
    let live = true;
    const { width: w, height: h } = THUMB;
    shaderQueue = shaderQueue
      .then(() =>
        runner.drawToPixels(
          { kind: "external", source: source.el as GPUCopyExternalImageSource, width: source.w, height: source.h },
          { code: shaderModuleCode(def), frame: shaderFrameBytes(w, h, 0.4, false), params: shaderParamBytes(def.params, shaderValues(def.params, {})), width: w, height: h },
        ),
      )
      .then((px) => {
        if (!live) return;
        g.putImageData(new ImageData(new Uint8ClampedArray(px), w, h), 0, 0);
        canvas.dataset.drawn = source.key;
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [def, source, tick]);
  return (
    <div className="thumb fx-thumb" data-testid="shader-thumb">
      <canvas ref={ref} />
      {problem && <div className="fx-thumb-note" data-testid="shader-thumb-no-gpu">{editorText("No WebGPU")}</div>}
    </div>
  );
}

/** A look as a clip's effects would carry it, compiled. */
const lookFx = (look: string) => clipFx({ type: "clip", effects: [{ module: LOOK, params: { look, intensity: 1 } }] });
/** An effect at its defaults, compiled (zoom pulse: a still at its widest). */
const effectFx = (e: EffectInfo): ClipFx | undefined => clipFx({ type: "clip", effects: [{ module: e.id, params: effectParams(e.id, {}) }] });

export function EffectsLibrary({ project }: { project: Project }) {
  useEditorLanguage();
  const { connection, projectShaders,biliupCanEdit } = useApp();
  const online = connection === "connected"&&!!biliupCanEdit;
  const selectedId = usePlayback((s) => s.selected);
  const playing = usePlayback((s) => s.playing);
  const frame = usePlayback((s) => Math.round(s.frame));
  const placed = findItem(project, selectedId);
  const media = placed?.item.media ? project.media.find((m) => m.id === placed.item.media) : undefined;
  const clip = placed && placed.item.type === "clip" && media && media.kind !== "audio" ? placed.item : undefined;
  const [query, setQuery] = useState("");
  // The thumbnails' source: the selected clip at the playhead (redrawn when it's paused somewhere new), else the sample.
  const [source, setSource] = useState<Source>(() => ({ el: samplePicture(), w: 384, h: 216, key: "sample" }));
  useEffect(() => {
    if (playing) return;
    let tries = 0;
    let timer = 0;
    const pick = () => {
      const el = clipPicture(clip?.id);
      if (el) {
        const w = el instanceof HTMLVideoElement ? el.videoWidth : el.naturalWidth;
        const h = el instanceof HTMLVideoElement ? el.videoHeight : el.naturalHeight;
        // A copy: the thumbnails keep this frame while the video moves on.
        const copy = document.createElement("canvas");
        const scale = Math.min(1, 480 / Math.max(1, w));
        copy.width = Math.max(1, Math.round(w * scale));
        copy.height = Math.max(1, Math.round(h * scale));
        copy.getContext("2d")!.drawImage(el, 0, 0, copy.width, copy.height);
        setSource({ el: copy, w: copy.width, h: copy.height, key: `${clip!.id}@${frame}` });
      } else if (clip && tries++ < 10) timer = window.setTimeout(pick, 200);
      else setSource((s) => (s.key === "sample" ? s : { el: samplePicture(), w: 384, h: 216, key: "sample" }));
    };
    timer = window.setTimeout(pick, 120);
    return () => clearTimeout(timer);
  }, [clip?.id, frame, playing]);

  const effects: ReadonlyArray<ModuleUse> = clip?.effects ?? [];
  const activeLook = (effects.find((e) => e.module === LOOK)?.params as { look?: string } | undefined)?.look;
  const has = new Set(effects.map((e) => e.module));
  const looks = useMemo(() => LOOKS.map((l) => ({ id: l.id, name: l.name, description: l.description, fx: lookFx(l.id) })), []);
  const fxs = useMemo(() => EFFECTS.map((e) => ({ info: e, fx: effectFx(e) })), []);
  const shownLooks = looks.filter((l) => matches(query, l.name, l.description));
  const shaders = useMemo(() => shaderDefs(project), [project, projectShaders]);
  const activeShader = (effects.find((e) => e.module === SHADER)?.params as { shader?: string } | undefined)?.shader;
  const shownShaders = shaders.filter((d) => matches(query, d.name, d.description ?? "", d.from, "Shader"));
  const shownFx = fxs.filter((e) => matches(query, e.info.name, e.info.description));
  const shownRecordingEffects = [[RECORDING_MASK,'Mask']].filter(([,label])=>matches(query,label,'马赛克 隐私 模糊 mosaic pixelate blur privacy'));

  const apply = (what: string, e: React.MouseEvent | React.KeyboardEvent) => {
    e.stopPropagation();
    (e.currentTarget as HTMLElement).blur();
    if (online) void actions.applyFx(what);
  };
  const tile = (what: string, name: string, title: string, active: boolean, thumb: React.ReactNode, testId: string, extra: Record<string, string>) => (
    <div
      key={what}
      className={`tile fx-tile${active ? " active" : ""}`}
      data-testid={testId}
      {...extra}
      data-active={active ? "true" : undefined}
      role="button"
      tabIndex={0}
      aria-disabled={!online}
      aria-pressed={active}
      draggable={online}
      title={title}
      onDragStart={(e) => {
        if(!beginEditorDrag({kind:'effect',id:what},e.dataTransfer))e.preventDefault();
      }}
      onDragEnd={endEditorDrag}
      onClick={(e) => apply(what, e)}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
        e.preventDefault();
        apply(what, e);
      }}
    >
      {thumb}
      <div className="tile-name">{name}</div>
    </div>
  );

  return (
    <aside className="library" data-testid="effects-library" data-source={source.key}>
      <div className="lib-head">{editorText("Effects")}</div>
      <div className="lib-section">
        <div className="music-filters">
          <div className="tr-search">
            <input className="field input" data-testid="effects-search" placeholder={editorText("Search filters and effects")} value={query} spellCheck={false} onChange={(e) => setQuery(e.target.value)} />
            {query && (
              <Tooltip label={editorText("Clear")}>
                <button className="icon-btn" data-testid="effects-search-clear" onClick={() => setQuery("")}>
                  <X />
                </button>
              </Tooltip>
            )}
          </div>
          <div className="muted small" data-testid="effects-target">
            {clip ? editorText("On {0}", { 0: clip.name ?? media?.name ?? clip.id }) : editorText("No clip selected")}
          </div>
        </div>
        <div className="grid2 fx-grid">{shownRecordingEffects.map(([module,label])=>{
          const existing=clip?.effects?.find(effect=>effect.module===module);
          const active=existing&&!existing.bypass;
          return tile(`effect:${module}`,editorText(label),editorText('Drag onto a video clip, or select a clip and click.'),!!active,<div className="thumb fx-thumb">{module===RECORDING_MASK?<Grid2X2 size={28}/>:<MessageSquareText size={28}/>}</div>,'recording-effect',{'data-module':module});
        })}</div>
        {shownLooks.length > 0 && (
          <>
            <div className="lib-label">{editorText("Filters")}</div>
            <div className="grid2 fx-grid" data-testid="filters-grid">
              {!query &&
                tile(
                  "look:",
                  editorText("None"),
                  editorText("No filter"),
                  !!clip && !activeLook,
                  <div className="thumb fx-thumb fx-none" data-testid="filter-thumb">
                    <Ban />
                  </div>,
                  "filter-tile",
                  { "data-look": "none" },
                )}
              {shownLooks.map((l) => tile(`look:${l.id}`, editorText(l.name), `${editorText(l.name)}: ${editorText(l.description)}`, activeLook === l.id, <FxThumb fx={l.fx} source={source} testId="filter-thumb" />, "filter-tile", { "data-look": l.id }))}
            </div>
          </>
        )}
        {shownFx.length > 0 && (
          <>
            <div className="lib-label">{editorText("Effects")}</div>
            <div className="grid2 fx-grid" data-testid="effects-grid">
              {shownFx.map(({ info, fx }) => tile(`effect:${info.id}`, editorText(info.name), `${editorText(info.name)}: ${editorText(info.description)}`, has.has(info.id), <FxThumb fx={fx} source={source} testId="effect-thumb" />, "effect-tile", { "data-module": info.id }))}
            </div>
          </>
        )}
        {shownShaders.length > 0 && (
          <>
            <div className="lib-label">{editorText("Shaders")}</div>
            <div className="grid2 fx-grid" data-testid="shaders-grid">
              {shownShaders.map((d) => tile(`shader:${d.id}`, d.name, d.description ? `${d.name}: ${d.description}` : d.name, activeShader === d.id, <ShaderThumb def={d} source={source} />, "shader-tile", { "data-shader": d.id }))}
            </div>
          </>
        )}
        {shownLooks.length === 0 && shownFx.length === 0 && shownShaders.length === 0 && shownRecordingEffects.length === 0 && (
          <div className="muted small" data-testid="effects-empty">
            {editorText("Nothing matches")}</div>
        )}
      </div>
    </aside>
  );
}
