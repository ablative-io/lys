/** The Sessions screen: every running agent session the caller may see, as its runner says, each opened to its live terminal with a line to type into, common keys and Stop. A session its runner saw end is not listed; nothing is inferred from a clock. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import type { RuntimeSession } from './RuntimeSessions';
import { Terminal } from './Terminal';
import './terminal.css';

/** A session whose runner did not answer when it was asked, named with the refusal. */
export interface Unanswered { session: string; machine: string; refusal: string; reason: string }

export function RunningSessions() {
  const { session } = useParams();
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const answer = await request<{ sessions: RuntimeSession[]; unanswered: Unanswered[] }>('/runtime/live');
    if (!Array.isArray(answer.sessions)) throw new Error('The service did not answer a session list.');
    if (!Array.isArray(answer.unanswered)) throw new Error('The service did not say which runners did not answer.');
    if (answer.sessions.some((entry) => entry.shown === 'stopped')) throw new Error('The service listed a stopped session as running.');
    return answer;
  }, 'runtime-live:' + revision);
  const open = load.status === 'ok' ? load.data.sessions.find((entry) => entry.session === session) : undefined;
  return <div className="page runner-sessions">
    <div className="eyebrow">Agents</div>
    <h1>Running sessions</h1>
    <p className="sub">Every agent session a runner holds that you may see. Open one to watch it, type to it or stop it.</p>
    <button type="button" className="btn" onClick={() => setRevision((value) => value + 1)}>Ask the runners again</button>
    <Gate load={load} title="Running sessions" ok={({ sessions, unanswered }) => <>
      {unanswered.length ? <div className="why-not" role="alert"><h3>Runners that did not answer</h3><ul>{unanswered.map((entry) => <li key={entry.session}><span className="mono">{entry.session}</span> on {entry.machine}: {entry.refusal}: {entry.reason}</li>)}</ul></div> : null}
      {sessions.length ? <table className="runner-list"><thead><tr><th>Agent</th><th>Machine</th><th>State</th><th>Since</th><th>Terminal</th></tr></thead><tbody>
      {sessions.map((entry) => <tr key={entry.session} aria-current={entry.session === session ? 'true' : undefined}>
        <td>{entry.agent ? <a href={'#/file/' + entry.agent}>{entry.agent}</a> : 'No identity attached'}<p className="mono">{entry.session}</p></td>
        <td>{entry.machine_name ?? entry.machine}</td>
        <td>{unanswered.some((silent) => silent.session === entry.session) ? 'Its runner did not answer; last reported ' + entry.last_reported : entry.shown === 'running' ? 'Running' : 'Starting, not yet confirmed'}<p className="note">{entry.what}</p></td>
        <td>{clock(entry.first_report_at)}</td>
        <td><a className="btn" href={'#/runtime/' + encodeURIComponent(entry.session)}>Open</a></td>
      </tr>)}
    </tbody></table> : <p>No running session was returned.</p>}
    </>} />
    {session ? <Terminal key={session} session={session} agent={open?.agent ?? null} /> : null}
  </div>;
}
