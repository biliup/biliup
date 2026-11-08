// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The library panel on the left: the media in the project (import, drop, add to the timeline)
// the Text tab's presets and the Elements tab's components and backgrounds. Which one shows is
// the rail's choice.

import { type Project } from "@/app/lib/editor/state/store";
import type { LibraryPanel } from "@/app/lib/editor/contracts";
import { CaptureLibrary } from "@/app/ui/editor/panels/Capture";
import { AudioLibrary, SfxLibrary } from "@/app/ui/editor/panels/Audio";
import { TransitionsLibrary } from "@/app/ui/editor/panels/Transitions";
import { EffectsLibrary } from "@/app/ui/editor/panels/Effects";
import { GenerateLibrary } from "@/app/ui/editor/panels/Generate";
import { TranscriptPanel } from "@/app/ui/editor/panels/TranscriptPanel";
import { BrandLibrary } from "@/app/ui/editor/panels/Brand";
import { SkillsLibrary } from "@/app/ui/editor/panels/Skills";
import { TextLibrary } from "@/app/ui/editor/panels/library/TextLibrary";
import { ElementsLibrary } from "@/app/ui/editor/panels/library/ElementsLibrary";
import { MediaLibrary } from "@/app/ui/editor/panels/library/MediaLibrary";
export { mediaMeta, Empty } from "@/app/ui/editor/panels/library/MediaTile";

export type LibraryTab = LibraryPanel;

export function Library({ project, dir, tab, onTab }: { project: Project; dir: string; tab: LibraryTab; onTab?: (tab: LibraryTab) => void }) {
  return tab === "brand" ? (
    <BrandLibrary />
  ) : tab === "skills" ? (
    <SkillsLibrary />
  ) : tab === "transcript" ? (
    <TranscriptPanel project={project} dir={dir} />
  ) : tab === "text" ? (
    <TextLibrary project={project} />
  ) : tab === "elements" ? (
    <ElementsLibrary project={project} />
  ) : tab === "audio" ? (
    <AudioLibrary project={project} />
  ) : tab === "sfx" ? (
    <SfxLibrary project={project} />
  ) : tab === "transitions" ? (
    <TransitionsLibrary project={project} />
  ) : tab === "effects" ? (
    <EffectsLibrary project={project} />
  ) : tab === "capture" ? (
    <CaptureLibrary />
  ) : tab === "generate" ? (
    <GenerateLibrary project={project} />
  ) : (
    <MediaLibrary project={project} onEdit={() => onTab?.("generate")} />
  );
}
