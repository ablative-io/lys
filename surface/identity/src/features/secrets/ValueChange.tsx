/**
 * A secret's value changed, and the secret retired, by its owner. A new value is sent once, to the broker, and never
 * kept in this tab or shown again; a change with no answer is checked against the broker's record of its operation,
 * never sent a second time. A retirement is kept with its operation, and sent again only under that same operation,
 * which the broker answers with what it recorded the first time.
 */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, Refused } from '../../api';
import { answeredNo, keepRecord, releaseRecord } from '../../kept';
import { secretsApi } from './secretsApi';
import { Act } from '../../shell/Act';

/** A kept change of one secret: its operation, never a value. */
export interface PendingChange { version: 1; operation: string; secret: string }

export const replaceKey = (secret: string): string => 'lys.pending.secrets.replace.' + secret;
export const retireKey = (secret: string): string => 'lys.pending.secrets.retire.' + secret;

/** The change kept under `key` for `secret`, in the one shape it is written in. */
export function savedChange(key: string, secret: string): PendingChange | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  let value: unknown;
  try { value = JSON.parse(raw); } catch { value = null; }
  if (!value || typeof value !== 'object' || !('version' in value) || value.version !== 1 || !('operation' in value)
    || typeof value.operation !== 'string' || !('secret' in value) || value.secret !== secret) {
    throw new Refused(0, { refusal: 'PendingSecretUnreadable', reason: 'The kept change of ' + secret + ' cannot be read. Remove ' + key + ' from this tab before changing it again.' });
  }
  return { version: 1, operation: value.operation, secret };
}

const words = (error: unknown): string => error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error);

/** The kept change under `key`, what failed, and whether a send is in flight. */
function useKept(key: string, secret: string) {
  const [restored] = useState(() => {
    try { return { pending: savedChange(key, secret), error: '' }; }
    catch (error) { return { pending: null, error: words(error) }; }
  });
  const [pending, setPending] = useState<PendingChange | null>(restored.pending);
  const [failure, setFailure] = useState(restored.error);
  const [done, setDone] = useState('');
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const keep = (next: PendingChange) => { keepRecord(key, next); setPending(next); };
  const release = () => { releaseRecord(key); setPending(null); };
  /** Run `act` once at a time, saying what failed. */
  const once = async (act: () => Promise<void>) => {
    if (working.current) return;
    working.current = true; setBusy(true); setFailure(''); setDone('');
    try { await act(); } catch (error) { setFailure(words(error)); } finally { working.current = false; setBusy(false); }
  };
  return { pending, failure, setFailure, done, setDone, busy, keep, release, once, blocked: busy || pending !== null || Boolean(restored.error) };
}

/** A new value for `secret`, from its owner. */
export function ReplaceValue({ secret, changed }: { secret: string; changed: () => void }) {
  const kept = useKept(replaceKey(secret), secret);
  const [value, setValue] = useState('');
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (kept.blocked) return;
    if (!value) { kept.setFailure('Type the new value.'); return; }
    const sending: PendingChange = { version: 1, operation: operationId(), secret };
    try { kept.keep(sending); } catch (error) { kept.setFailure('The change could not be kept in this tab, so it was not sent: ' + String(error)); return; }
    const typed = value;
    // The value leaves the form as it is sent: it is never kept, so it is never sent again.
    setValue('');
    void kept.once(async () => {
      try {
        const answer = await secretsApi.replace(secret, typed, sending.operation);
        if (answer.secret !== secret || answer.operation !== sending.operation) {
          throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service answered, but not with this change of ' + secret + '.' });
        }
        kept.release(); changed();
        kept.setDone(secret + '’s value was changed. The new value is sealed and will not be shown again.');
      } catch (error) {
        if (answeredNo(error)) kept.release();
        throw error;
      }
    });
  };
  const check = () => void kept.once(async () => {
    if (!kept.pending) return;
    const settings = await secretsApi.settings(secret);
    kept.release();
    if (settings.last_operation === kept.pending.operation) {
      changed(); kept.setDone(secret + '’s value was changed. The new value is sealed and will not be shown again.');
    } else kept.setFailure(secret + '’s value was not changed. Type the new value again to change it.');
  });
  return <form onSubmit={submit} aria-label={'Change the value of ' + secret}>
    <h2>Change its value</h2>
    <p className="note">Only its owner can change it. The value is sent once and never shown again.</p>
    <label className="field">New value<input type="password" autoComplete="new-password" value={value} disabled={kept.blocked} onChange={(event) => setValue(event.target.value)} /></label>
    <Act symbol="save" name="Change the value" word="Change" tone="primary" type="submit" disabled={kept.blocked} />
    {kept.pending ? <div role="status"><p>This change is not confirmed. The new value was not kept, so it is not sent again.</p>
      <Act symbol="again" name="Check whether it was changed" word="Check" disabled={kept.busy} onClick={check} /></div> : null}
    {kept.done ? <p className="note" role="status">{kept.done}</p> : null}
    {kept.failure ? <p className="why-not" role="alert">{kept.failure}</p> : null}
  </form>;
}

/** The words a retired secret is left with. */
export const retiredWords = (secret: string): string =>
  secret + ' is retired. Every handle on it has ended, its value is gone from the broker, and the name ' + secret + ' is never used again.';

/** Retiring `secret`, by its owner. */
export function RetireSecret({ secret, retired }: { secret: string; retired: (message: string) => void }) {
  const kept = useKept(retireKey(secret), secret);
  const [sure, setSure] = useState(false);
  const send = (sending: PendingChange) => void kept.once(async () => {
    try {
      const answer = await secretsApi.retire(secret, sending.operation);
      if (answer.secret !== secret || answer.operation !== sending.operation) {
        throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service answered, but not with ' + secret + ' retired.' });
      }
      kept.release(); retired(retiredWords(secret));
    } catch (error) {
      if (answeredNo(error) && !kept.pending) kept.release();
      throw error;
    }
  });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (kept.blocked || !sure) return;
    const sending: PendingChange = { version: 1, operation: operationId(), secret };
    try { kept.keep(sending); } catch (error) { kept.setFailure('The retirement could not be kept in this tab, so it was not sent: ' + String(error)); return; }
    send(sending);
  };
  return <form onSubmit={submit} aria-label={'Retire ' + secret}>
    <h2>Retire this secret</h2>
    <p className="note">Every handle on it ends, its value leaves the broker, and its name is never used again.</p>
    <label><input type="checkbox" checked={sure} disabled={kept.blocked} onChange={(event) => setSure(event.target.checked)} /> Retire {secret} for good</label>
    <Act symbol="retire" name="Retire this secret" word="Retire" tone="danger" type="submit" disabled={kept.blocked || !sure} />
    {kept.pending ? <div role="status"><p>Retiring {secret} is not confirmed. Its original request is kept.</p>
      <Act symbol="again" name="Check whether it was retired" word="Check" disabled={kept.busy} onClick={() => { if (kept.pending) send(kept.pending); }} /></div> : null}
    {kept.failure ? <p className="why-not" role="alert">{kept.failure}</p> : null}
  </form>;
}
