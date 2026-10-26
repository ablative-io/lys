/**
 * What waits on the signed-in person, as `GET /waiting` answers it: one list, oldest first, of the requests they may
 * decide, their agents' drafts, the app actions waiting on their approval and the access due their check. A part the
 * service could not read stands in the list as its own line, never as nothing. The rail's count and the Dashboard's Waiting
 * on you both read this one answer, so they can never disagree.
 */
import { request } from '../../api';
import type { WaitingEntryView } from '../../generated';
import { counted } from '../../shell/count';
import { subscribeChanges } from '../../live';

export type WaitingKind = WaitingEntryView['kind'];

/**
 * One thing the person may decide. A request entry carries the request as `GET /requests` answers it and an agent draft
 * the draft as `GET /drafts` answers it. An app action carries its product draft; a check row carries one agent's
 * due grants. Every complete row is decided from this one read.
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

export const WAITING_HOME = '#/?section=waiting-on-you';

/** Focus the section where it stands today; mounting Home also honours the same address. */
export function focusWaiting(): void {
  const section = document.getElementById('waiting-on-you');
  section?.focus();
  section?.scrollIntoView?.({ block: 'start' });
}

/** The page a line opens and the word on its button, always Open: a request opens itself on Requests. */
export function opens(line: WaitingEntry): { href: string; word: string } {
  if (line.kind === 'request') return { href: PLACE.request + '/' + encodeURIComponent(line.id), word: 'Open' };
  return { href: PLACE[line.kind], word: 'Open' };
}

/** Read `GET /waiting`, refusing an answer that is not a list, so nothing missing is shown as nothing. */
export function readWaiting(): Promise<WaitingLine[]> {
  if (!reading) {
    reading = request<{ waiting?: unknown }>('/waiting').then((answer) => {
      if (!answer || !Array.isArray(answer.waiting)) throw new Error('WaitingUnreadable: the service did not answer a list of what waits on you.');
      return answer.waiting as WaitingLine[];
    });
  }
  return reading;
}

// Home and the rail share this answer until the service signals a change.
// The last observer leaving releases it, so no sign-in inherits another's list.
let reading: Promise<WaitingLine[]> | null = null;
type Observer = { changed: (lines: WaitingLine[]) => void; refused: (error: unknown) => void };
const observers = new Set<Observer>();
let stop: (() => void) | null = null;

function publish(read: Promise<WaitingLine[]>): void {
  void read.then((lines) => {
    if (reading === read) for (const observer of observers) observer.changed(lines);
  }, (error: unknown) => {
    if (reading === read) for (const observer of observers) observer.refused(error);
  });
}

/** Observe the one waiting answer and invalidate it only on a service change signal. */
export function watchWaiting(changed: Observer['changed'], refused: Observer['refused']): () => void {
  const observer = { changed, refused };
  observers.add(observer);
  if (observers.size === 1) {
    let known = false;
    stop = subscribeChanges(() => {
      if (known) reading = null;
      known = true;
      publish(readWaiting());
    }, (error) => {
      reading = null;
      for (const each of observers) each.refused(error);
    });
  }
  publish(readWaiting());
  return () => {
    observers.delete(observer);
    if (!observers.size) { stop?.(); stop = null; reading = null; }
  };
}

/** A confirmed in-row decision updates both counts before Home reads again. */
export function waitingSettled(kind: WaitingKind, id: string): void {
  if (!reading) return;
  reading = reading.then((lines) => lines.filter((line) => isRefusedPart(line) || line.kind !== kind || line.id !== id));
  publish(reading);
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
