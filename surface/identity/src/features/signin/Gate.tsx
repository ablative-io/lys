import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { API, api } from '../../api';
import type { Load, Refused } from '../../api';
import { useSignedOutScreen } from '../../shell/ShellContext';

export function Loading({ title = 'the directory' }: { title?: string } = {}) {
  return (
    <div className="page">
      <div className="dim">Reading {title}…</div>
    </div>
  );
}

/** What a caller with no session is shown: a small card in the middle of the screen with the Lys mark, the title and one button that signs in through the service's configured issuer. The shell beside it shows nothing that needs a session. The service keeps the session; the running build is under Configuration. */
export function SignIn() {
  useSignedOutScreen();
  return (
    <div className="page signed-out">
      <div className="card">
        <div className="mark"><span className="seal">L</span><span>Lys</span></div>
        <h1>Sign in</h1>
        <div><a className="btn primary" href={API + '/login'}>Sign in</a></div>
      </div>
    </div>
  );
}

/** A refusal shown with the service's own name and reason. */
export function RefusedPage({ refused, title }: { refused: Refused; title: string }) {
  if (refused.status === 401) return <SignIn />;
  const serviceFailure = refused.status === 0 || refused.status >= 500;
  return (
    <div className="page">
      <div className="eyebrow">{title}</div>
      <h1>{serviceFailure ? 'The service could not complete this request' : refused.refusal.refusal === 'NoPerson' ? 'Your account needs access' : 'Refused'}</h1>
      {serviceFailure ? <p>If you were making a change, keep its original request until its outcome is confirmed. Do not submit it again as a new change.</p> : null}
      {refused.refusal.refusal === 'NoPerson' ? <p>Your sign-in worked, but your account has not been connected to the directory yet. Ask your administrator to connect it on the Credentials tab of your page in People and agents.</p> : null}
      <div className="why-not">
        <b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span>
      </div>
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
      () => { if (live) location.replace('/#/'); },
      (error: Refused) => { if (live) setRefused(error); },
    );
    return () => { live = false; };
  }, []);
  if (refused) return <RefusedPage refused={refused} title="Sign in" />;
  return <Loading />;
}
