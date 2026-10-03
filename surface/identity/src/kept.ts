/**
 * The one rule for a change whose outcome must be known. It is written down in this tab before it is sent, and it
 * is released only when its outcome is known: the answer confirmed it, or the service answered no and the caller
 * says a no settles it. Anything else (no answer, an answer that does not confirm, a fault) may have been carried
 * out, so the request stays kept and is sent again as it was.
 *
 * A walk of several requests (adding and running an agent, a queue of grants or asks, a computer and its runner)
 * keeps its own place between steps and sends each step through here.
 */
import { Refused } from './api';

/** The service's definite answer of no. */
export const answeredNo = (error: unknown): boolean => error instanceof Refused && error.status >= 400 && error.status < 500;

/**
 * Keep `record` under `key`, then `send`. `send` confirms its own answer and throws when it does not confirm.
 * `noSettles` is whether a definite no ends the request. It does for a first sending, and for a route that answers
 * a repeated operation with that operation's own recorded outcome (the settings save, the start), since there a no
 * says the operation was never carried out. It does not for a route that judges a second sending afresh, where a
 * later no cannot show the first sending was not carried out.
 * Failing to keep the record refuses the send.
 */
export async function sendKept<T>(key: string, record: unknown, send: () => Promise<T>, noSettles: boolean): Promise<T> {
  const kept = await keptSend(key, record, send, noSettles);
  if (kept.at !== 'confirmed') throw kept.error;
  if (kept.unreleased) throw kept.unreleased;
  return kept.answer;
}

/**
 * What became of one kept send, for a form that says each case in its own words. `unkept`: the browser would not
 * keep the record, so nothing was sent. `unknown`: no confirmed answer, the record stays kept. `confirmed` and
 * `refused` are settled; `unreleased` is then the browser's error if the kept copy could not be removed, in which
 * case checking that copy again answers the same outcome, never a second change.
 */
export type Kept<T> =
  | { at: 'confirmed'; answer: T; unreleased: Error | null }
  | { at: 'refused'; error: unknown; unreleased: Error | null }
  | { at: 'unkept'; error: unknown }
  | { at: 'unknown'; error: unknown };

export async function keptSend<T>(key: string, record: unknown, send: () => Promise<T>, noSettles: boolean): Promise<Kept<T>> {
  try { sessionStorage.setItem(key, JSON.stringify(record)); } catch (error) { return { at: 'unkept', error }; }
  let answer: T;
  try { answer = await send(); } catch (error) {
    return noSettles && answeredNo(error) ? { at: 'refused', error, unreleased: release(key) } : { at: 'unknown', error };
  }
  return { at: 'confirmed', answer, unreleased: release(key) };
}

function release(key: string): Error | null {
  try { sessionStorage.removeItem(key); return null; } catch (error) { return error instanceof Error ? error : new Error(String(error)); }
}
