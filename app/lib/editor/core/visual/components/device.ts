// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import deviceFrameJson from "@/resources/editor/modules/builtin/component.device-frame.json" with { type: "json" };
import type { Rect } from "@/app/lib/editor/core/visual/box";
import { num, DM_SANS, esc } from "@/app/lib/editor/core/visual/render";
import { manifestOf, type Draw, color, str } from "@/app/lib/editor/core/visual/components/shared";

// ---- device frame ----

export type Device = "phone" | "android" | "browser";
export const DEVICES: ReadonlyArray<Device> = ["phone", "android", "browser"];
export const deviceOf = (props: { readonly [key: string]: unknown }): Device => (DEVICES.includes(props.device as Device) ? (props.device as Device) : "phone");

/** The device's frame size and screen rect (its manifest's component.screens), in that size's pixels. */
export const deviceSpec = (device: Device) => manifestOf(deviceFrameJson).component.screens![device]!;

/**
 * Where the screen is in a frame drawn in a w×h box: the device's screen rect scaled to the box
 * (radius by the smaller scale). This is the clip's box under the frame; the bezel's hole is
 * 2 px (scaled) inside it, so the clip's edges always sit under the bezel (no seam).
 */
export function screenRect(device: Device, w: number, h: number): Rect & { readonly radius: number } {
  const spec = deviceSpec(device);
  const sx = w / spec.size.width;
  const sy = h / spec.size.height;
  const s = spec.screen;
  return { x: s.x * sx, y: s.y * sy, w: s.width * sx, h: s.height * sy, radius: (s.radius ?? 0) * Math.min(sx, sy) };
}

const roundRect = (x: number, y: number, w: number, h: number, r: number) => {
  const k = Math.max(0, Math.min(r, w / 2, h / 2));
  return `M${num(x + k)} ${num(y)}H${num(x + w - k)}A${num(k)} ${num(k)} 0 0 1 ${num(x + w)} ${num(y + k)}V${num(y + h - k)}A${num(k)} ${num(k)} 0 0 1 ${num(x + w - k)} ${num(y + h)}H${num(x + k)}A${num(k)} ${num(k)} 0 0 1 ${num(x)} ${num(y + h - k)}V${num(y + k)}A${num(k)} ${num(k)} 0 0 1 ${num(x + k)} ${num(y)}Z`;
};

/** The default frame colour (near-black) and the browser bar's text. */
export const FRAME_COLOR = "#1C1C1F";

/**
 * The bezel as an SVG at w×h (canvas px): the frame filled with `fill`, a transparent hole for the
 * screen (fill-rule evenodd), a hairline edge, and the device's details (camera pill, punch hole,
 * or the browser's dots and address bar). Generic shapes, no brand marks.
 */
export function bezelSvg(device: Device, w: number, h: number, fill: string, url = ""): string {
  const spec = deviceSpec(device);
  const s = Math.min(w / spec.size.width, h / spec.size.height);
  const scr = screenRect(device, w, h);
  const inset = 2 * s;
  const hole = roundRect(scr.x + inset, scr.y + inset, scr.w - 2 * inset, scr.h - 2 * inset, Math.max(0, scr.radius - inset));
  const outerR = device === "phone" ? 66 * s : device === "android" ? 46 * s : 16 * s;
  const outer = roundRect(0, 0, w, h, outerR);
  const parts = [`<path d="${outer} ${hole}" fill="${fill}" fill-rule="evenodd"/>`, `<path d="${roundRect(0.75 * s, 0.75 * s, w - 1.5 * s, h - 1.5 * s, outerR - 0.75 * s)}" fill="none" stroke="rgb(255 255 255 / 14%)" stroke-width="${num(1.5 * s)}"/>`];
  if (device === "phone") parts.push(`<rect x="${num(w / 2 - 60 * s)}" y="${num(scr.y + 18 * s)}" width="${num(120 * s)}" height="${num(34 * s)}" rx="${num(17 * s)}" fill="#000"/>`);
  else if (device === "android") parts.push(`<circle cx="${num(w / 2)}" cy="${num(scr.y + 28 * s)}" r="${num(13 * s)}" fill="#000"/>`);
  else {
    // The browser's top bar: three neutral dots and an address field.
    for (const [i, x] of [24, 46, 68].entries()) parts.push(`<circle cx="${num(x * s)}" cy="${num(22 * s)}" r="${num(6.5 * s)}" fill="rgb(255 255 255 / ${[22, 16, 12][i]}%)"/>`);
    const fw = Math.min(w * 0.5, 640 * s);
    parts.push(`<rect x="${num(w / 2 - fw / 2)}" y="${num(9 * s)}" width="${num(fw)}" height="${num(26 * s)}" rx="${num(13 * s)}" fill="rgb(255 255 255 / 8%)"/>`);
    if (url) parts.push(`<text x="${num(w / 2)}" y="${num(27 * s)}" text-anchor="middle" font-family="${DM_SANS.replace(/'/g, "")}" font-size="${num(14 * s)}" fill="rgb(255 255 255 / 60%)">${esc(url)}</text>`);
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${num(w)}" height="${num(h)}" viewBox="0 0 ${num(w)} ${num(h)}" style="position:absolute;left:0;top:0;display:block;overflow:visible">${parts.join("")}</svg>`;
}

export const drawDeviceFrame = ({ props, w, h, target }: Draw) => bezelSvg(deviceOf(props), w, h, color(target, props.color, FRAME_COLOR), str(props.url));
