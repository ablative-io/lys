/**
 * A Lys account's email, password and sign-in, changed on Lys screens and
 * carried out by the service: a person's own account in Settings, and
 * anyone's for the administrator. Lys says whether an account has a password
 * and which providers it signs in with: a person's own change is confirmed by
 * their password when they have one, and someone who signs in with Google and
 * has none sets a first password and changes their email with nothing to
 * confirm. Passwords are sent once and never shown or kept on the page.
 */
import { useEffect, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { useLocation } from 'react-router';
import { request } from '../../api';
import { reasonOf } from '../sign-in/SignIn';
import './recorded-form.css';
import { Act } from '../../shell/Act';
import type { SymbolName } from '../../shell/symbols';

interface AccountView {
  email: string | null;
  enabled: boolean;
  /** Whether the account has a Lys password; absent from an older service, which always asked for it. */
  password?: boolean;
  /** The providers it signs in with, by the name the sign-in page offers: "google"; absent from an older service. */
  providers?: string[];
}

/** A provider as the sign-in page names it. */
const PROVIDER: Record<string, string> = { google: 'Google', microsoft: 'Microsoft', github: 'GitHub' };
const providerName = (provider: string): string => PROVIDER[provider] ?? provider;

/** How the account signs in, in a sentence: its email, its providers, or both. */
export function signsInWith(account: AccountView): string {
  const providers = (account.providers ?? []).map(providerName);
  const via = providers.join(' or ');
  if (!account.password && providers.length) return `You sign in with ${via}.`;
  const email = account.email ?? 'your email';
  return providers.length ? `You sign in with ${email} or with ${via}.` : `You sign in with ${email}.`;
}

type Status = { kind: 'done'; words: string } | { kind: 'refused'; words: string } | null;

function useAccount(path: string) {
  const [account, setAccount] = useState<AccountView | null>(null);
  const [unavailable, setUnavailable] = useState('');
  useEffect(() => {
    let live = true;
    request<AccountView>(path).then(
      (answer) => {
        if (live) setAccount(answer);
      },
      (failure: unknown) => {
        if (live) setUnavailable(reasonOf(failure));
      },
    );
    return () => {
      live = false;
    };
  }, [path]);
  return { account, setAccount, unavailable };
}

/** One change of an account: a form whose fields are read on submit and cleared after. */
function Change({
  label,
  submit,
  children,
  action,
  saved,
  cancel,
  symbol = 'save',
  word = 'Save',
  tone = 'plain',
}: {
  label: string;
  submit: (form: FormData) => Promise<string>;
  children: ReactNode;
  action: string;
  saved?: (words: string) => void;
  cancel?: () => void;
  /** How the act is drawn: its symbol and the one word beside it. A change that saves what was typed is drawn as a save. */
  symbol?: SymbolName;
  word?: string;
  tone?: 'plain' | 'danger';
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
      if (saved) saved(words);
      else setStatus({ kind: 'done', words });
    } catch (failure) {
      setStatus({ kind: 'refused', words: failure instanceof Error && !('refusal' in failure) ? failure.message : reasonOf(failure) });
    } finally {
      setBusy(false);
    }
  }
  const said = (
    <>
      {status?.kind === 'done' ? <p role="status">{status.words}</p> : null}
      {status?.kind === 'refused' ? (
        <p role="alert" className="why-not">
          {status.words}
        </p>
      ) : null}
    </>
  );
  return (
    <form className="recorded-form" aria-label={label} onSubmit={send} aria-busy={busy} noValidate>
      {children}
      {said}
      <Act symbol={symbol} name={busy ? 'Saving…' : action} word={busy ? 'Saving…' : word} tone={tone} type="submit" disabled={busy} />
      {cancel ? (
        <>
          {' '}
          <Act symbol="close" name="Cancel" word="Cancel" disabled={busy} onClick={cancel} />
        </>
      ) : null}
    </form>
  );
}

function Field({ id, label, type = 'text', autoComplete }: { id: string; label: string; type?: string; autoComplete: string }) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input id={id} name={id} type={type} autoComplete={autoComplete} required />
    </div>
  );
}

const text = (form: FormData, name: string) => String(form.get(name) ?? '');

function samePasswords(form: FormData): string {
  const password = text(form, 'account-new-password');
  if (password !== text(form, 'account-new-password-again')) throw new Error('The two new passwords are not the same.');
  return password;
}

/** The sign-in address links to the account settings without mounting credential fields. */
export function OwnAccount() {
  const { account, unavailable } = useAccount('/me/account');
  return (
    <div className="card" id="lys-account">
      <p className="note">
        {unavailable || (account ? signsInWith(account) : 'Reading your sign-in address…')} <a href="#/settings/account">Change in Settings</a>
      </p>
    </div>
  );
}

/** Whether this page was reached by a founding letter's link (features/found/Prove.tsx), so it says how the person is signed in. */
const foundedByLetter = (): boolean => new URLSearchParams(location.hash.split('?')[1] ?? '').get('founded') === '1';

/** Whether this page was reached by a handed admission link (DIRECTORY-112, features/found/Prove.tsx at #/added/:proof). */
const addedByLink = (): boolean => new URLSearchParams(location.hash.split('?')[1] ?? '').get('added') === '1';

/** The product an admission goes on to, from the proving page's navigation state only (DIRECTORY-113 R3): an https origin, else none. */
function continueFrom(state: unknown): URL | null {
  if (typeof state !== 'object' || state === null || !('continueTo' in state) || typeof state.continueTo !== 'string') return null;
  try {
    const address = new URL(state.continueTo);
    return address.protocol === 'https:' && address.origin === state.continueTo ? address : null;
  } catch {
    return null;
  }
}

/** Credential fields exist only for the change the person has requested. */
export function AccountSettings() {
  const onward = continueFrom(useLocation().state);
  const { account, setAccount, unavailable } = useAccount('/me/account');
  const [changing, setChanging] = useState<'email' | 'password' | null>(null);
  const [notice, setNotice] = useState('');
  function saved(words: string) {
    setChanging(null);
    setNotice(words);
  }
  function open(change: 'email' | 'password') {
    setNotice('');
    setChanging(change);
  }
  return (
    <section aria-label="Your account">
      <h2>Your account</h2>
      {foundedByLetter() ? <p role="status" className="note founded-note">You are signed in by the letter's link. Set a password or connect Google here so you can sign in again.</p> : null}
      {addedByLink() ? <p role="status" className="note added-note">You are signed in by the link you were given. Set a password or connect Google here so you can sign in again.{onward ? <> <a href={onward.origin}>Continue to {onward.host}</a></> : null}</p> : null}
      {unavailable ? (
        <p role="alert" className="why-not">
          {unavailable}
        </p>
      ) : (
        <>
          {notice ? <p role="status">{notice}</p> : null}
          <div className="set-row">
            <div>
              <div>Email</div>
              <div className="d">{account ? signsInWith(account) : 'Reading your sign-in address…'}</div>
            </div>
            <Act symbol="edit" name="Change email" word="Change email" disabled={!account || changing !== null} onClick={() => open('email')} />
          </div>
          {changing === 'email' ? (
            <Change
              label="Change your email"
              action="Save email"
              saved={saved}
              cancel={() => setChanging(null)}
              submit={async (form) => {
                // An account with no password has nothing to confirm with: its session is the proof.
                const email = text(form, 'account-email').trim();
                const changed = await request<AccountView>('/me/account/email', account?.password === false ? { email } : { email, password: text(form, 'account-password') });
                setAccount(changed);
                return `You now sign in with ${changed.email ?? 'your new email'}.`;
              }}
            >
              <Field id="account-email" label="New email" type="email" autoComplete="email" />
              {account?.password === false ? null : <Field id="account-password" label="Your password, to confirm" type="password" autoComplete="current-password" />}
            </Change>
          ) : null}
          {account?.password === false ? (
            <>
              <div className="set-row">
                <div>
                  <div>Password</div>
                  <div className="d">You have no Lys password yet. Set one to sign in with your email as well.</div>
                </div>
                <Act symbol="edit" name="Set a password" word="Set a password" disabled={changing !== null} onClick={() => open('password')} />
              </div>
              {changing === 'password' ? (
                <Change
                  label="Set your password"
                  action="Save password"
                  saved={saved}
                  cancel={() => setChanging(null)}
                  submit={async (form) => {
                    const changed = await request<AccountView>('/me/account/password', { password: samePasswords(form) });
                    setAccount(changed);
                    const via = (changed.providers ?? []).map(providerName).join(' or ');
                    return via ? `Your password is set. You can sign in with it or with ${via}.` : 'Your password is set.';
                  }}
                >
                  <Field id="account-new-password" label="New password" type="password" autoComplete="new-password" />
                  <Field id="account-new-password-again" label="New password again" type="password" autoComplete="new-password" />
                </Change>
              ) : null}
            </>
          ) : (
            <>
              <div className="set-row">
                <div>
                  <div>Password</div>
                  <div className="d">Choose a new password to sign in to Lys.</div>
                </div>
                <Act symbol="edit" name="Change password" word="Change password" disabled={!account || changing !== null} onClick={() => open('password')} />
              </div>
              {changing === 'password' ? (
                <Change
                  label="Change your password"
                  action="Save password"
                  saved={saved}
                  cancel={() => setChanging(null)}
                  submit={async (form) => {
                    await request<AccountView>('/me/account/password', { current: text(form, 'account-current-password'), password: samePasswords(form) });
                    return 'Your password is changed.';
                  }}
                >
                  <Field id="account-current-password" label="Current password" type="password" autoComplete="current-password" />
                  <Field id="account-new-password" label="New password" type="password" autoComplete="new-password" />
                  <Field id="account-new-password-again" label="New password again" type="password" autoComplete="new-password" />
                </Change>
              ) : null}
            </>
          )}
        </>
      )}
    </section>
  );
}

/** A person's Lys account, as the administrator changes it. */
export function PersonAccount({ id }: { id: string }) {
  const path = '/directory/people/' + encodeURIComponent(id) + '/account';
  const { account, setAccount, unavailable } = useAccount(path);
  if (unavailable)
    return (
      <section className="card" aria-label="Lys account">
        <h2>Lys account</h2>
        <p className="note">{unavailable}</p>
      </section>
    );
  return (
    <section className="card" aria-label="Lys account">
      <h2>Lys account</h2>
      <p className="note">{account ? `Signs in with ${account.email ?? 'no email'}. Sign-in is ${account.enabled ? 'enabled' : 'disabled'}.` : 'Reading the account…'}</p>
      <Change
        label="Change this person's email"
        action="Change email"
        submit={async (form) => {
          const changed = await request<AccountView>(path + '/email', { email: text(form, 'account-email').trim() });
          setAccount(changed);
          return `They now sign in with ${changed.email ?? 'the new email'}.`;
        }}
      >
        <Field id="account-email" label="New email" type="email" autoComplete="off" />
      </Change>
      <Change
        label="Reset this person's password"
        action="Reset password"
        symbol="again"
        word="Reset"
        submit={async (form) => {
          await request<AccountView>(path + '/password', { password: samePasswords(form) });
          return 'The new password signs them in; the old one no longer does.';
        }}
      >
        <Field id="account-new-password" label="New password" type="password" autoComplete="new-password" />
        <Field id="account-new-password-again" label="New password again" type="password" autoComplete="new-password" />
      </Change>
      {account ? (
        <Change
          label={account.enabled ? 'Disable sign-in' : 'Enable sign-in'}
          action={account.enabled ? 'Disable sign-in' : 'Enable sign-in'}
          symbol={account.enabled ? 'suspend' : 'start'}
          word={account.enabled ? 'Disable' : 'Enable'}
          tone={account.enabled ? 'danger' : 'plain'}
          submit={async () => {
            const changed = await request<AccountView>(path + '/enabled', { enabled: !account.enabled });
            setAccount(changed);
            return changed.enabled ? 'They can sign in again.' : 'They can no longer sign in.';
          }}
        >
          <p className="note">{account.enabled ? 'Disabling stops this person signing in at their next sign-in.' : 'Enabling lets this person sign in again.'}</p>
        </Change>
      ) : null}
    </section>
  );
}

/** Only mounted within the administrator's account view. The service also
 * judges organisation.administer before reading or withdrawing a link. */
export function AdmissionWaiting({ id }: { id: string }) {
  const path = '/people/' + encodeURIComponent(id) + '/admission';
  const [waiting, setWaiting] = useState(false);
  const [reading, setReading] = useState('');
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const [notice, setNotice] = useState('');
  useEffect(() => {
    let live = true;
    setWaiting(false); setReading(''); setFailure(''); setNotice('');
    request<{ person: string; waiting: boolean }>(path).then((answer) => {
      if (live) setWaiting(answer.person === id && answer.waiting === true);
    }, (error: unknown) => { if (live) setReading(reasonOf(error)); });
    return () => { live = false; };
  }, [id, path]);
  async function withdraw() {
    if (busy) return;
    setBusy(true); setFailure('');
    try {
      const answer = await request<{ person: string; digest: string }>(path + '/withdraw', {});
      if (answer.person !== id || typeof answer.digest !== 'string' || !answer.digest) throw new Error('The service did not confirm the withdrawal.');
      setWaiting(false); setNotice('The sign-in link was withdrawn.');
    } catch (error) { setFailure(reasonOf(error)); }
    finally { setBusy(false); }
  }
  return <div className="admission-waiting">
    {reading ? <p role="status">The waiting sign-in link could not be read: {reading}</p> : null}
    {waiting ? <><p>A sign-in link is waiting</p><Act symbol="close" name="Withdraw sign-in link" word="Withdraw" disabled={busy} onClick={() => void withdraw()} /></> : null}
    {failure ? <p role="alert">{failure}</p> : null}
    {notice ? <p role="status">{notice}</p> : null}
  </div>;
}
