import { actionWords, resourceWords } from './action-words';
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
  kind: 'person' | 'agent' | 'service_account';
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

export async function readGrantWorld(knownMe?: MeView): Promise<GrantWorld> {
  const [me, people, list, model] = await Promise.all([knownMe ?? api.me(), api.people(), api.grants(), api.model()]);
  const who = new Map<string, Who>();
  for (const p of people.people) {
    who.set(p.id, { name: p.display_name, state: p.state, kind: 'person', responsible: null });
    for (const a of p.agents) who.set(a.id, { name: a.display_name, state: a.state, kind: 'agent', responsible: p.id });
  }
  who.set(me.person.id, { name: me.person.display_name, state: me.person.state, kind: 'person', responsible: null });
  for (const account of me.service_accounts) who.set(account.id, { name: account.name === 'Lys directory loader' ? "Lys's own service" : account.name, state: account.state as LifecycleState, kind: 'service_account', responsible: account.owner });
  return { me, people, list, model, byId: new Map(list.grants.map((g) => [g.id, g])), who };
}

export const nameOf = (w: GrantWorld, id: string): string => w.who.get(id)?.name ?? fileNo(id);

/** A grant's short name, as the mock-up writes `G-5`. */
export const grantNo = (id: string): string => 'G/' + id.slice(id.indexOf('-') + 1, id.indexOf('-') + 9);

export const onText = (g: Grant): string => resourceText(g.resource);

/** A grant's resource as a name: a directory collection in plain words (`the agents directory`), anything else as its reference. */
export const sourceText = (g: Grant): string => (g.resource.kind === 'directory' ? resourceWords(g.resource) : onText(g));

/** What follows the actions given on a grant's resource: `on the agents directory` for a directory collection, the reference otherwise. */
export const givenOnText = (g: Grant): string => (g.resource.kind === 'directory' ? 'on ' + resourceWords(g.resource) : onText(g));

/**
 * A resource named in a sentence: a directory collection as the directory's,
 * an identity the caller may see by its name, anything else as its kind and id.
 */
export const objectText = (w: GrantWorld, r: ResourceRef): string => {
  if (r.kind === 'directory') return `the directory's ${r.id}`;
  const who = w.who.get(r.id);
  return `${r.kind} ${who ? who.name : r.id}`;
};

/**
 * What `g` lets its holder do, from the actions the grant itself carries, which
 * are the actions the service checks when it is exercised: the widest-held
 * action first, as the model carries them (view before edit before grant).
 */
export function mayText(w: GrantWorld, g: Grant): string {
  if (!g.actions.length) return `You can take no action on ${objectText(w, g.resource)}.`;
  const listed = actionWords(w.model, g.resource, g.actions);
  return `${listed === 'everything here' ? 'You can do everything here' : listed} (${objectText(w, g.resource)}).`;
}

/**
 * A resource as the mock-up's pickers name it: a project or organisation by its id,
 * a directory collection in plain words (`the agents directory`), anything else as `name (type)`.
 */
export const resourceLabel = (r: ResourceRef): string => {
  if (r.kind === 'project' || r.kind === 'organisation') return resourceText(r);
  return r.kind === 'directory' ? resourceWords(r) : `${r.id} (${r.kind})`;
};

/** The chain from the root grant down to `g`. */
export function chainOf(w: GrantWorld, g: Grant): Grant[] {
  const out: Grant[] = [];
  const seen = new Set<string>();
  let at: Grant | undefined = g;
  // A chain is as long as it is; a grant met twice ends the walk, since a loop has no root to reach.
  while (at && !seen.has(at.id)) {
    seen.add(at.id);
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

/**
 * When it was last seen exercised; a grant with no observed use reads `not seen`, never `never used` (conformance 8.4).
 * A missing use report is said, so a zero recorded count is never read as no use.
 */
export const lastUsedText = (g: Grant): string => {
  const u = g.last_use;
  const seen = u.seen ? clock(u.at) + ' · ' + u.route : 'not seen';
  if (u.source === 'reported') return seen;
  const n = u.unreported.count;
  return seen + ' · ' + n + (n === 1 ? ' use' : ' uses') + ' not recorded';
};

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
