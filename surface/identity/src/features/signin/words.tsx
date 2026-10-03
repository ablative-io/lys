/** Failures explain the next act; identifiers remain available only in details. */
import { Refused } from '../../api';
import type { RoleChange } from '../roles/useRoleChange';

const REFUSALS: Record<string, string> = {
  NoPerson: 'Your sign-in account has not been connected to a person in Lys.',
  NotAdmitted: 'Your account does not have permission to do this.',
  SecretsUnavailable: 'Lys could not reach or use its secret storage service.',
  SignInProvidersUnavailable: 'Lys could not read or change its sign-in services.',
  SignInProvidersRefused: 'The sign-in service did not accept these settings.',
  ProviderRefused: 'The provider did not accept these sign-in settings.',
  HandleUnknown: 'That permission to use a secret was not found in the records you can see.',
  RequestHeld: 'An earlier approval still has no confirmed result.',
  LeaseExhausted: 'The permitted number of uses has been reached.',
  app_exists: 'An app is already registered with that short name.',
  schema_version_moved: 'Someone saved a newer version of these permissions.',
  schema_change_strands_grants: 'These permissions are still in use and cannot be removed yet.',
};

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
    ? REFUSALS[error.refusal.refusal] ?? plainReason(error.refusal.reason, hidden)
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

/** Checking a pending change sends its original request, with no new request identity. */
export function ChangeResult({ change }: { change: RoleChange }) {
  return <>{change.pending ? <div role="status"><p>This change has no confirmed answer. Its original details are retained.</p>
    <button className="btn" type="button" disabled={change.busy} onClick={change.retry}>Check original change</button></div> : null}
    {change.failure ? <p role="alert" className="why-not">{failureWords(change.failure, 'Check the details. If the result is unconfirmed, choose Check original change; do not create another change.')}</p> : null}</>;
}
