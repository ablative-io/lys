/**
 * What waits on the signed-in person, as `GET /waiting` answers it: one list, oldest first, of the requests they may
 * decide, their agents' drafts, the app actions waiting on their approval and the access due their check. A part the
 * service could not read stands in the list as its own line, never as nothing. The rail's count and the Dashboard's Waiting
 * on you both read this one answer, so they can never disagree.
 */
import { request } from '../../api';
import type { WaitingEntryView } from '../../generated';
import { counted } from '../../shell/count';

export type WaitingKind = WaitingEntryView['kind'];

/**
 * One thing the person may decide. A request entry carries the request as `GET /requests` answers it and an agent draft
 * the draft as `GET /drafts` answers it, so the row decides it from this one read; an app action and a check due carry
 * neither and are opened on their own pages.
 */
export type WaitingEntry = WaitingEntryView;

/** A part the service could not read for the person, by its refusal. */
export interface WaitingPartRefused { kind: WaitingKind; refusal: string; reason: string }

export type WaitingLine = WaitingEntry | WaitingPartRefused;

export const isRefusedPart = (line: WaitingLine): line is WaitingPartRefused => typeof (line as WaitingPartRefused).refusal === 'string';

/** The words for a part that could not be read, where its lines would stand. */
export const UNREAD: Record<WaitingKind, string> = {
  request: 'Requests could not be read',
  agent_draft: 'Your agents’ drafts could not be read',
  product_draft: 'Whether an app action waits for your approval could not be read',
  review: 'Whether the access you gave is due a check could not be read',
};

/** The page each part stands for. */
export const PLACE: Record<WaitingKind, string> = { request: '#/requests', agent_draft: '#/access/drafts', product_draft: '#/access/drafts', review: '#/reviews' };

/** The page a line opens and the word on its button, always Open: a request opens itself on Requests. */
export function opens(line: WaitingEntry): { href: string; word: string } {
  if (line.kind === 'request') return { href: PLACE.request + '/' + encodeURIComponent(line.id), word: 'Open' };
  return { href: PLACE[line.kind], word: 'Open' };
}

/** Read `GET /waiting`, refusing an answer that is not a list, so nothing missing is shown as nothing. */
export async function readWaiting(): Promise<WaitingLine[]> {
  const answer = await request<{ waiting?: unknown }>('/waiting');
  if (!answer || !Array.isArray(answer.waiting)) throw new Error('WaitingUnreadable: the service did not answer a list of what waits on you.');
  return answer.waiting as WaitingLine[];
}

/** How many lines the person can act on; null when a part could not be read, so no number stands for it. */
export function waitingCount(lines: WaitingLine[]): number | null {
  return lines.some(isRefusedPart) ? null : lines.length;
}

const things = (n: number): string => counted(n, 'things wait on you', 'thing waits on you');

/** The rail's count: the number of lines, or, when a part could not be read, which part and how many wait besides. */
export function railWaiting(lines: WaitingLine[]): { count: number; words: string } | { problem: string } {
  const refused = lines.filter(isRefusedPart);
  const entries = lines.length - refused.length;
  if (refused.length) return { problem: refused.map((part) => UNREAD[part.kind]).join('; ') + (entries ? '; ' + things(entries) + ' besides' : '') };
  return { count: entries, words: entries ? things(entries) : 'Nothing is waiting on you' };
}
