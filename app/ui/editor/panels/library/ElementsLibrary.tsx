import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import {matchesEditorAsset} from '@/app/lib/editor/asset-search';
import {searchStickers} from '@/app/lib/editor/sticker-library';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { SOLID } from "@/app/lib/editor/core/visual/render";
import { type ComponentInfo, componentDefaults, naturalSize, componentHtml, screenRect, COMPONENTS, ELEMENT_CATEGORIES, elementCategory, type ElementCategory } from "@/app/lib/editor/core/visual/components";
import { Eye, EyeOff, Plus, Blocks, ChevronDown, ChevronRight, Search, X } from "lucide-react";
import React, { useRef, useState, useLayoutEffect } from "react";
import { type Project, useApp, actions } from "@/app/lib/editor/state/store";
import {beginEditorDrag,endEditorDrag} from '@/app/ui/editor/timeline/dnd';
import { setDragChip } from "@/app/ui/editor/panels/dragChip";
import { showElement, hideElement, useHiddenElements } from "@/app/ui/editor/panels/elementsGrid";
import { openMenu } from "@/app/ui/editor/timeline/ContextMenu";
import { canGoBehind } from "@/app/ui/editor/preview/text";
import { getPlayback } from "@/app/ui/editor/preview/playback";
import { StickerPicker } from "@/app/ui/editor/panels/Stickers";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/** The frame a component's thumbnail shows (a terminal mid-way, its line typed). */
const THUMB_FRAME: Readonly<Record<string, number>> = { "component.terminal": 45, "component.agent-log": 150, "component.compare-table": 75, "component.lower-third": 45, "component.sticker": 30, "component.progress-bar": 200, "component.subscribe": 20, "component.callout": 30, "component.social-button": 60, "component.checklist": 150, "component.rating": 60 };
/** Props a thumbnail shows instead of the defaults (no pointer in a tile). */
const THUMB_PROPS: Readonly<Record<string, Record<string, unknown>>> = { "component.subscribe": { cursor: false }, "component.social-button": { cursor: false } };
/** A frame where everything has settled, for elements whose look is a moment in an animation. */
const SETTLED = 60;

/**
 * A component's thumbnail (D76): its real HTML (core componentHtml with its default props) laid out
 * at its default size on a 1080p canvas and scaled into the tile, like the preview's canvas. A
 * device frame gets a gradient on its screen, standing in for footage.
 */
function ComponentThumb({ info, project }: { info: ComponentInfo; project: Project }) {
  useEditorLanguage();
  const ref = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const update = () => setBox({ w: el.clientWidth, h: el.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  const canvas = { width: 1920, height: 1080 };
  const props = { ...componentDefaults(info.id), ...THUMB_PROPS[info.id] };
  const size = naturalSize({ component: info.id, props }, canvas);
  const item = { id: `thumb-${info.id}`, type: "component" as const, component: info.id, duration: 300, props, transform: { x: 0, y: 0, w: size.width, h: size.height } };
  const html = componentHtml(item, { canvas, theme: project.theme }, THUMB_FRAME[info.id] ?? (info.block.animated ? SETTLED : 0), { fps: 30 });
  const k = box.w && box.h ? Math.min((box.w * 0.86) / size.width, (box.h * 0.86) / size.height) : 0;
  const screen = info.id === "component.device-frame" ? screenRect("phone", size.width, size.height) : undefined;
  return (
    <div className="thumb comp-thumb" ref={ref} data-testid="component-thumb">
      <div className="comp-thumb-inner" style={{ width: size.width, height: size.height, transform: `translate(-50%, -50%) scale(${k})` }}>
        {screen && <div className="comp-thumb-screen" style={{ left: screen.x, top: screen.y, width: screen.w, height: screen.h, borderRadius: screen.radius }} />}
        <div dangerouslySetInnerHTML={{ __html: html }} />
      </div>
    </div>
  );
}

/** The Background tile's element (E3, D131): one background, its colours and gradient set in the Inspector. */
const BACKGROUND_PROPS = { color: "#0B1D3A" } as const;
const BACKGROUND_THUMB = "linear-gradient(160deg, #0B1D3A, #3B1E54)";

/** A grid tile's hide / show button (E2, D130). */
const HideButton = ({ id, name, hidden }: { id: string; name: string; hidden: boolean }) => (
  <button
    className="tile-hide"
    data-testid={hidden ? "element-show" : "element-hide"}
    title={hidden ? editorText("Show {0}", { 0: editorText(name) }) : editorText("Hide {0}", { 0: editorText(name) })}
    aria-label={hidden ? editorText("Show {0}", { 0: editorText(name) }) : editorText("Hide {0}", { 0: editorText(name) })}
    onClick={(e) => {
      e.stopPropagation();
      e.currentTarget.blur();
      if (hidden) showElement(id);
      else hideElement(id);
    }}
  >
    {hidden ? <Eye /> : <EyeOff />}
  </button>
);

/** Words a tile is found by: its name, its section and its search words. */
const ELEMENT_WORDS:Readonly<Record<string,string>>={
  'component.lower-third':'姓名 人名 字幕 人物介绍 下三分之一',
  'component.callout':'标注 标签 箭头 指向 注释',
  'component.quote':'引用 引言 评价 证言',
  'component.before-after':'前后 对比 分屏 比较',
  'component.end-card':'片尾 结尾 标志 网址',
  'component.subscribe':'订阅 关注 铃铛 按钮',
  'component.social-button':'点赞 关注 分享 评论 收藏 爱心 按钮',
  'component.follow':'关注 账号 用户名 社交 平台 标志',
  'component.follow-card':'关注 名片 账号 头像 频道 个人资料',
  'component.progress-bar':'进度 加载 时间 百分比',
  'component.countdown':'倒计时 时钟 秒 时间',
  'component.checklist':'清单 列表 待办 任务 勾选',
  'component.steps':'步骤 教程 流程 指引 编号',
  'component.rating':'评分 星级 评价 分数',
  'component.price-tag':'价格 标签 促销 折扣 优惠 购物',
  'component.map-pin':'地图 定位 地点 旅行 城市',
  'component.chart':'图表 柱状图 数据 数字 统计',
  'component.shape':'形状 矩形 圆形 线条 箭头 星形 三角形',
  'component.image-frame':'图片 照片 边框 相框 标志',
  'component.sticker':'贴纸 表情 符号',
  'component.device-frame':'设备 手机 笔记本 电脑 样机 屏幕 外框',
  'component.code-block':'代码 编程 代码片段',
  'component.terminal':'终端 命令行 控制台',
  'component.agent-log':'助手 人工智能 日志 模型',
  'component.compare-table':'对比 表格 基准 性能 比较',
};
const tileWords = (name: string, id: string) => [name,elementCategory(id).category,...elementCategory(id).keywords,ELEMENT_WORDS[id]??''];

/**
 * The Elements tab (D76, E2/E3 in D130/D131, sections and search in D251): one search over every
 * element and sticker, a section select (All sections, Titles & text, Social, Info, Shapes & media,
 * Stickers, …; a select, not chips, D263), and the elements under their section headers; a Background (a colour or gradient under the
 * text) sits with the shapes, and the stickers in their own
 * section with theirs inside. Drag one onto the canvas (it lands there, at the playhead) or the
 * timeline (at that time), or + adds it at the playhead. Any tile can be hidden (its eye, or
 * right-click); hidden ones (the dev-promo components start that way) are listed at the end and can
 * be shown again. Remembered in this browser.
 */
export function ElementsLibrary({ project }: { project: Project }) {
  useEditorLanguage();
  const { connection,biliupCanEdit } = useApp();
  const online = connection === "connected"&&!!biliupCanEdit;
  const hidden = useHiddenElements();
  const [showHidden, setShowHidden] = useState(false);
  const [query, setQuery] = useState("");
  const [only, setOnly] = useState<ElementCategory | undefined>(undefined);
  const menu = (e: React.MouseEvent, id: string, name: string, isHidden: boolean) => {
    e.preventDefault();
    openMenu(e.clientX, e.clientY, [isHidden ? { id: "element-show", get label() { return editorText("Show in the grid"); }, run: () => showElement(id) } : { id: "element-hide", get label() { return editorText("Hide from the grid"); }, run: () => hideElement(id) }], name);
  };
  const componentTile = (c: ComponentInfo, isHidden: boolean) => (
    <div
      key={c.id}
      className={`tile comp-tile ${isHidden ? "hidden-tile" : ""}`}
      data-testid={isHidden ? "hidden-element-tile" : "component-tile"}
      data-component={c.id}
      title={editorText(c.name)}
      draggable={online && !isHidden}
      onContextMenu={(e) => menu(e, c.id, c.name, isHidden)}
      onDragStart={(e) => {
        if(!online||isHidden||!beginEditorDrag({kind:'component',id:c.id},e.dataTransfer)){e.preventDefault();return;}
        setDragChip(e, editorText(c.name));
      }}
      onDragEnd={endEditorDrag}
    >
      <ComponentThumb info={c} project={project} />
      {!isHidden && (
        <Tooltip label={editorText("Add {0} at the playhead", { 0: editorText(c.name) })}>
          <button
            className="tile-add"
            data-testid="component-add"
            disabled={!online}
           
            aria-label={editorText("Add {0}", { 0: editorText(c.name) })}
            onClick={(e) => {
              e.stopPropagation();
              e.currentTarget.blur();
              void actions.addComponent(c.id);
            }}
          >
            <Plus />
          </button>
        </Tooltip>
      )}
      <HideButton id={c.id} name={editorText(c.name)} hidden={isHidden} />
      <div className="tile-name">{editorText(c.name)}</div>
    </div>
  );
  const backgroundHidden = hidden.includes(SOLID);
  const backgroundTile = (isHidden: boolean) => (
    <div key={SOLID} className={`tile comp-tile ${isHidden ? "hidden-tile" : ""}`} data-testid={isHidden ? "hidden-element-tile" : "background-tile"} data-component={SOLID} title={editorText("Background")} onContextMenu={(e) => menu(e, SOLID, "Background", isHidden)}>
      <div className="thumb bg-thumb" style={{ background: BACKGROUND_THUMB }} />
      {!isHidden && (
        <Tooltip label={editorText("Add a background at the playhead")}>
          <button
            className="tile-add"
            data-testid="background-add"
            disabled={!online}
           
            aria-label={editorText("Add a background")}
            onClick={(e) => {
              e.stopPropagation();
              e.currentTarget.blur();
              // D220: with a picture or text selected, behind it or over the whole edit.
              if (!canGoBehind(project, getPlayback().selected)) return void actions.addBackground(BACKGROUND_PROPS);
              const r = e.currentTarget.getBoundingClientRect();
              openMenu(r.left, r.bottom + 4, [
                { id: "background-behind", get label() { return editorText("Behind this"); }, run: () => void actions.addBackground(BACKGROUND_PROPS, true) },
                { id: "background-whole", get label() { return editorText("Whole edit"); }, run: () => void actions.addBackground(BACKGROUND_PROPS) },
              ], "Add a background");
            }}
          >
            <Plus />
          </button>
        </Tooltip>
      )}
      <HideButton id={SOLID} name="Background" hidden={isHidden} />
      <div className="tile-name">{editorText("Background")}</div>
    </div>
  );
  const searching = query.trim() !== "";
  const stickerResults = searchStickers(query);
  // Each section's visible tiles, filtered by the search.
  const tilesIn = (cat: ElementCategory) => {
    const tiles: Array<React.ReactNode> = [];
    for (const c of COMPONENTS) if (elementCategory(c.id).category === cat && !hidden.includes(c.id) && matchesEditorAsset(query,...tileWords(c.name,c.id))) tiles.push(componentTile(c, false));
    if (cat === "Shapes & media" && !backgroundHidden && matchesEditorAsset(query,"Background","Shapes & media","background colour color gradient fill backdrop","背景 颜色 渐变 填充 底色")) tiles.push(backgroundTile(false));
    return tiles;
  };
  const sections = ELEMENT_CATEGORIES.map((cat) => ({ cat, tiles: tilesIn(cat) })).filter(({ cat, tiles }) => tiles.length > 0 || cat === "Stickers" && (!searching || stickerResults.length > 0));
  const shown = sections.filter(({ cat }) => !only || cat === only);
  const hiddenCount = COMPONENTS.filter((c) => hidden.includes(c.id)).length + (backgroundHidden ? 1 : 0);
  return (
    <aside className="library" data-testid="elements-library">
      <div className="lib-head">
        {editorText("Elements")}{/* A plain select, not a row of chips (D263): the sections show as headers when it's All. */}
        {!searching && (
          <select className="field input elements-section" data-testid="elements-section" aria-label={editorText("Section")} value={only ?? ""} onChange={(e) => setOnly((e.target.value || undefined) as ElementCategory | undefined)}>
            <option value="">{editorText("All sections")}</option>
            {sections.map(({ cat }) => (
              <option key={cat} value={cat}>
                {editorText(cat)}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className="elements-top">
        <label className="elements-search">
          <Search aria-hidden />
          {/* One search over elements and stickers; its test id stays the stickers' one (D165). */}
          <input className="field input" data-testid="sticker-search" placeholder={editorText("Search elements")} value={query} spellCheck={false} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Escape" && query && (e.stopPropagation(), setQuery(""))} />
          {query && (
            <button className="icon-btn elements-search-clear" data-testid="elements-search-clear" aria-label={editorText("Clear the search")} onClick={() => setQuery("")}>
              <X />
            </button>
          )}
        </label>
      </div>
      {(searching ? sections : shown).map(({ cat, tiles }) =>
        cat === "Stickers" ? (
          <div key={cat} className="lib-section" data-testid="elements-category" data-category={cat}>
            <div className="lib-label elements-section-label">{editorText(cat)}</div>
            {tiles.length > 0 && <div className="grid2" data-testid="components-grid">{tiles}</div>}
            {(!searching || stickerResults.length > 0) && <div data-testid="stickers">
              <StickerPicker query={query} results={stickerResults} />
            </div>}
          </div>
        ) : tiles.length ? (
          <div key={cat} className="lib-section" data-testid="elements-category" data-category={cat}>
            <div className="lib-label elements-section-label">{editorText(cat)}</div>
            <div className="grid2" data-testid="components-grid">
              {tiles}
            </div>
          </div>
        ) : null,
      )}
      {searching && sections.length === 0 && <div className="lib-section muted" role="status">{editorText('Nothing matches')}</div>}
      {hiddenCount > 0 && !searching && !only && (
        <div className="lib-section">
          <button className="lib-label hidden-toggle" data-testid="hidden-elements-toggle" aria-expanded={showHidden} onClick={() => setShowHidden(!showHidden)}>
            {showHidden ? <ChevronDown /> : <ChevronRight />}
            {editorText("Hidden (")}{hiddenCount})
          </button>
          {showHidden && (
            <div className="grid2" data-testid="hidden-elements">
              {COMPONENTS.filter((c) => hidden.includes(c.id)).map((c) => componentTile(c, true))}
              {backgroundHidden && backgroundTile(true)}
            </div>
          )}
        </div>
      )}
    </aside>
  );
}
