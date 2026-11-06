// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// How the text Inspector edits (D129): every change goes to the selected text and, with several
// texts selected (⌘-click, marquee), to each of them, as one edit (planMany: one ⌘Z). preview()
// drafts it in the canvas while a control moves; commit() sends it once (dropping that draft).
import type { Item } from "@/app/lib/editor/core/timeline/types";

import { findItem, type Plan } from "@/app/ui/editor/timeline/edit";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { type ProjectModel } from "@/app/lib/editor/state/project";
import { planMany } from "@/app/ui/editor/preview/text";
import {useProjectEdit} from '../../workspace/projectEdit';
import type {GestureOrigin} from '../../workspace/pointerGesture';

type Make = (project: ProjectModel, id: string) => Plan | { readonly problem: string };

export function useTextEdit(project: ProjectModel, item: Item) {
  const also = usePlayback((s) => s.also);
  // Others selected with it, of the same kind (text with text, captions with captions).
  const ids = [item.id, ...also.filter((id) => id !== item.id && findItem(project, id)?.item.type === item.type)];
  const edit=useProjectEdit();
  return {
    ids,
    cancel:edit.cancel,
    /** Draft a change in the preview (a slider moving, a colour streaming). */
    preview(make: Make) {
      edit.preview(project,planMany(project,ids,make));
    },
    /** Send a change as one edit; any draft goes once it's applied. */
    commit(make: Make,origin?:GestureOrigin) {
      void edit.commit(p=>planMany(p,ids,make),origin);
    },
  };
}
export type TextEdit = ReturnType<typeof useTextEdit>;
