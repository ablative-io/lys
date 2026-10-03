/**
 * First-run setup on Lys's own setup page, and the configured
 * administrator's finishing step. The server owns identity and admission.
 *
 * The setup page is opened by the install with a one-time code in the
 * address fragment. The code is taken from the address once and the address
 * is replaced without it, so it is never left where a person reads it. When
 * no browser could be opened the install writes the code to a file instead,
 * and the page opened without a code asks for it.
 *
 * The password policy shown before submit is Lys's own, as the service's
 * configuration states it; a password is measured as the service measures
 * it, in bytes of UTF-8.
 */
import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, operationId, request } from '../../api';
import { sendKept } from '../../kept';
import { confirmReceipt } from '../people/recorded-receipt';
import { SIGNED_IN, reasonOf } from '../sign-in/SignIn';
import '../people/recorded-form.css';
import '../people/manage.css';
import '../sign-in/sign-in.css';

let taken: string | null = null;

/** The setup code this page was opened with, taken from the address once. */
export function setupCode(): string {
  if (taken === null) {
    const match = /(?:^#|&)code=([A-Za-z0-9]+)/.exec(location.hash);
    taken = match?.[1] ?? '';
    if (match) history.replaceState(null, '', location.pathname);
  }
  return taken;
}

/** Forget the taken code, as a new page load does. */
export function forgetSetupCode(): void {
  taken = null;
}

interface Policy { length_min: number; length_max: number; words: string }
interface Opened { purpose: 'first-run' | 'password'; email: string | null; policy: Policy | null }

/** Whether `password` is outside the policy's lengths, measured as the service measures it. */
function outOfLength(policy: Policy | null, password: string): boolean {
  if (!policy) return password === '';
  let length = 0;
  for (const character of password) {
    const point = character.codePointAt(0) ?? 0;
    length += point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4;
  }
  return length < policy.length_min || length > policy.length_max;
}

const closed = (failure: unknown) => failure instanceof Refused && failure.refusal.refusal === 'SetupClosed';

function setupRefusal(failure: unknown): string {
  if (closed(failure)) return 'Lys is already set up. Sign in instead.';
  if (failure instanceof Refused && failure.refusal.refusal === 'SetupCodeRefused') {
    return 'This setup code is not valid or was already used. Run lys identity setup-code on the machine Lys runs on for a fresh one.';
  }
  // Every setup route counts against one limit of five a minute (docs/SIGN-IN-ADMISSION.md).
  if (failure instanceof Refused && failure.refusal.refusal === 'SignInThrottled') {
    return 'Lys takes five setup attempts a minute, and that many have been made. Wait a minute, then try again.';
  }
  return reasonOf(failure);
}

/** The setup code typed in, for a page opened without one. */
function CodeEntry({ refusal, take }: { refusal: string; take: (code: string) => void }) {
  const [error, setError] = useState('');
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const code = String(new FormData(event.currentTarget).get('code') ?? '').trim();
    if (!code) { setError('Enter the setup code.'); return; }
    setError('');
    take(code);
  }
  const why = error || refusal;
  return <form className="recorded-form sign-in-form" aria-label="Setup code" onSubmit={submit} noValidate>
    <div className="field">
      <label htmlFor="setup-code">Setup code</label>
      <input id="setup-code" name="code" autoComplete="off" spellCheck={false} required aria-describedby="setup-code-where" />
      <p id="setup-code-where" className="note">The setup code is in the file named setup-code in the folder the install printed. Use the full path shown on the line “the setup code is in …”. Only the account that ran the install can read it. The copy under state/ is a verification record, not the code.</p>
    </div>
    {why ? <p role="alert" className="why-not">{why}</p> : null}
    <button className="btn primary" type="submit">Check the code</button>
  </form>;
}

/** Lys's setup page: the first administrator, or the administrator's new password. */
export function FirstRunSetup({ code: linked, done = () => location.replace(SIGNED_IN) }: { code: string; done?: () => void }) {
  const [code, setCode] = useState(linked);
  const [opened, setOpened] = useState<Opened | null>(null);
  const [refusal, setRefusal] = useState('');
  // A typed code that is refused is asked for again; a closed setup ends the page either way.
  const [entryRefusal, setEntryRefusal] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const operation = useRef(operationId());
  useEffect(() => {
    if (!code) return undefined;
    let live = true;
    request<Opened>('/setup/open', { code }).then(
      (answer) => { if (live) setOpened(answer); },
      (failure: unknown) => {
        if (!live) return;
        if (linked || closed(failure)) { setRefusal(setupRefusal(failure)); return; }
        setEntryRefusal(setupRefusal(failure));
        setCode('');
      },
    );
    return () => { live = false; };
  }, [code, linked]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy || !opened) return;
    const form = new FormData(event.currentTarget);
    const field = (name: string) => String(form.get(name) ?? '');
    const password = field('password');
    if (password !== field('confirm')) { setError('The two passwords are not the same.'); return; }
    if (outOfLength(opened.policy, password)) { setError(opened.policy?.words ?? 'Enter a password.'); return; }
    const firstRun = opened.purpose === 'first-run';
    const name = field('display_name').trim();
    const email = (opened.email ?? field('email')).trim();
    if (firstRun && (!name || !email)) { setError('Enter your name and your email.'); return; }
    setBusy(true);
    setError('');
    try {
      if (firstRun) {
        await request('/setup/administrator', { code, operation: operation.current, display_name: name, email, password });
      } else {
        await request('/setup/password', { code, password });
      }
      done();
    } catch (failure) {
      setError(setupRefusal(failure));
    } finally {
      setBusy(false);
    }
  }

  const firstRun = opened?.purpose !== 'password';
  return <div className="page sign-in">
    <div className="sign-in-mark" aria-hidden="true">ID</div>
    <div className="eyebrow">Welcome to Lys</div>
    <h1>{firstRun ? 'Set up Lys' : 'Choose a new password'}</h1>
    {refusal ? <div role="alert" className="why-not"><p>{refusal}</p><a className="btn" href="/#/sign-in">Go to sign in</a></div> : null}
    {!refusal && !code ? <CodeEntry refusal={entryRefusal} take={(typed) => { setEntryRefusal(''); setCode(typed); }} /> : null}
    {!refusal && code && !opened ? <p className="dim">Opening setup…</p> : null}
    {opened ? <form className="recorded-form sign-in-form" aria-label={firstRun ? 'Set up Lys' : 'New password'} onSubmit={submit} aria-busy={busy} noValidate>
      <p className="sub">{firstRun ? 'You will be the administrator. Choose how you sign in.' : `Set a new password for ${opened.email ?? 'the administrator'}.`}</p>
      {firstRun ? <div className="field">
        <label htmlFor="setup-full-name">Your name</label>
        <input id="setup-full-name" name="display_name" autoComplete="name" required disabled={busy} />
      </div> : null}
      {firstRun ? <div className="field">
        <label htmlFor="setup-email">Email</label>
        {opened.email
          ? <input id="setup-email" name="email" type="email" value={opened.email} readOnly />
          : <input id="setup-email" name="email" type="email" autoComplete="email" required disabled={busy} />}
      </div> : null}
      <div className="field">
        <label htmlFor="setup-password">Password</label>
        <input id="setup-password" name="password" type="password" autoComplete="new-password" required disabled={busy} aria-describedby={opened.policy ? 'setup-policy' : undefined} />
        {opened.policy ? <p id="setup-policy" className="note">{opened.policy.words}</p> : null}
      </div>
      <div className="field">
        <label htmlFor="setup-confirm">Password again</label>
        <input id="setup-confirm" name="confirm" type="password" autoComplete="new-password" required disabled={busy} />
      </div>
      {error ? <p role="alert" className="why-not">{error}</p> : null}
      <button className="btn primary" type="submit" disabled={busy}>{busy ? 'Setting up…' : firstRun ? 'Set up and sign in' : 'Save and sign in'}</button>
    </form> : null}
  </div>;
}

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
    // Profile::new in lys-identity refuses control characters; never submit an invalid pending request.
    if (!pending && ([...name].some((character) => {
      const point = character.codePointAt(0);
      return point !== undefined && (point < 32 || (point >= 127 && point <= 159));
    }))) { setNameError('Use a name without control characters.'); return; }
    sending.current = true;
    setBusy(true);
    setError('');
    try {
      const body = pending ?? { operation: operationId(), display_name: name };
      await sendKept(STORAGE, body, async () => {
        setPending(body);
        const answer = await request<unknown>('/setup', body);
        confirmReceipt(answer, body.operation, '/setup');
        if (!answer || typeof answer !== 'object' || !('person' in answer) || !('receipt' in answer)
          || !answer.receipt || typeof answer.receipt !== 'object' || !('identity' in answer.receipt)
          || !('change_kind' in answer.receipt) || answer.person !== answer.receipt.identity
          || answer.receipt.change_kind !== 7) {
          throw new Error('The setup confirmation did not match your request. Retry to check the same request safely.');
        }
      }, false);
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
          <p><small className="refusal-name">{error}</small></p>
        </div> : null}
        <button className="btn primary" type="submit" disabled={busy || Boolean(saved.error)}>
          {busy ? 'Finishing setup…' : pending ? 'Retry setup' : 'Finish setup'}
        </button>
        <p role="status" aria-live="polite">{busy ? 'Saving your account. You can stay on this page.' : ''}</p>
      </form>
    </div>
  </div>;
}
