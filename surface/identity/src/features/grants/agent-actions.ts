import type { GrantModel } from '../../generated/grants';

/** The prefix of the relation that carries one action alone, as the shipped model names it. */
export const ONE = 'only.';

/**
 * The actions an agent may be offered from `actions` on a resource of `kind`:
 * those the service has not withheld from agents. Until the served model names
 * its withheld list, nothing is offered, never everything. An app's resource
 * (a kind with a dot) offers nothing: the service refuses every agent grant on
 * one until the app opts in, so no checkbox may promise otherwise.
 */
export function agentOffer(model: GrantModel, kind: string, actions: string[]): { known: boolean; offered: string[] } {
  const withheld = (model as { withheld_from_agents?: unknown }).withheld_from_agents;
  if (!Array.isArray(withheld) || !withheld.every((each) => typeof each === 'string')) return { known: false, offered: [] };
  if (kind.includes('.')) return { known: true, offered: [] };
  const kept = new Set(withheld);
  return { known: true, offered: [...actions].filter((action) => !kept.has(action)).sort() };
}

/** The relation that carries one action alone. */
export const onlyRelation = (action: string): string => ONE + action;
