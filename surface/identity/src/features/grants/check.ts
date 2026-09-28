import { Refused, api } from '../../api';
import { PAGE_MAX, resourceText } from '../../generated/grants';
import type { Permit, ResourceRef } from '../../generated/grants';
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
  let after: string | null = null;
  let revision = 0;
  for (let page = 0; page < 1000; page += 1) {
    const answer = await api.who({ route: 'browser', resource, action, page_size: PAGE_MAX, after });
    revision = answer.revision;
    const found = answer.holders.find((h) => h.holder === who);
    if (found) return { ...base, ok: true, permit: found };
    if (answer.complete || !answer.next) break;
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

/** For each resource the caller can see, which of its actions each holder can exercise, from /grants/who. */
export async function reachMap(resources: { resource: ResourceRef; actions: string[] }[]): Promise<Map<string, Map<string, string[]>>> {
  const out = new Map<string, Map<string, string[]>>();
  for (const { resource, actions } of resources) {
    const byHolder = new Map<string, string[]>();
    for (const action of actions) {
      for (const h of await whoAll(resource, action)) byHolder.set(h.holder, [...(byHolder.get(h.holder) ?? []), action]);
    }
    out.set(resourceText(resource), byHolder);
  }
  return out;
}
