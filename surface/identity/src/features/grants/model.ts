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

export interface Standing {
  ok: boolean;
  kind?: string;
  why?: string;
  /** What a suspension refuses is an open question, so a suspended holder's reason says so. */
  open?: boolean;
}

const now = () => Math.floor(Date.now() / 1000);

/** Whether a grant stands, from the facts the service answered: revoked, window, holder state, source. */
export function standing(w: GrantWorld, g: Grant, depth = 0): Standing {
  if (g.revoked) return { ok: false, kind: 'grant revoked', why: `grant ${grantNo(g.id)} was revoked` };
  if (g.window.ends_at !== null && g.window.ends_at <= now()) return { ok: false, kind: 'grant expired', why: `grant ${grantNo(g.id)} ended ${day(g.window.ends_at)}` };
  const holder = w.who.get(g.holder);
  if (holder && holder.state !== 'active') return { ok: false, kind: 'identity not active', why: `${holder.name} is ${holder.state}`, open: holder.state === 'suspended' };
  if (g.source && depth < 64) {
    const up = w.byId.get(g.source);
    if (up) {
      const s = standing(w, up, depth + 1);
      if (!s.ok) return { ok: false, kind: 'authority withdrawn upstream', why: `it derives from ${grantNo(up.id)}, held by ${nameOf(w, up.holder)}, which no longer stands (${s.why})` };
    }
  }
  return { ok: true };
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

/**
 * What the caller cannot give an agent from the grants the service answered, each
 * with its reason (conformance 2.4): what may not be passed on, then more than is held.
 */
export function cannotGive(w: GrantWorld, source: Grant | null): [string, string][] {
  const mine = w.list.grants.filter((g) => g.holder === w.me.person.id && standing(w, g).ok);
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
