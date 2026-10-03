/**
 * The one rule for a change whose outcome must be known. It is written down in this tab before it is sent, and it
 * is released only when its outcome is known: the answer confirmed it, or the service answered no and the caller
 * says a no settles it. Anything else (no answer, an answer that does not confirm, a fault) may have been carried
 * out, so the request stays kept and is sent again as it was.
 *
 * A walk of several requests (adding and running an agent, a queue of grants or asks, a computer and its runner)
 * keeps its own place between steps; each uses `answeredNo` here for what a definite no is.
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
  sessionStorage.setItem(key, JSON.stringify(record));
  try {
    const answer = await send();
    sessionStorage.removeItem(key);
    return answer;
  } catch (error) {
    if (noSettles && answeredNo(error)) sessionStorage.removeItem(key);
    throw error;
  }
}
