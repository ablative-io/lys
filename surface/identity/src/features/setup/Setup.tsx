/** First administrator setup asks for a name; the server owns identity and admission. */
import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { Link } from 'react-router';
import { operationId, request } from '../../api';
import { confirmReceipt } from '../people/recorded-receipt';
import '../people/recorded-form.css';
import '../people/manage.css';

const STORAGE = 'lys.pending.first-setup';
interface Pending { operation: string; display_name: string }

function readPending(): Pending | null {
  const saved = sessionStorage.getItem(STORAGE);
  if (saved === null) return null;
  const value: unknown = JSON.parse(saved);
  if (!value || typeof value !== 'object' || !('operation' in value) || !('display_name' in value)
    || typeof value.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.operation)
    || typeof value.display_name !== 'string' || !value.display_name.trim()) {
    throw new Error('The saved setup request could not be read. Keep this tab open and contact your administrator.');
  }
  return { operation: value.operation, display_name: value.display_name };
}

function initial() {
  try { return { pending: readPending(), error: '' }; }
  catch (error) { return { pending: null, error: String(error) }; }
}

export function Setup({ completed }: { completed: () => void }) {
  const [saved] = useState(initial);
  const [pending, setPending] = useState(saved.pending);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(saved.error);
  const [nameError, setNameError] = useState('');
  const sending = useRef(false);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => { heading.current?.focus(); }, []);

  async function finish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (sending.current || saved.error) return;
    const name = String(new FormData(event.currentTarget).get('display_name') ?? '').trim();
    setNameError('');
    if (!pending && !name) { setNameError('Enter the name you want people to see.'); return; }
    // Profile::new in lys-identity declares these limits; never submit an invalid pending request.
    if (!pending && ([...name].length > 200 || [...name].some((character) => {
      const point = character.codePointAt(0);
      return point !== undefined && (point < 32 || (point >= 127 && point <= 159));
    }))) { setNameError('Use a name of 200 characters or fewer, without control characters.'); return; }
    sending.current = true;
    setBusy(true);
    setError('');
    try {
      const body = pending ?? { operation: operationId(), display_name: name };
      sessionStorage.setItem(STORAGE, JSON.stringify(body));
      setPending(body);
      const answer = await request<unknown>('/setup', body);
      confirmReceipt(answer, body.operation, '/setup');
      if (!answer || typeof answer !== 'object' || !('person' in answer) || !('receipt' in answer)
        || !answer.receipt || typeof answer.receipt !== 'object' || !('identity' in answer.receipt)
        || !('change_kind' in answer.receipt) || answer.person !== answer.receipt.identity
        || answer.receipt.change_kind !== 7) {
        throw new Error('The setup confirmation did not match your request. Retry to check the same request safely.');
      }
      sessionStorage.removeItem(STORAGE);
      completed();
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      sending.current = false;
      setBusy(false);
    }
  }

  return <div className="page directory-manage">
    <div className="head"><div>
      <div className="eyebrow">Welcome to Lys</div>
      <h1 ref={heading} tabIndex={-1}>Finish setting up your account</h1>
      <p className="sub">You’re signed in. Choose the name people will see in the directory.</p>
    </div></div>
    <div className="directory-editor">
      <form className="recorded-form" aria-label="Finish setup" onSubmit={finish} aria-busy={busy}>
        <div className="field">
          <label htmlFor="setup-name">Full name</label>
          <input id="setup-name" name="display_name" autoComplete="name" required
            defaultValue={pending?.display_name ?? ''} readOnly={pending !== null} disabled={busy || Boolean(saved.error)}
            aria-invalid={Boolean(nameError)} aria-describedby={nameError ? 'setup-name-help setup-name-error' : 'setup-name-help'} />
          {nameError ? <p id="setup-name-error" role="alert">{nameError}</p> : null}
          <p id="setup-name-help" className="note sec">Usually your first name and surname. If you use a single name, that’s fine too.</p>
        </div>
        <p className="sec">This connects the account you just signed in with and makes you the directory administrator.</p>
        {pending && !busy ? <p role="status">Your setup request is saved. Retry checks the same request safely, without creating another account.</p> : null}
        {error ? <div role="alert" className="why-not">
          <b>Setup could not be confirmed.</b>
          <p>{saved.error ? 'Your saved setup request could not be restored. Keep this tab open and ask your administrator for help.' : pending ? 'Your request is saved. Choose Retry setup to check it safely.' : 'Your browser could not save the request. Nothing has been submitted. Check browser storage before trying again.'}</p>
          <details><summary>Technical details</summary><p>{error}</p></details>
        </div> : null}
        <button className="btn primary" type="submit" disabled={busy || Boolean(saved.error)}>
          {busy ? 'Finishing setup…' : pending ? 'Retry setup' : 'Finish setup'}
        </button>
        <p role="status" aria-live="polite">{busy ? 'Saving your account. You can stay on this page.' : ''}</p>
      </form>
      <details><summary>Advanced</summary>
        <p className="sec">Manage people, sign-in connections and lifecycle states individually.</p>
        <Link to="/directory/manage?advanced=1">Open directory controls</Link>
      </details>
    </div>
  </div>;
}
