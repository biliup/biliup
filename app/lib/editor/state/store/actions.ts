// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Every action, one object (components call actions.x); each group lives in actions/*.
import { projectActions, projectFileActions } from "@/app/lib/editor/state/store/actions/project";
import { mediaActions } from "@/app/lib/editor/state/store/actions/media";
import { timelineActions } from "@/app/lib/editor/state/store/actions/timeline";
import { agentActions } from "@/app/lib/editor/state/store/actions/agent";
import { brandActions } from "@/app/lib/editor/state/store/actions/brand";
import { exportActions } from "@/app/lib/editor/state/store/actions/export";
import { settingsActions } from "@/app/lib/editor/state/store/actions/settings";
import { captureActions } from "@/app/lib/editor/state/store/actions/capture";
import { generateActions } from "@/app/lib/editor/state/store/actions/generate";

export const actions = {
  ...projectActions,
  ...mediaActions,
  ...timelineActions,
  ...projectFileActions,
  ...agentActions,
  ...brandActions,
  ...exportActions,
  ...settingsActions,
  ...captureActions,
  ...generateActions,
};
