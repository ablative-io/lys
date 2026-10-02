/**
 * Sign-in on Lys's own page. The email and password are posted to the
 * service, which carries them to the sign-in service server-side: the
 * browser never leaves Lys's origin. A refusal is the service's own words,
 * which never say whether the email or the password was wrong.
 */
import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { API, Refused, request } from '../../api';
import type { SignedIn } from '../../generated';
import '../people/recorded-form.css';
import './sign-in.css';

/** Where a person lands once signed in. */
export const SIGNED_IN = '/#/me';

/** A refusal in the words a person reads, without its name. */
export function reasonOf(failure: unknown): string {
  if (failure instanceof Refused) {
    if (failure.refusal.refusal === 'SignInRefused') return 'That email or password is not right.';
    if (failure.status === 0) return 'Lys could not be reached. Check your connection and try again.';
    return failure.refusal.reason.replace(/^[A-Za-z]+: /, '');
  }
  return String(failure);
}

/** A provider a person may sign in with: one button each. */
interface Offered { id: string; name: string; provider: string | null }

/** What a provider sign-in that did not finish is called, in Lys words. */
const PROVIDER_REFUSALS: Record<string, string> = {
  SignInStateUnknown: 'That sign-in expired or was already used. Try again.',
  SecondFactorUnsupported: 'That account asks for a second factor, which Lys sign-in does not take yet.',
  SignInThrottled: 'Too many sign-ins failed from here. Wait a little, then try again.',
};

/** The refusal a provider sign-in came back with, named in the address. */
function refusedInAddress(): string {
  const name = /[?&]refused=([A-Za-z]+)/.exec(location.hash)?.[1];
  if (!name) return '';
  return PROVIDER_REFUSALS[name] ?? 'The sign-in did not finish. Try again.';
}

function Providers() {
  const [offered, setOffered] = useState<Offered[]>([]);
  useEffect(() => {
    let live = true;
    request<{ providers: Offered[] }>('/sign-in/providers').then(
      (answer) => { if (live) setOffered(answer.providers); },
      () => { if (live) setOffered([]); },
    );
    return () => { live = false; };
  }, []);
  if (offered.length === 0) return null;
  return <div className="sign-in-providers" aria-label="Sign in with a provider">
    {offered.map((entry) => <a key={entry.id} className="btn" href={API + '/sign-in/providers/' + encodeURIComponent(entry.id)}>
      Sign in with {entry.name}
    </a>)}
  </div>;
}

/**
 * Where a product's sign-in, or a connected app's approval, continues once
 * the person is signed in: back to one of Lys's own authorize addresses,
 * never anywhere else.
 */
export function continuation(): string | null {
  const encoded = /[?&]continue=([^&]+)/.exec(location.hash)?.[1];
  if (!encoded) return null;
  let target: string;
  try { target = decodeURIComponent(encoded); } catch { return null; }
  const allowed = ['/oauth/authorize?', '/oauth/mcp/authorize?'];
  return allowed.some((start) => target.startsWith(start)) ? target : null;
}

/** The sign-in form, on its own or inside a screen that needs a session. */
export function SignIn({ signedIn = () => location.replace(SIGNED_IN) }: { signedIn?: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(refusedInAddress);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => { heading.current?.focus(); }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    const form = new FormData(event.currentTarget);
    const email = String(form.get('email') ?? '').trim();
    const password = String(form.get('password') ?? '');
    if (!email || !password) {
      setError('Enter your email and your password.');
      return;
    }
    setBusy(true);
    setError('');
    try {
      await request<SignedIn>('/sign-in', { email, password });
      const next = continuation();
      if (next) location.assign(next);
      else signedIn();
    } catch (failure) {
      setError(reasonOf(failure));
    } finally {
      setBusy(false);
    }
  }

  return <div className="page sign-in">
    <div className="sign-in-mark" aria-hidden="true">ID</div>
    <div className="eyebrow">Lys</div>
    <h1 ref={heading} tabIndex={-1}>Sign in</h1>
    <p className="sub">Sign in to Lys with your email and password.</p>
    <form className="recorded-form sign-in-form" aria-label="Sign in" onSubmit={submit} aria-busy={busy} noValidate>
      <div className="field">
        <label htmlFor="sign-in-email">Email</label>
        <input id="sign-in-email" name="email" type="email" autoComplete="username" required disabled={busy} />
      </div>
      <div className="field">
        <label htmlFor="sign-in-password">Password</label>
        <input id="sign-in-password" name="password" type="password" autoComplete="current-password" required disabled={busy} />
      </div>
      {error ? <p role="alert" className="why-not">{error}</p> : null}
      <button className="btn primary" type="submit" disabled={busy}>{busy ? 'Signing in…' : 'Sign in'}</button>
    </form>
    <Providers />
  </div>;
}
