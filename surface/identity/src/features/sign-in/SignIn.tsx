/**
 * Sign-in on Lys's own page. The email and password are posted to the
 * service, which carries them to the sign-in service server-side: the
 * browser never leaves Lys's origin. A refusal is the service's own words,
 * which never say whether the email or the password was wrong.
 */
import { useEffect, useRef, useState } from 'react';
import type { FormEvent, MouseEvent } from 'react';
import { API, Refused, request } from '../../api';
import type { SignedIn } from '../../generated';
import '../people/recorded-form.css';
import './sign-in.css';
import { Act } from '../../shell/Act';
import { AccountHelpWords, administratorWords, useAdministratorContact } from '../signin/AccountHelp';
import { LysMark } from '../../shell/LysMark';
import { forgetLanding, keepAcrossSignIn, rememberFileReturn, rememberLanding } from '../signin/fileReturn';

/** Where every sign-in begun on the sign-in page lands: Home. One begun on a page lands back on it (signin/Gate.tsx, signin/fileReturn.tsx). */
export const SIGNED_IN = '/#/';

/** A refusal in the words a person reads, without its name. */
export function reasonOf(failure: unknown): string {
  if (failure instanceof Refused) {
    if (failure.refusal.refusal === 'SignInRefused') return 'That email or password is not right.';
    if (failure.status === 0) return 'Lys could not be reached. Check your connection and try again.';
    return failure.refusal.reason.replace(/^[A-Za-z]+: /, '');
  }
  return String(failure);
}

/** Whether this page was opened by Sign out (features/me/SignOut.tsx), so it says why it is showing. */
const justSignedOut = (): boolean => new URLSearchParams(location.hash.split('?')[1] ?? '').has('signed-out');

/** A provider a person may sign in with: one button each. */
interface Offered { id: string; name: string; provider: string | null }

/** What a provider sign-in that did not finish is called, in Lys words. */
const PROVIDER_REFUSALS: Record<string, string> = {
  SignInCancelled: 'Sign-in was cancelled or permission was not given. Try again.',
  SecondFactorUnsupported: 'That account asks for a second factor, which Lys sign-in does not take yet.',
  SignInThrottled: 'Too many sign-ins failed from here. Wait a little, then try again.',
};

/** The refusal a provider sign-in came back with, named in the address. */
export function refusedInAddress(who = 'your administrator'): string {
  const params = new URLSearchParams(location.hash.split('?')[1] ?? '');
  const name = params.get('refused');
  if (!name) return '';
  const email = params.get('email');
  const account = 'This Google account' + (email ? ' (' + email + ')' : '');
  if (name === 'NotAdmittedYet') return account + " hasn't been added to Lys yet. Ask " + who + ' to add it.';
  if (name === 'AccountDisabled') return account + ' has been disabled in Lys. Ask ' + who + ' to enable it.';
  if (name === 'ProviderEmailMismatch') {
    const provider = params.get('provider') ?? 'The provider';
    const accountEmail = params.get('account_email') ?? 'an unknown address';
    const held = params.get('held_email') ?? 'an unknown address';
    return `${provider} returned the sign-in account ${accountEmail}, but Lys holds ${held} for this person. Ask ${who} to check your Account page.`;
  }
  // The browser-binding cookie is kept only at the address the sign-in began on. The service sends a refused
  // return to this page, at Lys's own address, so trying again here is the act that works.
  if (name === 'SignInStateUnknown') {
    return 'That sign-in could not be finished. It began at a different address from this one (' + location.host
      + '), or it expired or was already used. Sign in again on this page.';
  }
  return PROVIDER_REFUSALS[name] ?? 'The sign-in did not finish. Try again.';
}

/**
 * The providers Lys offers, whether the list could be read at all, and Lys's own address when the service
 * names it (`origin`, asked of the server in CONSENSUS 05 item 4): a provider returns only there.
 */
interface ProviderList { offered: Offered[]; unread: boolean; origin: string | null }

function useProviders(): ProviderList {
  const [list, setList] = useState<ProviderList>({ offered: [], unread: false, origin: null });
  useEffect(() => {
    let live = true;
    request<{ providers: Offered[]; origin?: unknown }>('/sign-in/providers').then(
      (answer) => { if (live) setList({ offered: answer.providers, unread: false, origin: typeof answer.origin === 'string' && /^https?:\/\/[^/?#]+$/.test(answer.origin) ? answer.origin : null }); },
      () => { if (live) setList({ offered: [], unread: true, origin: null }); },
    );
    return () => { live = false; };
  }, []);
  return list;
}

/** A page opened at another address than Lys's own: a provider could not finish here, so say so before it is tried, and link to the same page there. */
function Elsewhere({ offered, origin }: { offered: Offered[]; origin: string }) {
  let host = origin;
  try { host = new URL(origin).host; } catch { /* An origin that is not a URL is shown as the service gave it. */ }
  const names = offered.map((entry) => entry.name).join(' or ');
  return <div className="sign-in-providers" aria-label="Sign in with a provider">
    <p className="note" role="status">Sign in with {names} works only at {host}. You opened Lys at {location.host}.</p>
    <a className="btn" href={origin + '/' + location.hash}>Open Lys at {host}</a>
  </div>;
}

/** The line under the heading names every way in, so a person invited to use Google reads Google before the password fields. */
function waysIn(offered: Offered[]): string {
  if (offered.length === 0) return 'With your email and password.';
  return 'With ' + offered.map((entry) => entry.name).join(' or ') + ', or with your email and password.';
}

function Providers({ list }: { list: ProviderList }) {
  const { offered } = list;
  const [problem, setProblem] = useState('');
  if (list.unread) {
    return <p className="note sign-in-providers" role="status">Lys could not list its other ways to sign in, such as Google. Your email and password still work; to use another way, open this page again.</p>;
  }
  if (offered.length === 0) return null;
  if (list.origin && list.origin !== location.origin) return <Elsewhere offered={offered} origin={list.origin} />;
  const next = continuation();
  const query = next ? '?continue=' + encodeURIComponent(next) : '';
  // A file link the person opened is kept for the provider's return to /me (fileReturn.tsx).
  const remember = (event: MouseEvent) => {
    try {
      rememberFileReturn();
      // A product's or an app's sign-in continues to its own address instead (continuation below).
      if (!next) rememberLanding();
    } catch (error) {
      event.preventDefault();
      setProblem('This browser could not keep the link you opened while you sign in: ' + String(error) + '. Sign in, then open the link again.');
    }
  };
  return <div className="sign-in-providers" aria-label="Sign in with a provider">
    {offered.map((entry) => <a key={entry.id} className="btn" onClick={remember} href={API + '/sign-in/providers/' + encodeURIComponent(entry.id) + query}>
      Sign in with {entry.name}
    </a>)}
    {problem ? <p role="alert" className="why-not">{problem}</p> : null}
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

/** One store a Lys that refused to start found short, as `GET /health` names it. */
interface ShortStore { store: string; held: string; found: string }

/**
 * What a Lys that refused to start says on `GET /health` (DIRECTORY-107 R2): the stores it found short of their
 * receipt, what each holds and what was found, the one command that starts it at what was found, and the computer
 * to run it on. Null while Lys starts as it should, or when its health cannot be read.
 */
export interface RefusedStart { words: string; stores: ShortStore[]; command: string; computer: string }

/** The refused start a `/health` refusal names, or null for any other answer. */
export function refusedStart(failure: unknown): RefusedStart | null {
  if (!(failure instanceof Refused) || failure.refusal.refusal !== 'refused_to_start') return null;
  const answer = (failure.answer ?? {}) as { stores?: unknown; command?: unknown; computer?: unknown };
  const stores = Array.isArray(answer.stores)
    ? answer.stores.filter((entry): entry is ShortStore => typeof entry?.store === 'string' && typeof entry?.held === 'string' && typeof entry?.found === 'string')
    : [];
  return {
    words: failure.refusal.reason.replace(/^refused_to_start: /, ''),
    stores,
    command: typeof answer.command === 'string' ? answer.command : '',
    computer: typeof answer.computer === 'string' ? answer.computer : 'the computer Lys runs on',
  };
}

function useRefusedStart(): RefusedStart | null {
  const [refused, setRefused] = useState<RefusedStart | null>(null);
  useEffect(() => {
    let live = true;
    request<unknown>('/health').then(
      () => { if (live) setRefused(null); },
      (failure) => { if (live) setRefused(refusedStart(failure)); },
    );
    return () => { live = false; };
  }, []);
  return refused;
}

/** In place of the form while Lys refuses to start: what it lost, and the one way past it. */
export function RefusedToStart({ refused }: { refused: RefusedStart }) {
  return <div className="page sign-in" aria-label="Lys did not start">
    <LysMark />
    <h1 tabIndex={-1}>Lys did not start</h1>
    <p role="alert" className="why-not">{refused.words}</p>
    <ul className="note">
      {refused.stores.map((entry) => <li key={entry.store}>{entry.store}: its receipt holds {entry.held}; Lys found {entry.found}.</li>)}
    </ul>
    <p className="note">Nobody can sign in until this is settled. If what was found is all there is, start Lys at it by running this on {refused.computer}:</p>
    <pre><code>{refused.command}</code></pre>
  </div>;
}

/** The sign-in form, on its own or inside a screen that needs a session. */
export function SignIn({ signedIn = () => location.replace(SIGNED_IN) }: { signedIn?: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const help = useAdministratorContact();
  const providers = useProviders();
  const providerError = refusedInAddress(administratorWords(help.administrator));
  const [providerDismissed, setProviderDismissed] = useState(false);
  const stopped = useRefusedStart();
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
    try { forgetLanding(); } catch { /* Nothing kept to forget: the browser keeps no storage for this page. */ }
  }, []);

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
    setProviderDismissed(true);
    try {
      await request<SignedIn>('/sign-in', { email, password });
      // What someone else left open on this screen is never filled in for this person (fileReturn.tsx).
      const next = (await keepAcrossSignIn()) ? continuation() : null;
      if (next) location.assign(next);
      else signedIn();
    } catch (failure) {
      setError(reasonOf(failure));
    } finally {
      setBusy(false);
    }
  }

  if (stopped) return <RefusedToStart refused={stopped} />;
  return <div className="page sign-in">
    <LysMark />
    <h1 ref={heading} tabIndex={-1}>Sign in</h1>
    <p className="sub">{waysIn(providers.offered)}</p>
    {justSignedOut() ? <p role="status" className="note">You have signed out of Lys in this browser.</p> : null}
    <form className="recorded-form sign-in-form" aria-label="Sign in" onSubmit={submit} aria-busy={busy} noValidate>
      <div className="field">
        <label htmlFor="sign-in-email">Email</label>
        <input id="sign-in-email" name="email" type="email" autoComplete="username" required disabled={busy} />
      </div>
      <div className="field">
        <label htmlFor="sign-in-password">Password</label>
        <input id="sign-in-password" name="password" type="password" autoComplete="current-password" required disabled={busy} />
      </div>
      {error || (!providerDismissed && providerError) ? <p role="alert" className="why-not">{error || providerError}</p> : null}
      <Act symbol="enter" name={busy ? 'Signing in…' : 'Sign in'} word={busy ? 'Signing in…' : 'Sign in'} tone="primary" type="submit" disabled={busy} />
    </form>
    <p className="note found-line">New here? <a href="#/found">Found an organisation</a>.</p>
    <Providers list={providers} />
    <AccountHelpWords help={help} />
  </div>;
}
