import type { Grant, ResourceRef } from '../../generated/grants';
import { delegateChoices } from '../grants/delegateChoices';
import type { GrantWorld } from '../grants/model';
import { passesToAgents } from '../grants/model';
import { pendingGrantKey, readPendingGrant, readPendingRest } from '../grants/pendingGrant';

type Refusal = { at: 'refused'; reason: string };
type Target = { at: 'target'; resource: ResourceRef; actions: string[] };
export type GiveLink = { at: 'absent' } | Refusal | { at: 'ready'; source: Grant; choices: string[]; resource: ResourceRef; actions: string[] };
const refuse = (reason: string): Refusal => ({ at: 'refused', reason });
const decode = (value: string) => decodeURIComponent(value.replace(/\+/g, ' '));

/** Split before decoding so an escaped delimiter stays part of its identifier. */
function targetFrom(query: string): Target | Refusal {
  try {
    const members = query
      .replace(/^\?/, '')
      .split('&')
      .map((part) => {
        const equal = part.indexOf('=');
        return { name: decode(equal < 0 ? part : part.slice(0, equal)), value: equal < 0 ? '' : part.slice(equal + 1) };
      });
    for (const { name } of members)
      if (name !== 'give') return refuse(`The give link contains the unsupported query member ${name || '(empty)'}. Only give is allowed.`);
    if (members.length !== 1) return refuse('The link must contain exactly one give query member.');
    const value = members[0].value;
    const first = value.indexOf(':');
    const last = value.lastIndexOf(':');
    if (first < 1) return refuse('The give link is malformed. Use resource:id:action.');
    if (last === first) return refuse('The give link is missing an action. Use resource:id:action.');
    const resource = { kind: decode(value.slice(0, first)), id: decode(value.slice(first + 1, last)) };
    const actions = decode(value.slice(last + 1)).split(',');
    if (!resource.id) return refuse('The give link is missing a resource id (store).');
    if (actions.some((action) => !action.trim())) return refuse('The give link is missing an action.');
    if (new Set(actions).size !== actions.length) return refuse('The give link repeats an action.');
    return { at: 'target', resource, actions };
  } catch {
    return refuse('The give link contains invalid percent encoding.');
  }
}

/** A link can select only complete, visible form choices from one standing grant. */
export function giveLink(query: string, w: GrantWorld, recipient: string): GiveLink {
  if (!query) return { at: 'absent' };
  const target = targetFrom(query);
  if (target.at === 'refused') return target;
  const { resource, actions } = target;
  const kindKnown = resource.kind.includes('.')
    ? Object.hasOwn(w.model.kinds, resource.kind)
    : w.list.grants.some((grant) => grant.resource.kind === resource.kind);
  if (!kindKnown) return refuse(`Unknown resource kind ${resource.kind}. Nothing was filled.`);
  const who = w.who.get(recipient);
  if (!who || who.kind !== 'agent') return refuse(`This agent does not exist among the records you can see: ${recipient}.`);
  if (who.responsible !== w.me.person.id || who.state === 'retired')
    return refuse(`You cannot give access to ${who.name} (${recipient}); choose one of your agents that is not retired.`);
  const declared = resource.kind.includes('.') ? w.model.kinds[resource.kind].relations : w.model.relations;
  const agentActions = resource.kind.includes('.')
    ? w.model.kinds[resource.kind].agents
    : Object.values(declared)
        .flat()
        .filter((action) => !w.model.withheld_from_agents.includes(action));
  for (const action of actions) {
    if (
      !agentActions.includes(action) ||
      !Object.values(declared).some((carried) => carried.includes(action) && carried.every((other) => actions.includes(other)))
    ) {
      return refuse(`The Give form cannot offer action ${action} alone as requested on ${resource.kind}:${resource.id}. Nothing was filled.`);
    }
  }
  const sources = w.list.grants.filter(
    (grant) =>
      grant.holder === w.me.person.id &&
      grant.standing.stands &&
      passesToAgents(grant.pass_on) &&
      grant.resource.kind === resource.kind &&
      grant.resource.id === resource.id
  );
  for (const source of sources) {
    const { choices } = delegateChoices(w.model, source);
    const exact = [...choices].filter(([, choice]) => choice.actions.every((action) => actions.includes(action)));
    if (!actions.every((action) => exact.some(([, choice]) => choice.actions.includes(action)))) continue;
    const key = pendingGrantKey(w.me.person.id, source.id);
    if (readPendingGrant(key, w.me.person.id, source.id).kind !== 'empty' || readPendingRest(key, w.me.person.id, source.id).kind !== 'empty') {
      return refuse('This source has an earlier grant request. Open Give manually to settle its original choices before using this link. Nothing was filled.');
    }
    const selected: string[] = [];
    const covered = new Set<string>();
    for (const [id, choice] of exact.sort(
      ([, a], [, b]) => b.actions.length - a.actions.length || actions.indexOf(a.actions[0]) - actions.indexOf(b.actions[0])
    )) {
      if (choice.actions.every((action) => covered.has(action))) continue;
      selected.push(id);
      choice.actions.forEach((action) => covered.add(action));
    }
    return { at: 'ready', source, choices: selected, resource, actions };
  }
  return refuse(
    `You hold no grant for ${resource.kind}:${resource.id} with actions ${actions.join(', ')} that lets you pass it on to an agent. Nothing was filled.`
  );
}
