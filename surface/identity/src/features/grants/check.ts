import { Refused, api } from '../../api';
import { PAGE_MAX, resourceText } from '../../generated/grants';
import type { GrantMode, Permit, ResourceRef } from '../../generated/grants';
import { clock } from '../file/time';

export type Answer =
  | { ok: true; who: string; action: string; resource: ResourceRef; permit: Permit; t: string }
  | { ok: false; who: string; action: string; resource: ResourceRef; kind: string; why: string; t: string; revision: number | null; open: boolean };

const checked = () => clock(Math.floor(Date.now() / 1000)).split(' ').at(-1) ?? '';

/**
 * Can `who` exercise `action` on `resource`? The caller's own question goes
 * to /grants/why, which names its refusal. Anyone else's is read from the
 * same evaluator's /grants/who answer, every page of it, so a yes carries
 * its path and a no says it was not found among who can.
 */
export async function ask(caller: string, who: string, whoName: string, resource: ResourceRef, action: string): Promise<Answer> {
  const base = { who, action, resource, t: checked() };
  if (who === caller) {
    try {
      const permit = await api.why({ route: 'browser', resource, action });
      return { ...base, ok: true, permit };
    } catch (error) {
      if (!(error instanceof Refused)) throw error;
      const { refusal, reason } = error.refusal;
      const open = refusal === 'IdentityNotActive' && / is suspended,/.test(reason);
      return { ...base, ok: false, kind: refusal, why: reason.replace(/^[A-Za-z]+: /, ''), revision: null, open };
    }
  }
  // Every page is read, at one revision, before the answer is No. A read that did not finish is refused, never told as No.
  let after: string | null = null;
  let revision: number | null = null;
  const read = new Set<string>();
  for (;;) {
    const answer = await api.who({ route: 'browser', resource, action, page_size: PAGE_MAX, after });
    if (revision !== null && answer.revision !== revision) {
      throw new Refused(409, { refusal: 'GrantRevisionChanged', reason: `Permissions changed while reading ${resourceText(resource)}; refresh to read them again.` });
    }
    revision = answer.revision;
    const found = answer.holders.find((h) => h.holder === who);
    if (found) return { ...base, ok: true, permit: found };
    if (answer.complete) break;
    if (!answer.next || read.has(answer.next)) {
      throw new Refused(502, { refusal: 'PermissionPageIncomplete', reason: `The permission service did not advance its page for ${resourceText(resource)} (${action}).` });
    }
    read.add(answer.next);
    after = answer.next;
  }
  return {
    ...base,
    ok: false,
    kind: 'no grant',
    why: `${whoName} holds no grant you may see that gives ${action} on ${resourceText(resource)}`,
    revision,
    open: false,
  };
}

/** Everyone who can exercise `action` on `resource`, every page, each with its permit. */
export async function whoAll(resource: ResourceRef, action: string): Promise<(Permit & { holder: string })[]> {
  const out: (Permit & { holder: string })[] = [];
  let after: string | null = null;
  const cursors = new Set<string>();
  let revision: number | null = null;
  while (true) {
    const answer = await api.who({ route: 'browser', resource, action, page_size: PAGE_MAX, after });
    if (revision !== null && answer.revision !== revision) {
      throw new Refused(409, { refusal: 'GrantRevisionChanged', reason: `Permissions changed while reading ${resourceText(resource)}; refresh to read them again.` });
    }
    revision = answer.revision;
    out.push(...answer.holders);
    if (answer.complete) break;
    if (!answer.next || cursors.has(answer.next)) {
      throw new Refused(502, { refusal: 'PermissionPageIncomplete', reason: `The permission service did not advance its page for ${resourceText(resource)} (${action}).` });
    }
    cursors.add(answer.next);
    after = answer.next;
  }
  return out;
}

/**
 * For each resource the caller can see, which of its actions each holder can
 * exercise, from /grants/reach: every resource in one question, answered at
 * one revision. An answer that leaves out a resource asked about is refused
 * rather than drawn as a partial graph.
 */
export async function reachMap(resources: { resource: ResourceRef; actions: string[] }[]): Promise<Map<string, Map<string, string[]>>> {
  const answered = await reachAnswered(resources);
  return new Map([...answered].map(([res, byHolder]) => [res, new Map([...byHolder].map(([holder, held]) => [holder, held.actions]))]));
}

/** One holder's reach on one resource: the actions, and beside each the mode of the grant it rests on. */
export interface Reached { actions: string[]; modes: GrantMode[] }

/**
 * The reach of every resource asked about, by resource then holder, with the
 * resources the service named restricted (ACCESS-004 R2): placed in a parent
 * whose relations do not flow to them, so only a grant on the place reaches it.
 */
export type ReachedMap = Map<string, Map<string, Reached>> & { restricted: Set<string> };

/**
 * {@link reachMap} with each action's mode, as the service answers it beside
 * the action. An answer whose modes do not stand one beside each action is
 * refused, never padded.
 */
export async function reachWithModes(resources: { resource: ResourceRef; actions: string[] }[]): Promise<ReachedMap> {
  const answered = await reachAnswered(resources);
  for (const [res, byHolder] of answered) {
    for (const [holder, held] of byHolder) {
      if (!Array.isArray(held.modes) || held.modes.length !== held.actions.length) {
        throw new Refused(502, { refusal: 'PermissionAnswerIncomplete', reason: `The permission service did not answer the mode of each action ${holder} may take on ${res}.` });
      }
    }
  }
  return answered;
}

async function reachAnswered(resources: { resource: ResourceRef; actions: string[] }[]): Promise<ReachedMap> {
  const out: ReachedMap = Object.assign(new Map<string, Map<string, Reached>>(), { restricted: new Set<string>() });
  if (!resources.length) return out;
  const answer = await api.reach({ route: 'browser', resources: resources.map(({ resource, actions }) => ({ ...resource, actions })) });
  resources.forEach(({ resource }, n) => {
    const answered = answer.resources[n];
    if (!answered || answered.kind !== resource.kind || answered.id !== resource.id) {
      throw new Refused(502, { refusal: 'PermissionAnswerIncomplete', reason: `The permission service did not answer for ${resourceText(resource)}.` });
    }
    out.set(resourceText(resource), new Map(answered.holders.map(({ holder, actions, modes }) => [holder, { actions, modes }])));
    if (answered.restricted === true) out.restricted.add(resourceText(resource));
  });
  return out;
}
