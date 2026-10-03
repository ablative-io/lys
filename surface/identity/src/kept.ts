/**
 * The one rule for a change whose outcome must be known. It is written down in this tab before it is sent, and it
 * is released only when its outcome is known: the answer confirmed it, or the service answered no and the caller
 * says a no settles it. Anything else (no answer, an answer that does not confirm, a fault) may have been carried
 * out, so the request stays kept and is sent again as it was.
 */
import { Refused } from './api';

/** The service's definite answer of no. */
export const answeredNo = (error: unknown): boolean => error instanceof Refused && error.status >= 400 && error.status < 500;

/**
 * Keep `record` under `key`, then `send`. `send` confirms its own answer and throws when it does not confirm.
 * `noSettles` is whether a definite no ends the request: true for a first sending, false for a request sent again
 * after an unknown outcome, where a later no cannot show the first sending was not carried out.
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
