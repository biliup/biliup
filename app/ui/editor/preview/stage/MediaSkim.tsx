// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Issue #118: a media tile skimmed in the library shows in the main preview too (as in iMovie): one
// muted <video> of the file (its proxy when the preview uses proxies) over the frame, seeked to the
// skimmed time, letterboxed in black. Gone once the pointer leaves the tile; the project is drawn
// underneath all along, so the playhead's frame is back at once.
import { useSyncExternalStore } from "react";

import { usePlayback } from "@/app/ui/editor/preview/playback";
import { getState, mediaUrl, projectOf, proxyOf, proxyUrl, subscribeApp } from "@/app/lib/editor/state/store";
import { useProxiesOn } from "@/app/ui/editor/settings/proxies";
import {fallbackProxy} from '../compositor/media';
import {MediaVideo} from '../MediaVideo';

export function MediaSkim() {
  const hover = usePlayback((s) => s.hoverMedia);
  const media = useSyncExternalStore(subscribeApp, () => {
    const snapshot = getState().snapshot;
    return hover && snapshot ? projectOf(snapshot).media.find((m) => m.id === hover.media) : undefined;
  });
  const proxy = useSyncExternalStore(subscribeApp, () => (media ? proxyOf(getState(), media) : undefined));
  const proxiesOn = useProxiesOn();
  const seconds = hover?.seconds;
  if (!hover || !media || media.kind !== "video") return null;
  const src = proxiesOn && proxy?.phase === "done" && proxy.key ? proxyUrl(media.id, proxy.key) : mediaUrl(media.id, media.path);
  return <MediaVideo key={media.id} media={media} position={seconds} className="pv-skim" data-testid="media-skim" data-media={media.id} data-seconds={seconds} src={src} onError={e=>fallbackProxy(e.currentTarget,src,mediaUrl(media.id,media.path))} muted playsInline preload="auto" />;
}
