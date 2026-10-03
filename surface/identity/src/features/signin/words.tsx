/** Failures explain the next act; identifiers remain available only in details. */
import { Refused } from '../../api';
import { refusalWords } from '../people/Words';

/** Keep an unrecognised reason readable without repeating codes or raw identities. */
export function plainReason(reason: string, hidden: string[] = []): string {
  let words = reason;
  for (const value of hidden.filter(Boolean)) words = words.split(value).join('[secret withheld]');
  return words
    .replace(/\b(person|agent|grant|op)-[0-9a-f]{32}\b/g, (_identifier, kind: string) => ({ person: 'that person', agent: 'that agent', grant: 'that permission', op: 'that request' })[kind] ?? 'that record')
    .replace(/\b[A-Z][A-Za-z]+:\s*/g, '')
    .replace(/\bprincipal\b/gi, 'person or agent')
    .replace(/\boperation(?:s)?(?: id(?:s)?)?\b/gi, 'request')
    .replace(/\bscope\b/gi, 'access')
    .replace(/\bhandle(?:s)?\b/gi, 'permission to use a secret')
    .replace(/\bJSON\b/g, 'data');
}

/** A definite refusal or an unconfirmed answer, followed by the screen's next step. */
export function failureWords(error: unknown, next: string, hidden: string[] = []): string {
  const reason = error instanceof Refused
    ? refusalWords(error.refusal.refusal) ?? plainReason(error.refusal.reason, hidden)
    : plainReason(error instanceof Error ? error.message : String(error), hidden);
  return reason + ' ' + next;
}

/** Read failures do not imply that an earlier write failed. */
export function ReadFailure({ error, subject, administrator = false }: { error: Refused; subject: string; administrator?: boolean }) {
  if (error.refusal.refusal === 'SecretsUnavailable' && error.refusal.reason.includes('no secrets broker is configured')) {
    return <div className="why-not" role="alert"><p>Lys has no secrets store set up.</p><p>{administrator
      ? 'Set up secret storage in the service configuration and restart Lys.'
      : 'Ask your administrator to set up secret storage for Lys.'}</p></div>;
  }
  return <div className="why-not" role="alert"><p>Lys could not show {subject}.</p>
    <p>{failureWords(error, error.status === 403 || error.refusal.refusal === 'NoPerson'
      ? (administrator ? 'Check that this account has permission for this action.' : 'Ask your administrator to check your access.')
      : (administrator ? 'Check the service configuration and its reported errors. Keep any change whose result is unconfirmed; do not send it as a new change.' : 'Ask your administrator to check this service. Keep any change whose result is unconfirmed; do not send it as a new change.'))}</p>
  </div>;
}

export { IdentityName } from '../people/Words';
