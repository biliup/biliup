// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Browser capture composition: the selected screen becomes visible once sharing starts.
// Camera placement uses the same layout boxes as the imported timeline clips.
import { Monitor } from "lucide-react";
import { useEffect, useRef } from "react";
import { type MultiLayoutName, multiLayoutBoxes } from "@/app/lib/editor/core/timeline/place";

import { useRecorder } from "@/app/ui/editor/capture/recorder";

const pctOf = (n: number, of: number) => `${(n / of) * 100}%`;

export function Composite({ camera, screen, layout, canvas }: { camera: boolean; screen: boolean; layout: MultiLayoutName; canvas: { width: number; height: number } }) {
  const preview = useRecorder((s) => s.preview);
  const previewError = useRecorder((s) => s.previewError);
  const live = useRecorder((s) => s.screenLive);
  const cam = useRef<HTMLVideoElement>(null);
  const scr = useRef<HTMLVideoElement>(null);
  useEffect(() => {
    if (cam.current && cam.current.srcObject !== (preview ?? null)) cam.current.srcObject = preview ?? null;
  }, [preview, camera]);
  useEffect(() => {
    if (scr.current && scr.current.srcObject !== (live ?? null)) scr.current.srcObject = live ?? null;
  }, [live, screen]);
  const boxes = multiLayoutBoxes(layout, canvas.width, canvas.height);
  const at = (b: { x: number; y: number; w: number; h: number; radius?: number }) => ({ left: pctOf(b.x, canvas.width), top: pctOf(b.y, canvas.height), width: pctOf(b.w, canvas.width), height: pctOf(b.h, canvas.height), borderRadius: b.radius ? "50%" : undefined });
  const full = { left: 0, top: 0, width: "100%", height: "100%" };
  const screenBox = screen && camera && boxes.screen ? at(boxes.screen) : full;
  return (
    <div className="capture-composite" data-testid="capture-composite" data-layout={layout} data-live={live ? "yes" : "no"} style={{ aspectRatio: `${canvas.width} / ${canvas.height}` }}>
      {screen && (
        <div className="composite-screen" data-testid="capture-composite-screen" style={screenBox}>
          {live ? <video ref={scr} autoPlay muted playsInline /> : <Monitor className="composite-placeholder" />}
        </div>
      )}
      {camera && (
        <div className={`composite-camera ${screen ? "" : "full"}`} data-testid="capture-composite-camera" style={screen ? at(boxes.camera) : full}>
          <video ref={cam} data-testid="capture-camera-preview" autoPlay muted playsInline />
        </div>
      )}
      {camera && previewError && <div className="composite-error inline-error small">{previewError}</div>}
    </div>
  );
}
