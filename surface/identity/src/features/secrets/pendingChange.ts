/** A retained change is retried with its original operation; a record that cannot be read holds the form. */
import { Refused } from '../../api';
import { answeredNo } from '../../kept';

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

/** Whether a failure is the service's definite answer, which settles the change. */
export const definitelyRefused = (refused: Refused): boolean => answeredNo(refused);

/** A 2xx answer that does not say what was asked: the outcome is unknown. */
export const unconfirmed = (reason: string): Refused =>
  new Refused(200, { refusal: 'UnconfirmedAnswer', reason });
