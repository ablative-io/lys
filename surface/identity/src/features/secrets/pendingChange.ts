/**
 * An owner change to a secret is recorded in the tab before it is sent and
 * stays recorded until the service answers it definitely: a confirmed answer
 * that matches what was asked, or a refusal in the 4xx range. Anything else
 * (no answer, a 5xx, a 2xx that cannot be read or does not match) leaves the
 * change pending, and a pending change is never sent again from this tab.
 */
import { Refused } from '../../api';

/** The part of Storage a pending change needs; sessionStorage in the browser. */
export interface PendingStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/** The storage key of one kind of change to one secret. */
export const pendingKey = (change: 'scope' | 'recipients', secret: string): string =>
  `lys.pending.secrets.${change}.${secret}`;

/**
 * What is pending under a key: the change asked, as recorded, or a sentence
 * saying the record is damaged. A damaged record cannot show a resend is safe,
 * so it holds the form exactly as a readable one does.
 */
export function pendingAt(store: PendingStore, key: string): string | null {
  const saved = store.getItem(key);
  if (saved === null) return null;
  try {
    const record: unknown = JSON.parse(saved);
    if (record && typeof record === 'object' && 'asked' in record && typeof record.asked === 'string') return record.asked;
  } catch {
    // Named below: the record is damaged.
  }
  return `the pending record could not be read; inspect ${key}`;
}

/** Record the change before it is sent. A store that refuses the write refuses the send. */
export function recordPending(store: PendingStore, key: string, asked: string): void {
  store.setItem(key, JSON.stringify({ asked }));
}

/** Whether a failure is the service's definite answer, which settles the change. */
export const definitelyRefused = (refused: Refused): boolean => refused.status >= 400 && refused.status < 500;

/** A 2xx answer that does not say what was asked: the outcome is unknown. */
export const unconfirmed = (reason: string): Refused =>
  new Refused(200, { refusal: 'UnconfirmedAnswer', reason });
