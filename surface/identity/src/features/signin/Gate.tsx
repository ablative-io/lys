import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { API, api } from '../../api';
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
  const [authority, setAuthority] = useState('');
  useEffect(() => {
    let live = true;
    api.authority().then(
      (text) => live && setAuthority(text),
      () => live && setAuthority(''),
    );
    return () => {
      live = false;
    };
  }, []);
  return (
    <div className="page">
      <div className="eyebrow">Identity</div>
      <h1>Sign in</h1>
      <p className="sub">People sign in. Agents never sign in; they are registered.</p>
      <a className="btn primary" href={API + '/login'}>
        Sign in
      </a>
      {authority ? (
        <div className="empty-note" style={{ marginTop: 18, maxWidth: 640 }}>
          {authority}
        </div>
      ) : null}
    </div>
  );
}

/** A refusal shown with the service's own name and reason. */
export function RefusedPage({ refused, title }: { refused: Refused; title: string }) {
  if (refused.status === 401) return <SignIn />;
  return (
    <div className="page">
      <div className="eyebrow">{title}</div>
      <h1>Refused</h1>
      {refused.refusal.refusal === 'NoPerson' ? <p>A directory administrator must bind this sign-in to a person. <a href="#/directory/manage?action=login">Manage directory</a></p> : null}
      <div className="why-not">
        <b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span>
      </div>
    </div>
  );
}

/** Render `ok` once the read has answered, the refusal if it was refused. */
export function Gate<T>({ load, title, ok }: { load: Load<T>; title: string; ok: (data: T) => ReactNode }) {
  if (load.status === 'loading') return <Loading />;
  if (load.status === 'refused') return <RefusedPage refused={load.refused} title={title} />;
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
