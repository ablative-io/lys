import { Refused, request } from '../../api';
import { CANNOT_GIVE_REASONS } from '../../generated';
import type { CannotGiveAnswer, CannotGiveQuery, CannotGiveReason } from '../../generated';

/** The refusal of an answer that carries a reason outside the six. */
export const UNKNOWN_CANNOT_GIVE_REASON = 'unknown_cannot_give_reason';

const SUBJECTS = ['grant', 'service_account', 'relation', 'sign_in_identity'];

const isReason = (value: unknown): value is CannotGiveReason =>
  typeof value === 'string' && (CANNOT_GIVE_REASONS as readonly string[]).includes(value);

const unreadable = (reason: string) =>
  new Refused(200, { refusal: 'UnreadableResponse', reason: 'the cannot-give answer could not be read: ' + reason });

/**
 * The cannot-give answer as the server's contract types it (conformance 2.4).
 * An item whose reason is not one of the six refuses the whole answer by
 * name, so no item of it is shown and no reason is left blank; nothing is
 * defaulted, and nothing is added, dropped or reordered.
 */
export function readCannotGive(raw: unknown): CannotGiveAnswer {
  if (typeof raw !== 'object' || raw === null) throw unreadable('it is not an object');
  const answer = raw as Record<string, unknown>;
  if (typeof answer.source !== 'string' || typeof answer.recipient !== 'string' || !Array.isArray(answer.items)) {
    throw unreadable('it does not name its source, its recipient and its items');
  }
  for (const item of answer.items as unknown[]) {
    const reason = (item as { reason?: unknown } | null)?.reason;
    if (!isReason(reason)) {
      throw new Refused(200, {
        refusal: UNKNOWN_CANNOT_GIVE_REASON,
        reason: `the service answered the cannot-give reason ${JSON.stringify(reason)}, which is not one of ${CANNOT_GIVE_REASONS.join(', ')}, so the whole answer is refused`,
      });
    }
    const subject = (item as { subject?: unknown }).subject;
    if (typeof subject !== 'string' || !SUBJECTS.includes(subject)) throw unreadable(`an item's subject is ${JSON.stringify(subject)}`);
  }
  return raw as CannotGiveAnswer;
}

/** The query GET /grants/cannot-give is asked with, from the browser. */
export function cannotGivePath(source: string, recipient: string): string {
  const query: CannotGiveQuery = { route: 'browser', source, recipient };
  return '/grants/cannot-give?' + (['route', 'source', 'recipient'] as const).map((k) => k + '=' + encodeURIComponent(query[k])).join('&');
}

/** Ask the service what the caller cannot give `recipient` from the grant `source`. */
export async function askCannotGive(source: string, recipient: string): Promise<CannotGiveAnswer> {
  return readCannotGive(await request<unknown>(cannotGivePath(source, recipient)));
}
