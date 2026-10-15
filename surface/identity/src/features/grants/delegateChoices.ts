import type { Grant, GrantModel } from '../../generated/grants';
import { agentMayHold, appAgentCarriers, RELATION_CHOICE, singleActionCarriers, withheldFromAgents } from './ActionPicker';
import { appKind } from './model';

/** The exact choices the agent form can carry, shared with links that fill it. */
export function delegateChoices(model: GrantModel, source: Grant) {
  const app = source.resource.kind.includes('.');
  const declared = app ? appKind(model, source.resource.kind) : null;
  const pass = source.pass_on;
  const passable = pass.kind === 'to' && pass.recipients.includes('agent') ? source.actions.filter((action) => pass.actions.includes(action)) : [];
  const known = declared !== null || withheldFromAgents(model) !== null;
  const offered = known ? passable.filter((action) => agentMayHold(model, source.resource.kind, [action])).sort() : [];
  const carriers = singleActionCarriers(model, source.resource.kind);
  const appChoices = app ? appAgentCarriers(model, source.resource.kind, offered) : [];
  const carried = offered.filter((action) => carriers.has(action) || appChoices.some(([, actions]) => actions.includes(action)));
  const choices = new Map<string, { relation: string; actions: string[] }>();
  for (const action of carried) {
    const carrier = carriers.get(action);
    if (carrier !== undefined) choices.set(action, { relation: carrier, actions: [action] });
  }
  for (const [relation, actions] of appChoices) {
    if (!actions.every((action) => choices.has(action))) choices.set(RELATION_CHOICE + relation, { relation, actions });
  }
  return { known, offered, carried, choices };
}
