import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { api, Refused } from '../../api';
import type { Load } from '../../api';
import { useSignedOutScreen } from '../../shell/ShellContext';
import { RefusedToStart, SignIn as SignInForm, refusedStart } from '../sign-in/SignIn';
import { ErrorWords } from '../people/Words';
import { keepAcrossSignIn, takeFileReturn } from './fileReturn';

/** What a page shows while its read is pending: a sentence saying what is being read, never a page title spliced into one. */
export function Loading({ words = 'Opening Lys…' }: { words?: string } = {}) {
  return (
    <div className="page">
      <div className="dim">{words}</div>
    </div>
  );
}

/** Where a page that asked for sign-in goes once it is signed in: the same page, read again. */
export const signedInHere = { open: (): void => location.reload() };

/**
 * What a caller with no session meets on any page that needed one: Lys's one sign-in form, in place, at the
 * address they opened. Once signed in, that page opens. The shell beside it shows nothing that needs a session.
 */
export function SignIn() {
  useSignedOutScreen();
  return <SignInForm signedIn={() => signedInHere.open()} />;
}

/** The heading a refusal page carries: what happened, in words, never the refusal's code. */
function refusedHeading(refused: Refused): string {
  if (refused.status >= 500) return 'Lys could not complete this request';
  if (refused.refusal.refusal === 'NoPerson') return 'Your account needs access';
  if (refused.status === 403) return 'You do not have access to this';
  if (refused.status === 404) return 'This could not be found';
  return 'Lys could not open this';
}

/**
 * Lys did not answer at all. The page's own address names where; the likely cause is the computer Lys runs on,
 * which for a person with one computer is their own, so "ask the administrator" would send them to themselves.
 * A failed read reads again by itself once Lys answers (api.ts `useLoad`), so nothing here asks to press anything.
 */
function NotAnswering({ refused }: { refused: Refused }) {
  return <>
    <h1>Lys at {location.host} is not answering</h1>
    <p>The computer Lys runs on may be off, asleep or off the network: check that it is switched on and awake. When it answers, this page reads again by itself.</p>
    <details className="note"><summary>Error details</summary><p className="refusal-name">{refused.refusal.refusal}: {refused.refusal.reason}</p></details>
  </>;
}

/**
 * A refusal shown in plain words, its code and the service's own reason kept in the Error details fold. A page
 * shows this for a read, so it never warns about resending a change.
 */
export function RefusedPage({ refused, title }: { refused: Refused; title: string }) {
  if (refused.status === 401) return <SignIn />;
  // Lys refused to start (DIRECTORY-107 R2): every page says what it lost and the one way past it, never a riddle.
  const stopped = refusedStart(refused);
  if (stopped) return <RefusedToStart refused={stopped} />;
  if (refused.status === 0) return <div className="page"><div className="eyebrow">{title}</div><NotAnswering refused={refused} /></div>;
  return (
    <div className="page">
      <div className="eyebrow">{title}</div>
      <h1>{refusedHeading(refused)}</h1>
      {refused.refusal.refusal === 'NoPerson' ? <p>Your sign-in worked, but your account has not been connected to the directory yet. Ask your administrator to connect it on the Credentials tab of your page in People and agents.</p> : null}
      <ErrorWords problem={refused} />
      <p><a className="btn" href="#/">Back to the start</a></p>
    </div>
  );
}

/**
 * Render `ok` once the read has answered, the refusal if it was refused. `title` names the page above a refusal;
 * `reading` is the sentence shown while it reads, where "Reading {title}…" would not be one.
 */
export function Gate<T>({ load, title, reading, ok, renderError }: { load: Load<T>; title: string; reading?: string; ok: (data: T) => ReactNode; renderError?: (refused: Refused) => ReactNode }) {
  if (load.status === 'loading') return <Loading words={reading ?? 'Reading ' + title + '…'} />;
  if (load.status === 'refused') return <>{load.refused.status !== 401 && renderError ? renderError(load.refused) : <RefusedPage refused={load.refused} title={title} />}</>;
  return <>{ok(load.data)}</>;
}

/** The issuer sends the browser back here; the service finishes sign-in and sets its cookie. */
export function Callback() {
  const [refused, setRefused] = useState<Refused | null>(null);
  const exchange = useRef<ReturnType<typeof api.callback> | null>(null);
  useEffect(() => {
    // The code and state are single-use. StrictMode may replay this effect;
    // keep the same exchange, with only the current effect observing its result.
    let live = true;
    exchange.current ??= api.callback(location.search);
    exchange.current.then(
      async () => {
        await keepAcrossSignIn();
        if (live) {
          try { location.replace('/' + (takeFileReturn() ?? '#/')); }
          catch (error) { setRefused(new Refused(0, { refusal: 'FileReturnUnreadable', reason: String(error) })); }
        }
      },
      (error: Refused) => { if (live) setRefused(error); },
    );
    return () => { live = false; };
  }, []);
  if (refused) return <RefusedPage refused={refused} title="Sign in" />;
  return <Loading words="Finishing sign-in…" />;
}
