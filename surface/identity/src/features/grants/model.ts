import { api } from '../../api';
import type { LifecycleState, MeView, PeopleView } from '../../generated';
import { resourceText } from '../../generated/grants';
import type { Grant, GrantList, PassOn } from '../../generated/grants';
import { fileNo } from '../people/directory';
import { day } from '../file/time';

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
  byId: Map<string, Grant>;
  who: Map<string, Who>;
}

export async function readGrantWorld(): Promise<GrantWorld> {
  const [me, people, list] = await Promise.all([api.me(), api.people(), api.grants()]);
  const who = new Map<string, Who>();
  for (const p of people.people) {
    who.set(p.id, { name: p.display_name, state: p.state, kind: 'person', responsible: null });
    for (const a of p.agents) who.set(a.id, { name: a.display_name, state: a.state, kind: 'agent', responsible: p.id });
  }
  who.set(me.person.id, { name: me.person.display_name, state: me.person.state, kind: 'person', responsible: null });
  return { me, people, list, byId: new Map(list.grants.map((g) => [g.id, g])), who };
}

export const nameOf = (w: GrantWorld, id: string): string => w.who.get(id)?.name ?? fileNo(id);

/** A grant's short name, as the mock-up writes `G-5`. */
export const grantNo = (id: string): string => 'G/' + id.slice(id.indexOf('-') + 1, id.indexOf('-') + 9);

export const onText = (g: Grant): string => resourceText(g.resource);

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
}

const now = () => Math.floor(Date.now() / 1000);

/** Whether a grant stands, from the facts the service answered: revoked, window, holder state, source. */
export function standing(w: GrantWorld, g: Grant, depth = 0): Standing {
  if (g.revoked) return { ok: false, kind: 'grant revoked', why: `grant ${grantNo(g.id)} was revoked` };
  if (g.window.ends_at !== null && g.window.ends_at <= now()) return { ok: false, kind: 'grant expired', why: `grant ${grantNo(g.id)} ended ${day(g.window.ends_at)}` };
  const holder = w.who.get(g.holder);
  if (holder && holder.state !== 'active') return { ok: false, kind: 'identity not active', why: `${holder.name} is ${holder.state}` };
  if (g.source && depth < 64) {
    const up = w.byId.get(g.source);
    if (up) {
      const s = standing(w, up, depth + 1);
      if (!s.ok) return { ok: false, kind: 'authority withdrawn upstream', why: `it derives from ${grantNo(up.id)}, held by ${nameOf(w, up.holder)}, which no longer stands (${s.why})` };
    }
  }
  return { ok: true };
}

/** "You may pass it on", in the mock-up's words. */
export function passText(p: PassOn): string {
  if (p.kind === 'use_only') return 'no';
  const people = p.recipients.includes('person');
  const agents = p.recipients.includes('agent');
  return 'yes, to ' + (people && agents ? 'people and agents' : agents ? 'agents' : 'people');
}

export const passesToAgents = (p: PassOn): boolean => p.kind === 'to' && p.recipients.includes('agent');

export const lastsText = (g: Grant): string => (g.window.ends_at === null ? 'no end' : 'until ' + day(g.window.ends_at));

/** Each relation the service has resolved in a grant the caller can see, with its actions. */
export function relationsSeen(w: GrantWorld): Map<string, string[]> {
  const out = new Map<string, string[]>();
  for (const g of w.list.grants) if (!out.has(g.relation)) out.set(g.relation, [...g.actions].sort());
  return out;
}

/** Whether every action of `actions` is one `g` lets its holder pass on. */
export const withinPassOn = (g: Grant, actions: string[]): boolean =>
  g.pass_on.kind === 'to' && actions.every((a) => (g.pass_on as { actions: string[] }).actions.includes(a));

/** Everything the caller cannot give an agent, each with its reason (conformance 2.4). */
export function cannotGive(w: GrantWorld, source: Grant | null): [string, string][] {
  const mine = w.list.grants.filter((g) => g.holder === w.me.person.id && standing(w, g).ok);
  const out: [string, string][] = [];
  for (const g of mine) {
    if (g.pass_on.kind === 'use_only') out.push([`${g.relation} of ${onText(g)}`, `You may use it; ${grantNo(g.id)} does not let you pass it on.`]);
    else if (!g.pass_on.recipients.includes('agent')) out.push([`${g.relation} of ${onText(g)}`, `People only, never agents: ${grantNo(g.id)} passes on to people.`]);
  }
  if (source) {
    for (const [relation, actions] of relationsSeen(w)) {
      if (!withinPassOn(source, actions)) out.push([`${relation} of ${onText(source)}`, 'More than you hold.']);
    }
  }
  out.push(['Your sign-in identities', 'They prove who you are. No agent can hold them.']);
  return out;
}
