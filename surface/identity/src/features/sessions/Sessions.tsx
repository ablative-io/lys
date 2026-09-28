import { RuntimeSessions } from '../runtime/RuntimeSessions';
/** Live sign-ins use public session IDs; ending one requires an explicit confirmation. */
import { useRef, useState } from 'react';
import { useSearchParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { Gate, SignIn } from '../signin/Gate';

/** crates/lys-identity-server/src/sessions_api.rs: SessionView and SessionsView. */
export interface SessionView {
  id: string;
  login: { issuer: string; subject: string };
  started_at: number;
  ends_at: number;
  current: boolean;
}
interface SessionsView { person: string; sessions: SessionView[] }
const sessionPath = (person: string) => person ? '/directory/people/' + encodeURIComponent(person) + '/sessions' : '/sessions';

export function Sessions() {
  const [params, setParams] = useSearchParams();
  const person = params.get('person') ?? '';
  const people = useLoad(api.people, 'session-people');
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Account security</div><h1>Signed-in sessions</h1>
      <p className="sub">See where you’re signed in and end a session you no longer need.</p></div></div>
    {people.status === 'ok' && people.data.scope === 'directory' ? <details>
      <summary>Advanced: another person’s sessions</summary>
      <label className="field">Person<select value={person} onChange={(event) => setParams(event.target.value ? { person: event.target.value } : {})}>
        <option value="">My sessions</option>{people.data.people.map((entry) => <option key={entry.id} value={entry.id}>{entry.display_name}</option>)}
      </select></label>
    </details> : null}
    <SessionList key={person} person={person} />
    <RuntimeSessions />
  </div>;
}

export function SessionList({ person }: { person: string }) {
  const [revision, setRevision] = useState(0);
  const [confirm, setConfirm] = useState<SessionView | null>(null);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');
  const [ended, setEnded] = useState(false);
  const [blocked, setBlocked] = useState<{ id: string; revision: number } | null>(null);
  const sending = useRef(false);
  const path = sessionPath(person);
  const load = useLoad(async () => ({ ...await request<SessionsView>(path), readRevision: revision }), path + ':' + revision);
  const blockedId = blocked && !(load.status === 'ok' && load.data.readRevision > blocked.revision) ? blocked.id : '';
  async function finish(session: SessionView) {
    if (sending.current || blockedId === session.id) return;
    sending.current = true;
    setBusy(true);
    setProblem('');
    try {
      const answer = await request<{ ended: string }>(path + '/' + encodeURIComponent(session.id) + '/end', {});
      if (answer.ended !== session.id) throw new Error('The answer did not confirm this session was ended. Refresh the session list before trying again.');
      setConfirm(null);
      if (session.current) setEnded(true);
      else setRevision((value) => value + 1);
    } catch (error) {
      setBlocked({ id: session.id, revision });
      setProblem(error instanceof Error ? error.message : String(error));
    } finally { sending.current = false; setBusy(false); }
  }
  if (ended) return <SignIn />;
  return <>
    <button className="btn" disabled={busy || load.status === 'loading'} onClick={() => { setConfirm(null); setRevision((value) => value + 1); }}>Refresh sessions</button>
    {problem ? <div role="alert" className="why-not"><b>The session end could not be confirmed.</b><p>{problem}</p><p>Refresh to see whether it is still signed in. This request will not be repeated automatically.</p></div> : null}
    {confirm ? <section aria-label="Confirm session end" className="card">
      <h2>{confirm.current ? 'Sign out of this session?' : 'End this session?'}</h2>
      <p>{confirm.current ? 'You will need to sign in again to continue.' : 'That session will need to sign in again. Your current session stays open.'}</p>
      <button className="btn primary" disabled={busy || blockedId === confirm.id} onClick={() => { void finish(confirm); }}>{busy ? 'Ending session…' : 'Confirm end session'}</button>
      <button className="btn" disabled={busy} onClick={() => setConfirm(null)}>Cancel</button>
    </section> : null}
    <Gate load={load} title="Sessions" ok={(view) => view.sessions.length ? <table>
      <thead><tr><th>Session</th><th>Started</th><th>Expires</th><th>Action</th></tr></thead>
      <tbody>{view.sessions.map((session) => <tr key={session.id}>
        <td>{session.current ? 'This session' : 'Another signed-in session'}<details><summary>Sign-in details</summary>
          <p>Provider: {session.login.issuer}</p><p>Account identifier: {session.login.subject}</p><p>Session identifier: {session.id}</p>
        </details></td><td>{clock(session.started_at)}</td><td>{clock(session.ends_at)}</td>
        <td><button className="btn" disabled={busy || blockedId === session.id} onClick={() => setConfirm(session)}>{session.current ? 'Sign out' : 'End session'}</button></td>
      </tr>)}</tbody>
    </table> : <p className="note">No live sessions were returned for this person.</p>} />
  </>;
}
