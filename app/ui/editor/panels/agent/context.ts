// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { friendlyText } from "@/app/lib/editor/core/text/mentions";
import { createContext, useContext } from "react";
import type { Project } from "@/app/lib/editor/state/store";

/** The project, for names instead of ids in what the cards show (AG5). */
export const ProjectContext = createContext<Project | undefined>(undefined);
export const useFriendly = () => {
  const project = useContext(ProjectContext);
  return (text: string) => friendlyText(project as never, text);
};
