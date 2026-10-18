/** Live sign-ins use public session IDs; ending one requires an explicit confirmation. */
import { useRef, useState } from 'react';
import { request, useLoad } from '../../api';
import { clock } from '../file/time';
import { Gate, SignIn } from '../signin/Gate';
import { ReadFailure, failureWords } from '../signin/words';
import { Act } from '../../shell/Act';
import { TableScroll } from '../../shell/Listing';

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
const providerName = (issuer: string): string => { try { return new URL(issuer).host; } catch { return 'Provider address not recognised'; } };

/** Said on a person's own page when this browser is the only place they are signed in. */
const ELSEWHERE = 'You are not signed in anywhere else.';

export function SessionList({ person }: { person: string }) {
  const [revision, setRevision] = useState(0);
  const [confirm, setConfirm] = useState<SessionView | null>(null);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');
  const [ended, setEnded] = useState(false);
  const [confirmedEnds, setConfirmedEnds] = useState<string[]>([]);
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
      if (answer.ended !== session.id) throw new Error('Lys answered, but its answer named a different session.');
      setConfirm(null);
      if (session.current) setEnded(true);
      else setConfirmedEnds((held) => [...held, answer.ended]);
    } catch (error) {
      setConfirm(null);
      setBlocked({ id: session.id, revision });
      setProblem(failureWords(error, 'Keep this page open until the session list has answered.'));
      setRevision((value) => value + 1);
    } finally { sending.current = false; setBusy(false); }
  }
  if (ended) return <SignIn />;
  return <>
    {problem ? <div role="alert" className="why-not"><b>The end of that session could not be confirmed.</b><p>Lys has read your sessions again below. If that session is still listed, it can still be used: choose End session to try again. Nothing is sent again on its own.</p><p className="sec">Lys said: {problem}</p></div> : null}
    {confirm ? <section aria-label="Confirm session end" className="card">
      <h2>{confirm.current ? 'Sign out of this session?' : 'End this session?'}</h2>
      <p>{confirm.current ? 'You will need to sign in again to continue.' : 'That session will need to sign in again. Your current session stays open.'}</p>
      <Act symbol={confirm.current ? 'leave' : 'stop'} name={busy ? 'Ending session…' : 'Confirm end session'} word={busy ? 'Ending…' : 'End'} tone="danger" disabled={busy || blockedId === confirm.id} onClick={() => { void finish(confirm); }} />
      <Act symbol="close" name="Cancel" word="Cancel" disabled={busy} onClick={() => setConfirm(null)} />
    </section> : null}
    <Gate load={load} title="the sign-in sessions" renderError={(error) => <ReadFailure error={error} subject="the sign-in sessions" administrator={Boolean(person)} />} ok={(view) => view.sessions.some((session) => !confirmedEnds.includes(session.id)) ? <><TableScroll label="Sign-in sessions"><table>
      <thead><tr><th>Session</th><th>Started</th><th>Expires</th><th>Action</th></tr></thead>
      <tbody>{view.sessions.filter((session) => !confirmedEnds.includes(session.id)).sort((a, b) => Number(b.current) - Number(a.current) || b.started_at - a.started_at).map((session) => <tr key={session.id}>
        <td>{session.current ? 'This session' : 'Another signed-in session'}<p className="note">Provider: <span title={session.login.issuer}>{providerName(session.login.issuer)}</span></p></td><td>{clock(session.started_at)}</td><td>{clock(session.ends_at)}</td>
        <td><Act symbol={session.current ? 'leave' : 'stop'} name={session.current ? 'Sign out' : 'End session'} word={session.current ? 'Sign out' : 'End'} disabled={busy || blockedId === session.id} onClick={() => setConfirm(session)} /></td>
      </tr>)}</tbody>
    </table></TableScroll>{!person && !view.sessions.some((session) => !session.current && !confirmedEnds.includes(session.id)) ? <p className="note">{ELSEWHERE}</p> : null}</> : <p className="note">{person ? 'This person is not signed in anywhere.' : ELSEWHERE}</p>} />
  </>;
}
