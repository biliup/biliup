// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Components (D76): html modules drawn from their manifests (spec/modules/builtin/component.*.json)
// by one pure function, componentHtml(item, target, localFrame), shared by the preview (which puts
// the HTML on the canvas), the export (which has Chromium rasterise it: one PNG per distinct look)
// and the agent's frame renders. So preview == export by construction, as for text (D39).
//
// A component draws in a box like a clip (D75, box.ts): transform {x, y, w, h} in canvas pixels,
// opacity, x / y / opacity / scale keys. No w and h = the manifest's default size (component.size,
// canvas pixels on a canvas whose shorter side is 1080, scaled to the project's), centred; a
// fullFrame component fills the canvas. The body is laid out at the box's size in canvas pixels, so
// resizing the box re-lays it out (text sizes follow the box). scale zooms the whole box about its
// centre (not clipped).
//
// Browser-safe (no node:fs): the manifests are imported as JSON, so the renderer's Inspector reads
// the same params schema the validator checks props against. test/core/components.test.ts keeps the
// registry and the manifests in step: every renderer has a manifest, every built-in component
// manifest is either drawn here or listed in UNDRAWN_COMPONENTS, defaults validate.
//
// Media inside a component (the end card's logo) is a `{ media }` prop; target.mediaUrl turns it
// into a URL (the preview's /media/file URL; the export's data: URL). Footage never goes inside a
// component: a device frame is a bezel with a transparent screen over a separate clip (planDeviceFrame).

export { COMPONENT_IMAGE } from "@/app/lib/editor/core/visual/components/shared";
export type { ComponentBlock, Draw, ComponentInfo } from "@/app/lib/editor/core/visual/components/shared";
export { DEVICES, deviceSpec, screenRect, FRAME_COLOR, bezelSvg } from "@/app/lib/editor/core/visual/components/device";
export type { Device } from "@/app/lib/editor/core/visual/components/device";
export { terminalState, agentLogState } from "@/app/lib/editor/core/visual/components/draw";
export type { TerminalLine } from "@/app/lib/editor/core/visual/components/draw";
export { COMPONENTS, componentInfo, DRAWN_COMPONENTS, UNDRAWN_COMPONENTS, isDrawnComponent, componentScale, naturalSize, componentBox, componentRest, defaultComponentBox, componentDefaults, componentHtml, componentParts, wrapComponent, ELEMENT_CATEGORIES, elementCategory, type ElementCategory } from "@/app/lib/editor/core/visual/components/registry";
export { screenTransform, screenKeys, planDeviceFrame, alignScreen } from "@/app/lib/editor/core/visual/components/deviceFrame";
export type { DeviceFrameRequest } from "@/app/lib/editor/core/visual/components/deviceFrame";
