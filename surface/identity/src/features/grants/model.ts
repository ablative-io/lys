import { api } from '../../api';
import type { LifecycleState, MeView, PeopleView } from '../../generated';
import { resourceText } from '../../generated/grants';
import type { Grant, GrantList, GrantModel, PassOn, ResourceRef } from '../../generated/grants';
import { fileNo } from '../people/directory';
import { clock, day } from '../file/time';

/** Who everyone is, as far as the caller may see: name and state by id. */
export interface Who {
  name: string;
  state: LifecycleState;
  kind: 'person' | 'agent';
  responsible: string | null;
}

/** Everything the grant screens read, all from the service. */
export interface GrantWorld {
  me: MeView;
  people: PeopleView;
  list: GrantList;
  /** The permission model the service judges grants against: its version, and each relation's actions. */
  model: GrantModel;
  byId: Map<string, Grant>;
  who: Map<string, Who>;
}

export async function readGrantWorld(): Promise<GrantWorld> {
  const [me, people, list, model] = await Promise.all([api.me(), api.people(), api.grants(), api.model()]);
  const who = new Map<string, Who>();
  for (const p of people.people) {
    who.set(p.id, { name: p.display_name, state: p.state, kind: 'person', responsible: null });
    for (const a of p.agents) who.set(a.id, { name: a.display_name, state: a.state, kind: 'agent', responsible: p.id });
  }
  who.set(me.person.id, { name: me.person.display_name, state: me.person.state, kind: 'person', responsible: null });
  return { me, people, list, model, byId: new Map(list.grants.map((g) => [g.id, g])), who };
}

export const nameOf = (w: GrantWorld, id: string): string => w.who.get(id)?.name ?? fileNo(id);

/** A grant's short name, as the mock-up writes `G-5`. */
export const grantNo = (id: string): string => 'G/' + id.slice(id.indexOf('-') + 1, id.indexOf('-') + 9);

export const onText = (g: Grant): string => resourceText(g.resource);

/** A resource as the mock-up's pickers name it: a project or organisation by its id, anything else as `name (type)`. */
export const resourceLabel = (r: ResourceRef): string =>
  r.kind === 'project' || r.kind === 'organisation' ? resourceText(r) : `${r.id} (${r.kind})`;

/** The chain from the root grant down to `g`. */
export function chainOf(w: GrantWorld, g: Grant): Grant[] {
  const out: Grant[] = [];
  let at: Grant | undefined = g;
  while (at && out.length < 64) {
    out.unshift(at);
    at = at.source ? w.byId.get(at.source) : undefined;
  }
  return out;
}

/** Why a grant does not stand, as the service judged it over its whole chain. */
export interface Void {
  why: string;
  /** What a suspension refuses is an open question, so a suspended holder's refusal says so. */
  open: boolean;
}

/**
 * Why `g` does not stand, in the service's words, or null when the service
 * answers that it stands. Nothing here walks a chain, reads a clock or decides
 * standing from a holder's state: the service's standing is the only answer.
 */
export function voidOf(w: GrantWorld, g: Grant): Void | null {
  const s = g.standing;
  if (s.stands) return null;
  const upstream = s.grant !== null && s.grant !== g.id;
  const why = upstream ? `it derives from ${grantNo(s.grant ?? '')}, which no longer stands (${s.reason})` : s.reason;
  return { why, open: s.refusal === 'IdentityNotActive' && w.who.get(g.holder)?.state === 'suspended' };
}

/** "Passable", in the mock-up's words: to whom it may be passed on, or `not passable`. */
export function passText(p: PassOn): string {
  if (p.kind === 'use_only') return 'not passable';
  const to: string[] = [];
  if (p.recipients.includes('person')) to.push('human');
  if (p.recipients.includes('agent')) to.push('agent');
  return to.join(', ');
}

export const passesToAgents = (p: PassOn): boolean => p.kind === 'to' && p.recipients.includes('agent');

/** When it was last seen exercised; a grant with no observed use reads `not seen`, never `never used` (conformance 8.4). */
export const lastUsedText = (g: Grant): string => (g.last_use.seen ? clock(g.last_use.at) + ' · ' + g.last_use.route : 'not seen');

export const lastsText = (g: Grant): string => (g.window.ends_at === null ? 'no end' : 'until ' + day(g.window.ends_at));

/**
 * Each relation the service's model defines, with the actions it carries, widest
 * first as the mock-up lists them (owner, editor, viewer). The order is read from
 * the actions each carries, never from a relation's name.
 */
export function relationsOf(w: GrantWorld): [string, string[]][] {
  const out: [string, string[]][] = Object.entries(w.model.relations).map(([r, actions]) => [r, [...actions].sort()]);
  return out.sort(([a, x], [b, y]) => y.length - x.length || a.localeCompare(b));
}

/**
 * The relations that carry `action`, as the mock-up's answer names them
 * (`post needs editors or owners`): narrowest first, by the actions each carries.
 * The relation of the grant exercised is always among them.
 */
export function needsText(w: GrantWorld, action: string, held: string | null): string {
  const names = relationsOf(w).filter(([, actions]) => actions.includes(action)).map(([r]) => r).reverse();
  if (held !== null && !names.includes(held)) names.push(held);
  return `${action} needs ${names.map((r) => r + 's').join(' or ')}`;
}

/** Whether every action of `actions` is one `g` lets its holder pass on. */
export const withinPassOn = (g: Grant, actions: string[]): boolean =>
  g.pass_on.kind === 'to' && actions.every((a) => (g.pass_on as { actions: string[] }).actions.includes(a));

/**
 * What the caller cannot give an agent from the grants the service answered, each
 * with its reason (conformance 2.4): what may not be passed on, then more than is held.
 */
export function cannotGive(w: GrantWorld, source: Grant | null): [string, string][] {
  const mine = w.list.grants.filter((g) => g.holder === w.me.person.id && g.standing.stands);
  const out: [string, string][] = [];
  for (const g of mine) {
    if (g.pass_on.kind === 'use_only') out.push([`${g.relation} of ${onText(g)}`, `You may use it; ${grantNo(g.id)} does not let you pass it on.`]);
    else if (!g.pass_on.recipients.includes('agent')) out.push([`${g.relation} of ${onText(g)}`, `People only, never agents: ${grantNo(g.id)} passes on to people.`]);
  }
  if (source) {
    for (const [relation, actions] of relationsOf(w)) {
      if (!withinPassOn(source, actions)) out.push([`${relation} of ${onText(source)}`, 'More than you hold.']);
    }
  }
  return out;
}
