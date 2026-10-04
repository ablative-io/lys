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

/**
 * Who `id` is, said so that two of the same name can be told apart: the name, whose agent or account it is, and the
 * file number. Names are not unique, so every place that shows a bare name carries this as its title.
 */
export const whoTitle = (w: GrantWorld, id: string): string => {
  const who = w.who.get(id);
  if (!who) return fileNo(id);
  const of = who.responsible ? w.who.get(who.responsible)?.name : undefined;
  const whose = of ? (who.kind === 'agent' ? ', agent of ' : ', account of ') + of : '';
  // The letter is the record's own kind, not a guess from the id's shape.
  const letter = who.kind === 'agent' ? 'A/' : who.kind === 'person' ? 'P/' : 'S/';
  return who.name + whose + ', ' + letter + id.slice(id.indexOf('-') + 1, id.indexOf('-') + 9);
};

/** A grant's short name, as the mock-up writes `G-5`. */
export const grantNo = (id: string): string => 'G/' + id.slice(id.indexOf('-') + 1, id.indexOf('-') + 9);

export const onText = (g: Grant): string => resourceText(g.resource);

/** A grant's resource as a name: a directory collection in plain words (`the agents directory`), anything else as its reference. */
export const sourceText = (g: Grant): string => (g.resource.kind === 'directory' ? resourceWords(g.resource) : onText(g));

/** What follows the actions given on a grant's resource: `on the agents directory` for a directory collection, the reference otherwise. */
export const givenOnText = (g: Grant): string => (g.resource.kind === 'directory' ? 'on ' + resourceWords(g.resource) : onText(g));

/** A kind as a person says it: `service account`, never `service_account`. */
const kindWords = (kind: string): string => kind.replace(/_/g, ' ');

/**
 * An identity's or a grant's raw identifier, which a person never reads as a label. A service account's id begins
 * `op-` (crates/lys-identity/src/id.rs), as an operation's, a team's and a computer's do, so an `op-` id is read as
 * a name only when the directory answered a service account under it.
 */
const rawId = /\b(?:person|agent|grant|op)-[0-9a-f]{8,}\b/g;

/**
 * A resource as a person reads it: a directory collection in plain words, an
 * identity by its name (never its raw id), anything else as its kind and id.
 */
export const resourceName = (w: GrantWorld, r: ResourceRef): string => {
  if (r.kind === 'directory') return resourceWords(r);
  const who = w.who.get(r.id);
  if (who) return `${kindWords(r.kind)} ${who.name}`;
  return `${kindWords(r.kind)} ${named(w, r.id)}`;
};

/**
 * A sentence from the service with every raw identifier in it put as a person
 * reads it: an identity by its name, a grant by its short name.
 */
export function named(w: GrantWorld, text: string): string {
  return text.replace(rawId, (id) => (id.toLowerCase().startsWith('grant-') ? grantNo(id) : id.startsWith('op-') && !w.who.has(id) ? id : nameOf(w, id)));
}

/**
 * What `g` lets its holder do: the actions the grant itself carries, which are
 * the actions the service checks when it is exercised, widest first as the model
 * carries them. Only the actions: the resource is the grant's On, and is said there.
 */
export function mayText(w: GrantWorld, g: Grant): string {
  if (!g.actions.length) return 'No action';
  const listed = actionWords(w.model, g.resource, g.actions);
  return listed === 'everything here' ? 'Everything here' : listed;
}

/**
 * A resource as the pickers name it: a project or organisation by its id,
 * a directory collection in plain words (`the agents directory`), an identity by
 * its name, anything else as `name (type)`. Never a raw identity id.
 */
export const resourceLabel = (r: ResourceRef, w?: GrantWorld): string => {
  if (r.kind === 'project' || r.kind === 'organisation') return resourceText(r);
  if (r.kind === 'directory') return resourceWords(r);
  const who = w?.who.get(r.id);
  return `${who ? who.name : w ? named(w, r.id) : r.id} (${kindWords(r.kind)})`;
};

/** A resource's full reference for a cell's title: its kind and its name where it is an identity. */
export const resourceTitle = (w: GrantWorld, r: ResourceRef): string => (w.who.has(r.id) ? `${kindWords(r.kind)} ${whoTitle(w, r.id)}` : r.id.match(rawId) ? resourceName(w, r) : resourceText(r));

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
 * Why `g` does not stand, in the service's words with names for its raw ids, or null when the service
 * answers that it stands. Nothing here walks a chain, reads a clock or decides
 * standing from a holder's state: the service's standing is the only answer.
 */
export function voidOf(w: GrantWorld, g: Grant): Void | null {
  const s = g.standing;
  if (s.stands) return null;
  const upstream = s.grant !== null && s.grant !== g.id;
  const why = upstream ? `it derives from ${grantNo(s.grant ?? '')}, which no longer stands (${named(w, s.reason)})` : named(w, s.reason);
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

/** A grant's window as its effective end stands: `27 Sep to 4 Oct`, or `from 27 Sep, no end`. */
export const windowText = (g: Grant): string => (g.effective_ends_at === null ? `from ${day(g.window.starts_at)}, no end` : `${day(g.window.starts_at)} to ${day(g.effective_ends_at)}`);

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
