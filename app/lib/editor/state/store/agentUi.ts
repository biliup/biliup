// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
let panelSeq = 0;
/** The next panel request's sequence number (the agent's ui_show and the actions share it). */
export const nextPanelSeq = () => ++panelSeq;
