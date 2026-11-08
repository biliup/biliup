// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Project access is independent of the provider. Model choices and favourites belong
// to the configured API, and share the editor's browser preference lifecycle.
import { AGENT_ACCESS, AGENT_EFFORTS, type AgentAccess, type AgentEffort } from "@/app/lib/editor/contracts";
import { useMemo } from 'react';
import { browserPreference, useBrowserPreference } from '../browser-preference';

export interface AgentPrefs {
  // A model id for the configured provider; undefined uses the server model setting.
  readonly model?: string | undefined;
  // undefined uses the provider's default effort.
  readonly effort?: AgentEffort | undefined;
  readonly access: AgentAccess;
}

// D214: Auto for every project never set; the editor server enforces this scope.
export const DEFAULT_AGENT_PREFS: AgentPrefs = { access: "auto" };
export type AgentModelProvider = 'openai' | 'google';
export const isAgentModelProvider = (value: unknown): value is AgentModelProvider => value === 'openai' || value === 'google';
export const validAgentModel = (value: unknown): value is string => typeof value === 'string' && /^[A-Za-z0-9._-]{1,100}$/.test(value);
export const effortsFor = (provider: string): ReadonlyArray<AgentEffort> => provider === 'openai' ? AGENT_EFFORTS : provider === 'google' ? ['low', 'medium', 'high'] : [];
type ModelPrefs = Pick<AgentPrefs, 'model' | 'effort'>;
interface ProjectPrefs { readonly dir: string; readonly access: AgentAccess; readonly models: Partial<Record<AgentModelProvider, ModelPrefs>> }
interface Favourite { readonly provider: AgentModelProvider; readonly model: string }
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const validAccess = (value: unknown): value is AgentAccess => (AGENT_ACCESS as ReadonlyArray<unknown>).includes(value);
const validModelPrefs = (value: unknown, provider: string): value is ModelPrefs => object(value)
  && (value.model === undefined || validAgentModel(value.model))
  && (value.effort === undefined || (effortsFor(provider) as ReadonlyArray<unknown>).includes(value.effort));
const validProjects = (value: unknown): value is ReadonlyArray<ProjectPrefs> => Array.isArray(value) && value.length <= 50
  && value.every(entry => object(entry) && typeof entry.dir === 'string' && /^biliup:[1-9]\d*$/.test(entry.dir)
    && validAccess(entry.access) && object(entry.models)
    && Object.entries(entry.models).every(([provider, prefs]) => isAgentModelProvider(provider) && validModelPrefs(prefs, provider)))
  && new Set(value.map(entry => entry.dir)).size === value.length;
const projects = browserPreference<ReadonlyArray<ProjectPrefs>>('agent.projects', [], validProjects);
const savedFavourites = browserPreference<ReadonlyArray<Favourite>>('agent.favourites', [], (value): value is ReadonlyArray<Favourite> =>
  Array.isArray(value) && value.every(entry => object(entry) && isAgentModelProvider(entry.provider) && validAgentModel(entry.model))
  && new Set(value.map(entry => `${entry.provider}:${entry.model}`)).size === value.length);

function prefsFor(entries: ReadonlyArray<ProjectPrefs>, dir: string, provider: string): AgentPrefs {
  const entry = entries.find(entry => entry.dir === dir);
  return entry ? { ...(isAgentModelProvider(provider) ? entry.models[provider] : {}), access: entry.access } : DEFAULT_AGENT_PREFS;
}
export const agentPrefs = (dir: string, provider: string) => prefsFor(projects.get(), dir, provider);

export function setAgentPrefs(dir: string, provider: string, patch: Partial<AgentPrefs>) {
  if (!isAgentModelProvider(provider) || !/^biliup:[1-9]\d*$/.test(dir)) return;
  const entries = projects.get(), previous = entries.find(entry => entry.dir === dir);
  const { access, ...model } = { ...agentPrefs(dir, provider), ...patch };
  if (!validAccess(access) || !validModelPrefs(model, provider)) return;
  projects.set([{ dir, access, models: { ...previous?.models, [provider]: model } }, ...entries.filter(entry => entry.dir !== dir)].slice(0, 50));
}

export function toggleFavourite(provider: string, model: string) {
  if (!isAgentModelProvider(provider) || !validAgentModel(model)) return;
  const entries = savedFavourites.get(), contains = entries.some(entry => entry.provider === provider && entry.model === model);
  savedFavourites.set(contains ? entries.filter(entry => entry.provider !== provider || entry.model !== model) : [...entries, { provider, model }]);
}

/** Searchable choices come from actual configuration/user selections, never a speculative model catalogue. */
export function useAgentPrefs(dir: string, provider: string) {
  const entries = useBrowserPreference(projects), favourites = useBrowserPreference(savedFavourites);
  return useMemo(() => {
    const saved = favourites.filter(entry => entry.provider === provider).map(entry => entry.model);
    const recent = isAgentModelProvider(provider) ? entries.flatMap(entry => { const model = entry.models[provider]?.model; return model ? [model] : []; }) : [];
    return { prefs: prefsFor(entries, dir, provider), favourites: saved, models: [...new Set([...saved, ...recent])] };
  }, [entries, favourites, dir, provider]);
}

const skipStoryboard = browserPreference('agent.skipStoryboard', false, (value): value is boolean => typeof value === 'boolean');
export const skipStoryboardSetting = skipStoryboard.get;
export const setSkipStoryboard = skipStoryboard.set;
export const useSkipStoryboard = () => useBrowserPreference(skipStoryboard);
