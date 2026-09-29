/**
 * A Lys account's email, password and sign-in, changed on Lys screens and
 * carried out by the service: a person's own account on the You screen,
 * confirmed by their password, and anyone's for the administrator.
 * Passwords are sent once and never shown or kept on the page.
 */
import { useEffect, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { useParams } from 'react-router';
import { request } from '../../api';
import { reasonOf } from '../sign-in/SignIn';
import './recorded-form.css';

interface AccountView { email: string | null; enabled: boolean }

type Status = { kind: 'done'; words: string } | { kind: 'refused'; words: string } | null;

function useAccount(path: string) {
  const [account, setAccount] = useState<AccountView | null>(null);
  const [unavailable, setUnavailable] = useState('');
  useEffect(() => {
    let live = true;
    request<AccountView>(path).then(
      (answer) => { if (live) setAccount(answer); },
      (failure: unknown) => { if (live) setUnavailable(reasonOf(failure)); },
    );
    return () => { live = false; };
  }, [path]);
  return { account, setAccount, unavailable };
}

/** One change of an account: a form whose fields are read on submit and cleared after. */
function Change({ label, submit, children, action }: {
  label: string;
  submit: (form: FormData) => Promise<string>;
  children: ReactNode;
  action: string;
}) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<Status>(null);
  async function send(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    const element = event.currentTarget;
    setBusy(true);
    setStatus(null);
    try {
      const words = await submit(new FormData(element));
      element.reset();
      setStatus({ kind: 'done', words });
    } catch (failure) {
      setStatus({ kind: 'refused', words: failure instanceof Error && !('refusal' in failure) ? failure.message : reasonOf(failure) });
    } finally {
      setBusy(false);
    }
  }
  return <form className="recorded-form" aria-label={label} onSubmit={send} aria-busy={busy} noValidate>
    {children}
    {status?.kind === 'done' ? <p role="status">{status.words}</p> : null}
    {status?.kind === 'refused' ? <p role="alert" className="why-not">{status.words}</p> : null}
    <button className="btn" type="submit" disabled={busy}>{busy ? 'Saving…' : action}</button>
  </form>;
}

function Field({ id, label, type = 'text', autoComplete }: { id: string; label: string; type?: string; autoComplete: string }) {
  return <div className="field">
    <label htmlFor={id}>{label}</label>
    <input id={id} name={id} type={type} autoComplete={autoComplete} required />
  </div>;
}

const text = (form: FormData, name: string) => String(form.get(name) ?? '');

function samePasswords(form: FormData): string {
  const password = text(form, 'account-new-password');
  if (password !== text(form, 'account-new-password-again')) throw new Error('The two new passwords are not the same.');
  return password;
}

/** The signed-in person's own Lys account, on the You screen. */
export function OwnAccount({ readOnly = false }: { readOnly?: boolean }) {
  const { account, setAccount, unavailable } = useAccount('/me/account');
  if (unavailable) return <div className="card" id="lys-account"><h2>Your Lys account</h2><p className="note">{unavailable}</p></div>;
  return <div className="card" id="lys-account">
    <h2>Your Lys account</h2>
    <p className="note">You sign in to Lys with {account?.email ?? 'your email'}.</p>
    {readOnly ? null : <><Change label="Change your email" action="Change email" submit={async (form) => {
      const changed = await request<AccountView>('/me/account/email', { email: text(form, 'account-email').trim(), password: text(form, 'account-password') });
      setAccount(changed);
      return `You now sign in with ${changed.email ?? 'your new email'}.`;
    }}>
      <Field id="account-email" label="New email" type="email" autoComplete="email" />
      <Field id="account-password" label="Your password, to confirm" type="password" autoComplete="current-password" />
    </Change>
    <Change label="Change your password" action="Change password" submit={async (form) => {
      await request<AccountView>('/me/account/password', { current: text(form, 'account-current-password'), password: samePasswords(form) });
      return 'Your password is changed.';
    }}>
      <Field id="account-current-password" label="Current password" type="password" autoComplete="current-password" />
      <Field id="account-new-password" label="New password" type="password" autoComplete="new-password" />
      <Field id="account-new-password-again" label="New password again" type="password" autoComplete="new-password" />
    </Change></>}
  </div>;
}

/** A person's Lys account, as the administrator changes it. */
export function PersonAccount({ id }: { id: string }) {
  const path = '/directory/people/' + encodeURIComponent(id) + '/account';
  const { account, setAccount, unavailable } = useAccount(path);
  if (unavailable) return <section className="card"><h2>Lys account</h2><p className="note">{unavailable}</p></section>;
  return <section className="card" aria-label="Lys account">
    <h2>Lys account</h2>
    <p className="note">{account ? `Signs in with ${account.email ?? 'no email'}. Sign-in is ${account.enabled ? 'enabled' : 'disabled'}.` : 'Reading the account…'}</p>
    <Change label="Change this person's email" action="Change email" submit={async (form) => {
      const changed = await request<AccountView>(path + '/email', { email: text(form, 'account-email').trim() });
      setAccount(changed);
      return `They now sign in with ${changed.email ?? 'the new email'}.`;
    }}>
      <Field id="account-email" label="New email" type="email" autoComplete="off" />
    </Change>
    <Change label="Reset this person's password" action="Reset password" submit={async (form) => {
      await request<AccountView>(path + '/password', { password: samePasswords(form) });
      return 'The new password signs them in; the old one no longer does.';
    }}>
      <Field id="account-new-password" label="New password" type="password" autoComplete="new-password" />
      <Field id="account-new-password-again" label="New password again" type="password" autoComplete="new-password" />
    </Change>
    {account ? <Change label={account.enabled ? 'Disable sign-in' : 'Enable sign-in'} action={account.enabled ? 'Disable sign-in' : 'Enable sign-in'} submit={async () => {
      const changed = await request<AccountView>(path + '/enabled', { enabled: !account.enabled });
      setAccount(changed);
      return changed.enabled ? 'They can sign in again.' : 'They can no longer sign in.';
    }}><p className="note">{account.enabled ? 'Disabling stops this person signing in at their next sign-in.' : 'Enabling lets this person sign in again.'}</p></Change> : null}
  </section>;
}

/** The administrator's account screen for one person. */
export function AccountPage() {
  const { id = '' } = useParams();
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Directory</div><h1>Lys account</h1>
      <p className="sub">Change this person's email and password, or whether they can sign in.</p></div></div>
    <PersonAccount id={id} />
  </div>;
}
