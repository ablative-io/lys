import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { api, Refused } from '../../api';
import type { Load } from '../../api';
import { useSignedOutScreen } from '../../shell/ShellContext';
import { SignIn as SignInForm } from '../sign-in/SignIn';
import { ErrorWords } from '../people/Words';
import { takeFileReturn } from './fileReturn';

export function Loading({ title = 'the directory' }: { title?: string } = {}) {
  return (
    <div className="page">
      <div className="dim">Reading {title}…</div>
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
  if (refused.status === 0 || refused.status >= 500) return 'The service could not complete this request';
  if (refused.refusal.refusal === 'NoPerson') return 'Your account needs access';
  if (refused.status === 403) return 'You do not have access to this';
  if (refused.status === 404) return 'This could not be found';
  return 'Lys could not open this';
}

/** A refusal shown in plain words, its code and the service's own reason kept in the Error details fold. */
export function RefusedPage({ refused, title }: { refused: Refused; title: string }) {
  if (refused.status === 401) return <SignIn />;
  const serviceFailure = refused.status === 0 || refused.status >= 500;
  return (
    <div className="page">
      <div className="eyebrow">{title}</div>
      <h1>{refusedHeading(refused)}</h1>
      {serviceFailure ? <p>If you were making a change, keep its original request until its outcome is confirmed. Do not submit it again as a new change.</p> : null}
      {refused.refusal.refusal === 'NoPerson' ? <p>Your sign-in worked, but your account has not been connected to the directory yet. Ask your administrator to connect it on the Credentials tab of your page in People and agents.</p> : null}
      <ErrorWords problem={refused} />
      <p><a className="btn" href="#/">Back to the start</a></p>
    </div>
  );
}

/** Render `ok` once the read has answered, the refusal if it was refused. */
export function Gate<T>({ load, title, ok, renderError }: { load: Load<T>; title: string; ok: (data: T) => ReactNode; renderError?: (refused: Refused) => ReactNode }) {
  if (load.status === 'loading') return <Loading title={title} />;
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
      () => {
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
  return <Loading />;
}
