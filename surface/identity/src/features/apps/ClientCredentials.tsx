/**
 * An approved app's client credentials (DIRECTORY-081): the administrator issues one here and gives it to the app as
 * its client secret. Its value is shown once, with a way to copy it, and is gone when this page is left: Lys keeps no
 * copy, only the means to confirm it. Every credential the app holds is listed with who issued it and when, and who
 * revoked it and why; revoking asks first, and the app's next sign-in with it is refused. Approval's own custody of the
 * app's secrets is confirmed here too, by the names Lys secrets keeps them under, never their values.
 */
import { useState } from 'react';
import { operationId } from '../../api';
import { Act } from '../../shell/Act';
import { Copy } from '../network/JoinCode';
import { clock } from '../file/time';
import { send } from './SchemaBuilder';
import { refusalWords, who } from './Apps';
import type { AppRecord, By } from './Apps';

/** One of an app's client credentials as Lys lists it. It never carries a value. */
export interface CredentialRecord {
  credential_id: string; issued_by: By; issued_at: number;
  revoked_by: By | null; revoked_at: number | null; revoked_reason: string | null;
  ended_by_retirement: boolean; live: boolean; ended_at_broker: boolean;
}
/** The references approval answers once Lys secrets has custody of the app's secrets. */
export interface StoredCredentials { app: string; client_secret_ref: string; api_credential_ref: string }
/** A credential just issued: the one answer that carries its value. */
interface Given { credential_id: string; credential: string }

const isBy = (value: unknown): value is By => !!value && typeof value === 'object' && typeof (value as By).kind === 'string';
const orNull = <T,>(value: unknown, is: (value: unknown) => value is T): boolean => value === null || is(value);
const isNumber = (value: unknown): value is number => Number.isSafeInteger(value);
const isString = (value: unknown): value is string => typeof value === 'string';

/** Whether `value` is a credential record as Lys lists one. */
export function credentialRecord(value: unknown): value is CredentialRecord {
  const record = value as Partial<CredentialRecord> | null;
  return !!record && typeof record.credential_id === 'string' && isBy(record.issued_by) && isNumber(record.issued_at)
    && orNull(record.revoked_by, isBy) && orNull(record.revoked_at, isNumber) && orNull(record.revoked_reason, isString)
    && typeof record.ended_by_retirement === 'boolean' && typeof record.live === 'boolean' && typeof record.ended_at_broker === 'boolean';
}

function given(value: unknown, app: string): Given {
  const answer = value as { app?: unknown; credential_id?: unknown; credential?: unknown } | null;
  if (!answer || answer.app !== app || typeof answer.credential_id !== 'string' || typeof answer.credential !== 'string'
    || !answer.credential.startsWith('lys-client.' + app + '.')) {
    throw new Error('Lys answered, but did not give a credential for ' + app + '. Look at its credentials below before issuing another.');
  }
  return { credential_id: answer.credential_id, credential: answer.credential };
}

function listed(value: unknown, app: string): CredentialRecord[] {
  const answer = value as { id?: unknown; client_credentials?: unknown } | null;
  if (!answer || answer.id !== app || !Array.isArray(answer.client_credentials) || !answer.client_credentials.every(credentialRecord)) {
    throw new Error('Lys answered, but did not list ' + app + '’s credentials. Open Apps again to see them.');
  }
  return answer.client_credentials;
}

/** Where a credential stands, in words. */
function standing(credential: CredentialRecord): string {
  if (credential.live) return 'In use: the app signs people in with it.';
  const ended = credential.revoked_by && credential.revoked_at !== null
    ? 'Revoked by ' + who(credential.revoked_by) + ', ' + clock(credential.revoked_at) + (credential.revoked_reason ? ': ' + credential.revoked_reason : '') + '.'
    : 'Ended when the app was retired.';
  return credential.ended_at_broker ? ended : ended + ' Lys refuses it; Lys secrets is told to forget it at the next change made here.';
}

/** Approval's custody, confirmed by the names Lys secrets keeps the app's two secrets under. */
export function CustodyConfirmed({ answer }: { answer: StoredCredentials }) {
  return <section role="status"><p>Lys secrets holds this app’s sign-in secret and its key for calling Lys, owned by you. Nobody is shown either; the app is given a client credential issued below.</p>
    <p className="note">Their names in Lys secrets: sign-in secret {answer.client_secret_ref}; key for calling Lys {answer.api_credential_ref}.</p></section>;
}

/** An approved app's client credentials: issue one, shown once; the list; revoke one, after asking. A retired app's
 *  are listed only: its retirement ended them all. */
export function ClientCredentials({ app }: { app: AppRecord }) {
  const open = app.state === 'approved';
  const [held, setHeld] = useState<CredentialRecord[]>(app.client_credentials);
  const [shown, setShown] = useState<Given | null>(null);
  const [asking, setAsking] = useState<string | null>(null);
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState('');
  const path = (rest: string) => '/apps/' + encodeURIComponent(app.id) + rest;
  const run = async (act: () => Promise<void>) => {
    if (busy) return;
    setBusy(true); setRefusal('');
    try { await act(); } catch (error) { setRefusal(refusalWords(error)); }
    setBusy(false);
  };
  const issue = () => run(async () => {
    const answer = given(await send('POST', path('/credentials/issue'), { operation: operationId() }), app.id);
    setShown(answer);
    setHeld(listed(await send('GET', path('')), app.id));
  });
  const revoke = (credential: string) => run(async () => {
    setHeld(listed(await send('POST', path('/credentials/' + encodeURIComponent(credential) + '/revoke'), { operation: operationId(), reason: reason.trim() }), app.id));
    setAsking(null); setReason('');
  });
  return <section className="app-credentials" aria-label={'Client credentials of ' + app.id}>
    <b>Client credentials</b>
    {open ? <p className="note">{app.name} signs people in with a client credential you give it as its client secret. Each is shown once, when issued.</p> : null}
    {shown ? <div className="app-secret" role="group" aria-label={'Credential ' + shown.credential_id + ' of ' + app.id}>
      <p>Give this to {app.name} as its client secret. It is shown once, here, and Lys keeps no copy: if it is lost, issue another and revoke this one.</p>
      <p><code data-credential="value">{shown.credential}</code> <Copy text={shown.credential} what="The credential" /></p>
      <Act symbol="approve" name="Done with the credential" word="Done" onClick={() => setShown(null)} />
    </div> : null}
    {held.length ? <ul className="app-list">{held.map((credential) => <li key={credential.credential_id} data-credential={credential.credential_id}>
      <span className="mono">{credential.credential_id}</span> issued by {who(credential.issued_by)}, {clock(credential.issued_at)}. {standing(credential)}
      {open && credential.live && asking !== credential.credential_id ? <> <Act symbol="revoke" name={'Revoke credential ' + credential.credential_id} word="Revoke" tone="danger" disabled={busy} onClick={() => { setAsking(credential.credential_id); setReason(''); }} /></> : null}
      {asking === credential.credential_id ? <div className="app-actions" role="group" aria-label={'Revoke credential ' + credential.credential_id + '?'}>
        <p>Revoke {credential.credential_id}? {app.name} is refused the next time it signs someone in with it. This cannot be undone.</p>
        <label className="field">Why<input value={reason} onChange={(event) => setReason(event.target.value)} /></label>
        <Act symbol="revoke" name={'Revoke credential ' + credential.credential_id + ' now'} word="Revoke" tone="danger" disabled={busy} onClick={() => void revoke(credential.credential_id)} />
        <Act symbol="close" name="Keep the credential" word="Keep" disabled={busy} onClick={() => setAsking(null)} />
      </div> : null}
    </li>)}</ul> : <p className="dim">{open ? 'No client credential has been issued, so ' + app.name + ' cannot sign anyone in yet.' : 'No client credential was issued.'}</p>}
    {open ? <Act symbol="add" name={'Issue a client credential for ' + app.name} word="Issue" tone="primary" disabled={busy} onClick={() => void issue()} /> : null}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
  </section>;
}
