import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { API, api, useLoad } from '../../api';
import type { Load, Refused } from '../../api';

export function Loading() {
  return (
    <div className="page">
      <div className="dim">Reading the directory…</div>
    </div>
  );
}

/** Sign-in, through the service's configured issuer. The service keeps the session. */
export function SignIn() {
  const authority = useLoad(api.authority, 'sign-in-authority');
  return (
    <div className="page">
      <div className="eyebrow">Identity</div>
      <h1>Sign in</h1>
      <p className="sub">Use your account to open the directory and manage your access.</p>
      <a className="btn primary" href={API + '/login'}>
        Sign in
      </a>
      <details style={{ marginTop: 18, maxWidth: 640 }}>
        <summary>Advanced: how access is managed</summary>
        <p>People sign in. Agents are registered by a person responsible for them.</p>
        {authority.status === 'loading' ? <p>Reading access details…</p>
          : authority.status === 'ok' ? <><p>{authority.data.authority}</p><p className="dim">Build <code>{authority.data.build}</code></p></>
          : <p role="status">Access details are unavailable: {authority.refused.refusal.refusal} — {authority.refused.refusal.reason}</p>}
      </details>
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
      {refused.refusal.refusal === 'NoPerson' ? <p>Your sign-in worked, but your account has not been connected to the directory yet. Ask your administrator to connect it. <a href="#/directory/manage?action=login">Administrator controls</a></p> : null}
      <div className="why-not">
        <b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span>
      </div>
    </div>
  );
}

/** Render `ok` once the read has answered, the refusal if it was refused. */
export function Gate<T>({ load, title, ok, renderError }: { load: Load<T>; title: string; ok: (data: T) => ReactNode; renderError?: (refused: Refused) => ReactNode }) {
  if (load.status === 'loading') return <Loading />;
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
      () => { if (live) location.replace('/#/me'); },
      (error: Refused) => { if (live) setRefused(error); },
    );
    return () => { live = false; };
  }, []);
  if (refused) return <RefusedPage refused={refused} title="Sign in" />;
  return <Loading />;
}
