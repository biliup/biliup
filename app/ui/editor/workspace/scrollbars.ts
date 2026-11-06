import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D209 (issue #29): thin overlay scrollbars, one style for every scroll area (panels, timeline,
// Inspector, Agent, dialogs). The native ones are hidden (they took space and were wide when macOS
// shows them always); these are drawn over the content, so they take no room. A bar shows while
// its area scrolls and while the pointer is over the area, fades out after HIDE_MS, and can be
// dragged by its thumb, which takes the pointer only once the pointer has rested on it.
const HIDE_MS = 900;
const MIN_THUMB = 24;
const EDGE = 2;

type Axis = "y" | "x";
interface Bar {
  readonly el: HTMLElement;
  readonly axis: Axis;
  readonly track: HTMLDivElement;
  readonly thumb: HTMLDivElement;
  timer: ReturnType<typeof setTimeout> | undefined;
  held: boolean;
  hovered: boolean;
}

let layer: HTMLDivElement | undefined;
const bars = new Map<string, Bar>();
const ids = new WeakMap<HTMLElement, number>();
let nextId = 1;
const keyOf = (el: HTMLElement, axis: Axis) => {
  let id = ids.get(el);
  if (!id) ids.set(el, (id = nextId++));
  return `${id}:${axis}`;
};

/** Whether `el` scrolls along `axis` (overflow auto or scroll, with more content than room). */
function scrolls(el: HTMLElement, axis: Axis) {
  const style = getComputedStyle(el);
  const overflow = axis === "y" ? style.overflowY : style.overflowX;
  if (overflow !== "auto" && overflow !== "scroll") return false;
  return axis === "y" ? el.scrollHeight > el.clientHeight + 1 : el.scrollWidth > el.clientWidth + 1;
}

/** The nearest area from `start` up that scrolls along `axis`. */
function scroller(start: Element | null, axis: Axis): HTMLElement | undefined {
  for (let el = start instanceof HTMLElement ? start : start?.parentElement; el && el !== editorRoot() && el !== editorRoot(); el = el.parentElement) {
    if (el.closest("[data-scrollbars=off]")) return undefined;
    if (scrolls(el, axis)) return el;
  }
  return undefined;
}

function place(bar: Bar) {
  const { el, axis, track, thumb } = bar;
  const r = el.getBoundingClientRect();
  if (!el.isConnected || r.width === 0 || r.height === 0 || !scrolls(el, axis)) return hide(bar, true);
  if (axis === "y") {
    const size = el.clientHeight;
    const len = Math.max(MIN_THUMB, (size / el.scrollHeight) * (r.height - EDGE * 2));
    const room = r.height - EDGE * 2 - len;
    const max = el.scrollHeight - size;
    track.style.cssText = `left:${r.right - 10}px;top:${r.top + EDGE}px;width:10px;height:${r.height - EDGE * 2}px`;
    thumb.style.cssText = `top:${max > 0 ? (el.scrollTop / max) * room : 0}px;height:${len}px`;
  } else {
    const size = el.clientWidth;
    const len = Math.max(MIN_THUMB, (size / el.scrollWidth) * (r.width - EDGE * 2));
    const room = r.width - EDGE * 2 - len;
    const max = el.scrollWidth - size;
    track.style.cssText = `left:${r.left + EDGE}px;top:${r.bottom - 10}px;height:10px;width:${r.width - EDGE * 2}px`;
    thumb.style.cssText = `left:${max > 0 ? (el.scrollLeft / max) * room : 0}px;width:${len}px`;
  }
}

function hide(bar: Bar, now = false) {
  clearTimeout(bar.timer);
  const go = () => {
    // An area that's gone (a screen closed under the pointer) takes its bar with it.
    if ((bar.held || bar.hovered) && bar.el.isConnected) return;
    bar.track.dataset.visible = "false";
    // Removed once it has faded (a scroll in the meantime brings it back).
    bar.timer = setTimeout(() => {
      if (bar.track.dataset.visible === "false") {
        bar.track.remove();
        bars.delete(keyOf(bar.el, bar.axis));
      }
    }, 250);
  };
  if (now) go();
  else bar.timer = setTimeout(go, HIDE_MS);
}

function show(el: HTMLElement, axis: Axis) {
  if (!layer) return;
  const key = keyOf(el, axis);
  let bar = bars.get(key);
  if (!bar) {
    const track = document.createElement("div");
    track.className = `overlay-scrollbar ${axis}`;
    track.dataset.testid = "scrollbar";
    track.dataset.axis = axis;
    const thumb = document.createElement("div");
    thumb.className = "overlay-thumb";
    thumb.dataset.testid = "scrollbar-thumb";
    track.append(thumb);
    layer.append(track);
    bar = { el, axis, track, thumb, timer: undefined, held: false, hovered: false };
    bars.set(key, bar);
    wire(bar);
  }
  clearTimeout(bar.timer);
  place(bar);
  bar.track.dataset.visible = "true";
  hide(bar);
}

function wire(bar: Bar) {
  const { el, axis, track, thumb } = bar;
  thumb.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const rect = track.getBoundingClientRect();
    const t = thumb.getBoundingClientRect();
    const along = (ev: PointerEvent) => (axis === "y" ? ev.clientY : ev.clientX);
    const room = (axis === "y" ? rect.height - t.height : rect.width - t.width) || 1;
    const max = axis === "y" ? el.scrollHeight - el.clientHeight : el.scrollWidth - el.clientWidth;
    bar.held = true;
    track.dataset.held = "true";
    const p0 = along(e);
    const s0 = axis === "y" ? el.scrollTop : el.scrollLeft;
    thumb.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      const v = s0 + ((along(ev) - p0) / room) * max;
      if (axis === "y") el.scrollTop = v;
      else el.scrollLeft = v;
    };
    const up = () => {
      bar.held = false;
      delete track.dataset.held;
      delete thumb.dataset.armed;
      bar.hovered = false;
      thumb.removeEventListener("pointermove", move);
      thumb.removeEventListener("pointerup", up);
      thumb.removeEventListener("pointercancel", up);
      hide(bar);
    };
    thumb.addEventListener("pointermove", move);
    thumb.addEventListener("pointerup", up);
    thumb.addEventListener("pointercancel", up);
  });
}

/** How long the pointer rests on a thumb before the thumb takes it (D209). */
const ARM_MS = 250;
let armTimer: ReturnType<typeof setTimeout> | undefined;
let arming: Bar | undefined;

/**
 * A thumb takes the pointer only after the pointer has rested on it (ARM_MS): a bar drawn over an
 * area's edge never swallows a press that was aimed at what's under it (a clip on the timeline's
 * bottom lane, a row's button), the way macOS's overlay scrollers only get hot when hovered.
 */
function arm(x: number, y: number) {
  let over: Bar | undefined;
  for (const bar of bars.values()) {
    if (bar.track.dataset.visible !== "true") continue;
    const r = bar.thumb.getBoundingClientRect();
    if (x >= r.left - 2 && x <= r.right + 2 && y >= r.top - 2 && y <= r.bottom + 2) over = bar;
    else if (bar.thumb.dataset.armed && !bar.held) {
      delete bar.thumb.dataset.armed;
      bar.hovered = false;
      hide(bar);
    }
  }
  if (over === arming) return;
  clearTimeout(armTimer);
  arming = over;
  if (!over || over.thumb.dataset.armed) return;
  const bar = over;
  armTimer = setTimeout(() => {
    if (arming !== bar) return;
    bar.thumb.dataset.armed = "true";
    bar.hovered = true;
    clearTimeout(bar.timer);
  }, ARM_MS);
}

let hoverFrame = 0;
let lastPoint = { x: -1, y: -1 };
let lastHover: Element | null = null;

/** Installs the overlay scrollbars for the whole page (once). */
export function installScrollbars() {
  if (layer) return;
  const root = editorRoot();
  const controller = new AbortController();
  editorRoot().classList.add("overlay-scrollbars");
  layer = document.createElement("div");
  layer.className = "overlay-scrollbars-layer";
  editorRoot().append(layer);
  root.addEventListener(
    "scroll",
    (e) => {
      const el = e.target;
      if (!(el instanceof HTMLElement)) return;
      // An area that opts out (the rail, D263) gets no bar while it scrolls either.
      if (el.closest("[data-scrollbars=off]")) return;
      if (scrolls(el, "y")) show(el, "y");
      if (scrolls(el, "x")) show(el, "x");
    },
    { capture: true, signal: controller.signal },
  );
  // Over an area that scrolls: its bars show (and stay while the pointer is there).
  root.addEventListener(
    "pointermove",
    (e) => {
      if (e.buttons) return;
      arm(e.clientX, e.clientY);
      lastHover = e.target as Element | null;
      lastPoint = { x: e.clientX, y: e.clientY };
      if (hoverFrame) return;
      hoverFrame = requestAnimationFrame(() => {
        hoverFrame = 0;
        if (lastHover?.closest?.(".overlay-scrollbar")) return;
        const y = scroller(lastHover, "y");
        const x = scroller(lastHover, "x");
        if (y) show(y, "y");
        if (x) show(x, "x");
        // A bar that just showed under a resting pointer can arm too.
        arm(lastPoint.x, lastPoint.y);
      });
    },
    { passive: true, signal: controller.signal },
  );
  const replace = () => {
    for (const bar of bars.values()) if (bar.track.dataset.visible === "true") place(bar);
  };
  window.addEventListener("resize", replace, { signal: controller.signal });
  // Bars of areas that left the page (the screen changed) go at once.
  const timer = setInterval(() => {
    for (const bar of bars.values()) if (!bar.el.isConnected) {
      bar.held = bar.hovered = false;
      hide(bar, true);
    }
  }, 500);
  return () => {
    controller.abort(); clearInterval(timer); clearTimeout(armTimer); cancelAnimationFrame(hoverFrame);
    for (const bar of bars.values()) clearTimeout(bar.timer);
    bars.clear(); layer?.remove(); layer = undefined; hoverFrame = 0; lastHover = null;
    root.classList.remove('overlay-scrollbars');
  };
}
